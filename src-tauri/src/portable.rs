//! Portable configuration export/import.
//!
//! A portable export is a single versioned JSON document containing the global
//! appearance settings, the active profile name, and every profile's portable
//! settings with its keymap SVG embedded. Bluetooth MAC addresses and original
//! keymap filesystem paths are deliberately excluded so the document is safe to
//! copy between machines.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{
    is_valid_theme, Appearance, Config, HudVisibility, OverlayAppearance, Profile, AUTO_DEVICE,
};

pub const PORTABLE_FORMAT: &str = "kb-hud-portable";
pub const PORTABLE_VERSION: u32 = 1;

/// Upper bound on the serialized import document size, guarding against
/// pathologically large or malicious files before parsing.
pub const MAX_IMPORT_BYTES: usize = 32 * 1024 * 1024;

/// Upper bound on a single embedded keymap SVG.
pub const MAX_KEYMAP_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum PortableError {
    Io(String),
    Serialize(String),
    UnsupportedVersion(u32),
    WrongFormat(String),
    InvalidTheme(String),
    EmptyProfileName,
    DuplicateProfile(String),
    MissingActiveProfile(String),
    InvalidValue(String),
    InvalidKeymap(String),
    KeymapRead { profile: String, path: String },
    KeymapTooLarge(String),
    DocumentTooLarge,
}

impl std::fmt::Display for PortableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortableError::Io(msg) => write!(f, "io error: {msg}"),
            PortableError::Serialize(msg) => write!(f, "serialization error: {msg}"),
            PortableError::UnsupportedVersion(v) => {
                write!(f, "unsupported portable version {v}")
            }
            PortableError::WrongFormat(fmt) => {
                write!(f, "unrecognized portable format '{fmt}'")
            }
            PortableError::InvalidTheme(id) => write!(f, "unknown theme id '{id}'"),
            PortableError::EmptyProfileName => write!(f, "profile name must not be empty"),
            PortableError::DuplicateProfile(name) => {
                write!(f, "duplicate profile name '{name}'")
            }
            PortableError::MissingActiveProfile(name) => {
                write!(f, "active profile '{name}' not present in import")
            }
            PortableError::InvalidValue(msg) => write!(f, "invalid value: {msg}"),
            PortableError::InvalidKeymap(msg) => write!(f, "invalid keymap: {msg}"),
            PortableError::KeymapRead { profile, path } => {
                write!(f, "profile '{profile}': could not read keymap '{path}'")
            }
            PortableError::KeymapTooLarge(profile) => {
                write!(f, "profile '{profile}': embedded keymap exceeds size limit")
            }
            PortableError::DocumentTooLarge => {
                write!(f, "import document exceeds size limit")
            }
        }
    }
}

/// An embedded keymap SVG within a portable export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedKeymap {
    /// Display metadata only; the backend never uses this to construct paths.
    pub filename: String,
    pub content: String,
}

/// Portable form of a profile: excludes `deviceMac` and `svgPath`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableProfile {
    pub name: String,
    pub scale: f64,
    pub hud: HudVisibility,
    pub overlay_appearance: OverlayAppearance,
    /// `None` when the profile has no configured keymap.
    pub keymap: Option<EmbeddedKeymap>,
}

/// The versioned portable export document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableExport {
    pub format: String,
    pub version: u32,
    pub appearance: Appearance,
    pub active_profile: String,
    pub profiles: Vec<PortableProfile>,
}

/// Builds a portable export from the current configuration, embedding every
/// readable configured keymap SVG. Profiles without a configured keymap are
/// represented with a null keymap. If any configured (non-empty) keymap cannot
/// be read, the entire export fails rather than producing an incomplete backup.
pub fn build_export(config: &Config) -> Result<PortableExport, PortableError> {
    let mut profiles = Vec::with_capacity(config.profiles.len());
    for profile in &config.profiles {
        let keymap = if profile.svg_path.trim().is_empty() {
            None
        } else {
            let content =
                fs::read_to_string(&profile.svg_path).map_err(|_| PortableError::KeymapRead {
                    profile: profile.name.clone(),
                    path: profile.svg_path.clone(),
                })?;
            Some(EmbeddedKeymap {
                filename: file_name_of(&profile.svg_path),
                content,
            })
        };
        profiles.push(PortableProfile {
            name: profile.name.clone(),
            scale: profile.scale,
            hud: profile.hud.clone(),
            overlay_appearance: profile.overlay_appearance.clone(),
            keymap,
        });
    }
    Ok(PortableExport {
        format: PORTABLE_FORMAT.to_string(),
        version: PORTABLE_VERSION,
        appearance: config.appearance.clone(),
        active_profile: config.active_profile.clone(),
        profiles,
    })
}

fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "keymap.svg".to_string())
}

/// Serializes an export and writes it atomically to `path`. The document is
/// written to a sibling temporary file first and renamed into place, so a
/// failure never leaves a partially replaced destination.
pub fn write_export(path: &Path, export: &PortableExport) -> Result<(), PortableError> {
    let raw = serde_json::to_string_pretty(export)
        .map_err(|e| PortableError::Serialize(e.to_string()))?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, &raw).map_err(|e| PortableError::Io(e.to_string()))?;
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        PortableError::Io(e.to_string())
    })?;
    Ok(())
}

/// Validates the portable document's structural fields (format, version, theme
/// ids, profile names, active profile reference, and numeric ranges). Used by
/// both the inspection preview and the confirmed commit path.
pub fn validate_export(export: &PortableExport) -> Result<(), PortableError> {
    if export.format != PORTABLE_FORMAT {
        return Err(PortableError::WrongFormat(export.format.clone()));
    }
    if export.version != PORTABLE_VERSION {
        return Err(PortableError::UnsupportedVersion(export.version));
    }
    if !is_valid_theme(&export.appearance.light_theme) {
        return Err(PortableError::InvalidTheme(
            export.appearance.light_theme.clone(),
        ));
    }
    if !is_valid_theme(&export.appearance.dark_theme) {
        return Err(PortableError::InvalidTheme(
            export.appearance.dark_theme.clone(),
        ));
    }
    if export.profiles.is_empty() {
        return Err(PortableError::InvalidValue(
            "import must contain at least one profile".to_string(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for profile in &export.profiles {
        if profile.name.trim().is_empty() {
            return Err(PortableError::EmptyProfileName);
        }
        if !seen.insert(profile.name.clone()) {
            return Err(PortableError::DuplicateProfile(profile.name.clone()));
        }
        validate_finite_scale(profile)?;
        profile
            .overlay_appearance
            .validate()
            .map_err(|e| PortableError::InvalidValue(e.to_string()))?;
        if let Some(keymap) = &profile.keymap {
            if keymap.content.len() > MAX_KEYMAP_BYTES {
                return Err(PortableError::KeymapTooLarge(profile.name.clone()));
            }
            validate_keymap_content(&profile.name, &keymap.content)?;
        }
    }
    if !seen.contains(&export.active_profile) {
        return Err(PortableError::MissingActiveProfile(
            export.active_profile.clone(),
        ));
    }
    Ok(())
}

fn validate_finite_scale(profile: &PortableProfile) -> Result<(), PortableError> {
    if !profile.scale.is_finite() || profile.scale <= 0.0 {
        return Err(PortableError::InvalidValue(format!(
            "profile '{}': scale must be a positive finite number",
            profile.name
        )));
    }
    Ok(())
}

/// Parses a raw import document with a bounded size check.
pub fn parse_import(raw: &str) -> Result<PortableExport, PortableError> {
    if raw.len() > MAX_IMPORT_BYTES {
        return Err(PortableError::DocumentTooLarge);
    }
    serde_json::from_str(raw).map_err(|e| PortableError::Serialize(e.to_string()))
}

/// Basic backend SVG compatibility check. Full geometric parsing happens when
/// the overlay loads the keymap, but this rejects empty or non-SVG payloads
/// before they are written to app-managed storage.
pub fn validate_keymap_content(profile: &str, content: &str) -> Result<(), PortableError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(PortableError::InvalidKeymap(format!(
            "profile '{profile}': embedded keymap is empty"
        )));
    }
    if !trimmed.to_lowercase().contains("<svg") {
        return Err(PortableError::InvalidKeymap(format!(
            "profile '{profile}': embedded keymap is not an SVG document"
        )));
    }
    Ok(())
}

/// Derives a safe, content-addressed filename for an imported keymap. The
/// imported filename metadata is deliberately ignored so path-like values can
/// never influence where the file is written.
pub fn managed_keymap_filename(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(16).map(|b| format!("{b:02x}")).collect();
    format!("{hex}.svg")
}

/// Per-profile keymap validation result surfaced in the import preview.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportKeymapResult {
    pub profile: String,
    /// "embedded", "none", or "invalid".
    pub status: String,
    pub valid: bool,
}

/// Replacement summary returned by the inspection command. It never mutates
/// current state; it only describes what a confirmed import would replace.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub profile_count: usize,
    pub active_profile: String,
    pub light_theme: String,
    pub dark_theme: String,
    pub keymaps: Vec<ImportKeymapResult>,
    /// Imported profiles always reset to automatic device discovery.
    pub device_reset_to_auto: bool,
}

/// Parses and fully validates an import document, then builds a replacement
/// summary. Returns actionable errors without mutating any current state.
pub fn inspect_import(raw: &str) -> Result<ImportSummary, PortableError> {
    let export = parse_import(raw)?;
    validate_export(&export)?;
    let keymaps = export
        .profiles
        .iter()
        .map(|profile| match &profile.keymap {
            None => ImportKeymapResult {
                profile: profile.name.clone(),
                status: "none".to_string(),
                valid: true,
            },
            Some(_) => ImportKeymapResult {
                profile: profile.name.clone(),
                status: "embedded".to_string(),
                valid: true,
            },
        })
        .collect();
    Ok(ImportSummary {
        profile_count: export.profiles.len(),
        active_profile: export.active_profile.clone(),
        light_theme: export.appearance.light_theme.clone(),
        dark_theme: export.appearance.dark_theme.clone(),
        keymaps,
        device_reset_to_auto: true,
    })
}

/// Writes every embedded keymap into `keymap_dir` under a content-addressed
/// name and returns a map of profile name -> managed path. Imported filename
/// metadata is ignored for path construction.
pub fn stage_imported_keymaps(
    export: &PortableExport,
    keymap_dir: &Path,
) -> Result<HashMap<String, String>, PortableError> {
    fs::create_dir_all(keymap_dir).map_err(|e| PortableError::Io(e.to_string()))?;
    let mut paths = HashMap::new();
    for profile in &export.profiles {
        if let Some(keymap) = &profile.keymap {
            validate_keymap_content(&profile.name, &keymap.content)?;
            let filename = managed_keymap_filename(&keymap.content);
            let dest = keymap_dir.join(&filename);
            fs::write(&dest, &keymap.content).map_err(|e| PortableError::Io(e.to_string()))?;
            paths.insert(profile.name.clone(), dest.to_string_lossy().to_string());
        }
    }
    Ok(paths)
}

/// Builds the replacement configuration from a validated export. Every imported
/// profile receives `deviceMac: "auto"` and a managed keymap path when present.
pub fn build_imported_config(
    export: &PortableExport,
    keymap_paths: &HashMap<String, String>,
) -> Config {
    let profiles = export
        .profiles
        .iter()
        .map(|p| Profile {
            name: p.name.clone(),
            svg_path: keymap_paths.get(&p.name).cloned().unwrap_or_default(),
            device_mac: AUTO_DEVICE.to_string(),
            scale: p.scale,
            hud: p.hud.clone(),
            overlay_appearance: p.overlay_appearance.clone(),
        })
        .collect();
    Config {
        active_profile: export.active_profile.clone(),
        appearance: export.appearance.clone(),
        profiles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ConfigStore, Profile, ProfilePatch, AUTO_DEVICE};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kb-hud-portable-{label}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_svg(dir: &Path, name: &str) -> String {
        let path = dir.join(name);
        fs::write(&path, "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>").unwrap();
        path.to_string_lossy().to_string()
    }

    fn sample_config(dir: &Path) -> Config {
        let svg_a = write_svg(dir, "a.svg");
        Config {
            active_profile: "alpha".to_string(),
            appearance: Appearance::default(),
            profiles: vec![
                Profile {
                    name: "alpha".to_string(),
                    svg_path: svg_a,
                    device_mac: "AA:BB:CC:DD:EE:FF".to_string(),
                    scale: 1.25,
                    hud: HudVisibility::default(),
                    overlay_appearance: OverlayAppearance::default(),
                },
                Profile {
                    name: "beta".to_string(),
                    svg_path: String::new(),
                    device_mac: AUTO_DEVICE.to_string(),
                    scale: 1.0,
                    hud: HudVisibility::default(),
                    overlay_appearance: OverlayAppearance::default(),
                },
            ],
        }
    }

    #[test]
    fn export_embeds_configured_keymap_and_nulls_unconfigured() {
        let dir = temp_dir("export");
        let config = sample_config(&dir);
        let export = build_export(&config).unwrap();
        assert_eq!(export.profiles.len(), 2);
        let alpha = &export.profiles[0];
        assert!(alpha.keymap.is_some());
        assert_eq!(alpha.keymap.as_ref().unwrap().filename, "a.svg");
        assert!(alpha.keymap.as_ref().unwrap().content.contains("<svg"));
        let beta = &export.profiles[1];
        assert!(beta.keymap.is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_contains_no_device_addresses_or_paths() {
        let dir = temp_dir("no-mac");
        let config = sample_config(&dir);
        let export = build_export(&config).unwrap();
        let raw = serde_json::to_string(&export).unwrap();
        assert!(!raw.contains("deviceMac"));
        assert!(!raw.contains("svgPath"));
        assert!(!raw.contains("AA:BB:CC:DD:EE:FF"));
        assert!(!raw.contains("/a.svg"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_fails_when_configured_keymap_cannot_be_read() {
        let dir = temp_dir("missing");
        let mut config = sample_config(&dir);
        config.profiles[0].svg_path = dir.join("does-not-exist.svg").to_string_lossy().to_string();
        let result = build_export(&config);
        assert!(matches!(result, Err(PortableError::KeymapRead { .. })));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_accepts_a_round_tripped_export() {
        let dir = temp_dir("valid");
        let config = sample_config(&dir);
        let export = build_export(&config).unwrap();
        assert!(validate_export(&export).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_rejects_wrong_format_and_version() {
        let dir = temp_dir("fmt");
        let config = sample_config(&dir);
        let mut export = build_export(&config).unwrap();
        export.format = "something-else".to_string();
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::WrongFormat(_))
        ));
        let mut export = build_export(&config).unwrap();
        export.version = 99;
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::UnsupportedVersion(99))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_rejects_unknown_theme_ids() {
        let dir = temp_dir("theme");
        let config = sample_config(&dir);
        let mut export = build_export(&config).unwrap();
        export.appearance.light_theme = "solarized".to_string();
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::InvalidTheme(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_rejects_duplicate_and_empty_profile_names() {
        let dir = temp_dir("names");
        let config = sample_config(&dir);
        let mut export = build_export(&config).unwrap();
        export.profiles[1].name = "alpha".to_string();
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::DuplicateProfile(_))
        ));
        let mut export = build_export(&config).unwrap();
        export.profiles[1].name = "   ".to_string();
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::EmptyProfileName)
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_rejects_missing_active_profile() {
        let dir = temp_dir("active");
        let config = sample_config(&dir);
        let mut export = build_export(&config).unwrap();
        export.active_profile = "ghost".to_string();
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::MissingActiveProfile(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_rejects_non_finite_scale() {
        let dir = temp_dir("scale");
        let config = sample_config(&dir);
        let mut export = build_export(&config).unwrap();
        export.profiles[0].scale = f64::NAN;
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::InvalidValue(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_export_does_not_partially_replace_destination() {
        let dir = temp_dir("write");
        let config = sample_config(&dir);
        let export = build_export(&config).unwrap();
        let dest = dir.join("backup.json");
        fs::write(&dest, "pre-existing content").unwrap();
        write_export(&dest, &export).unwrap();
        let written = fs::read_to_string(&dest).unwrap();
        assert!(written.contains("kb-hud-portable"));
        assert!(!dir.join("backup.json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_import_rejects_oversized_documents() {
        let huge = "x".repeat(MAX_IMPORT_BYTES + 1);
        assert!(matches!(
            parse_import(&huge),
            Err(PortableError::DocumentTooLarge)
        ));
    }

    #[test]
    fn config_store_round_trips_through_export_shape() {
        let dir = temp_dir("store");
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        store
            .update_profile(
                "default",
                ProfilePatch {
                    scale: Some(1.5),
                    ..Default::default()
                },
            )
            .unwrap();
        let export = build_export(store.config()).unwrap();
        assert!(validate_export(&export).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn managed_keymap_filename_is_content_derived_and_stable() {
        let a = managed_keymap_filename("<svg>same</svg>");
        let b = managed_keymap_filename("<svg>same</svg>");
        let c = managed_keymap_filename("<svg>different</svg>");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.ends_with(".svg"));
        assert!(!a.contains('/'));
    }

    #[test]
    fn validate_keymap_content_rejects_empty_and_non_svg() {
        assert!(matches!(
            validate_keymap_content("p", ""),
            Err(PortableError::InvalidKeymap(_))
        ));
        assert!(matches!(
            validate_keymap_content("p", "   "),
            Err(PortableError::InvalidKeymap(_))
        ));
        assert!(matches!(
            validate_keymap_content("p", "not an svg"),
            Err(PortableError::InvalidKeymap(_))
        ));
        assert!(validate_keymap_content("p", "<svg></svg>").is_ok());
    }

    #[test]
    fn inspect_import_summarizes_profiles_themes_and_keymaps() {
        let dir = temp_dir("inspect");
        let config = sample_config(&dir);
        let export = build_export(&config).unwrap();
        let raw = serde_json::to_string(&export).unwrap();
        let summary = inspect_import(&raw).unwrap();
        assert_eq!(summary.profile_count, 2);
        assert_eq!(summary.active_profile, "alpha");
        assert_eq!(summary.light_theme, "latte");
        assert_eq!(summary.dark_theme, "mocha");
        assert!(summary.device_reset_to_auto);
        assert_eq!(summary.keymaps.len(), 2);
        assert_eq!(summary.keymaps[0].status, "embedded");
        assert_eq!(summary.keymaps[1].status, "none");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_imported_config_assigns_auto_devices_and_managed_paths() {
        let dir = temp_dir("buildcfg");
        let config = sample_config(&dir);
        let export = build_export(&config).unwrap();
        let keymap_dir = dir.join("keymaps");
        let paths = stage_imported_keymaps(&export, &keymap_dir).unwrap();
        let imported = build_imported_config(&export, &paths);
        assert_eq!(imported.profiles.len(), 2);
        for profile in &imported.profiles {
            assert_eq!(profile.device_mac, AUTO_DEVICE);
        }
        let alpha = imported
            .profiles
            .iter()
            .find(|p| p.name == "alpha")
            .unwrap();
        assert!(alpha.svg_path.contains("keymaps"));
        assert!(alpha.svg_path.ends_with(".svg"));
        // The managed basename is a 32-hex content hash, not the original name.
        let managed_basename = Path::new(&alpha.svg_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(managed_basename.len(), 36); // 32 hex chars + ".svg"
        assert_ne!(managed_basename, "a.svg");
        let beta = imported.profiles.iter().find(|p| p.name == "beta").unwrap();
        assert_eq!(beta.svg_path, "");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_like_imported_filenames_are_ignored() {
        let dir = temp_dir("pathlike");
        let config = sample_config(&dir);
        let mut export = build_export(&config).unwrap();
        export.profiles[0].keymap.as_mut().unwrap().filename = "../../evil.svg".to_string();
        let keymap_dir = dir.join("keymaps");
        let paths = stage_imported_keymaps(&export, &keymap_dir).unwrap();
        let managed = paths.get("alpha").unwrap();
        assert!(managed.starts_with(&keymap_dir.to_string_lossy().to_string()));
        assert!(!managed.contains("evil"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_embedded_svg_blocks_import_before_config_change() {
        let dir = temp_dir("rollback");
        let store_dir = dir.join("store");
        fs::create_dir_all(&store_dir).unwrap();
        let mut store = ConfigStore::load_or_create(&store_dir).unwrap();
        let before = store.config().clone();

        let export = PortableExport {
            format: PORTABLE_FORMAT.to_string(),
            version: PORTABLE_VERSION,
            appearance: Appearance::default(),
            active_profile: "imp".to_string(),
            profiles: vec![PortableProfile {
                name: "imp".to_string(),
                scale: 1.0,
                hud: HudVisibility::default(),
                overlay_appearance: OverlayAppearance::default(),
                keymap: Some(EmbeddedKeymap {
                    filename: "bad.svg".to_string(),
                    content: "this is not an svg".to_string(),
                }),
            }],
        };
        assert!(matches!(
            validate_export(&export),
            Err(PortableError::InvalidKeymap(_))
        ));
        let keymap_dir = dir.join("keymaps");
        assert!(stage_imported_keymaps(&export, &keymap_dir).is_err());
        // Active configuration is untouched.
        assert_eq!(store.config(), &before);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn valid_replace_all_commit_replaces_configuration() {
        let dir = temp_dir("commit");
        let store_dir = dir.join("store");
        fs::create_dir_all(&store_dir).unwrap();
        let mut store = ConfigStore::load_or_create(&store_dir).unwrap();
        assert_eq!(store.config().profiles.len(), 1);

        let svg_content = "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        let export = PortableExport {
            format: PORTABLE_FORMAT.to_string(),
            version: PORTABLE_VERSION,
            appearance: Appearance::default(),
            active_profile: "imported-b".to_string(),
            profiles: vec![
                PortableProfile {
                    name: "imported-a".to_string(),
                    scale: 1.5,
                    hud: HudVisibility::default(),
                    overlay_appearance: OverlayAppearance::default(),
                    keymap: Some(EmbeddedKeymap {
                        filename: "a.svg".to_string(),
                        content: svg_content.to_string(),
                    }),
                },
                PortableProfile {
                    name: "imported-b".to_string(),
                    scale: 1.0,
                    hud: HudVisibility::default(),
                    overlay_appearance: OverlayAppearance::default(),
                    keymap: None,
                },
            ],
        };
        validate_export(&export).unwrap();
        let keymap_dir = store_dir.join("keymaps");
        let paths = stage_imported_keymaps(&export, &keymap_dir).unwrap();
        let new_config = build_imported_config(&export, &paths);
        store.replace_all(new_config).unwrap();

        let config = store.config();
        assert_eq!(config.profiles.len(), 2);
        assert_eq!(config.active_profile, "imported-b");
        for p in &config.profiles {
            assert_eq!(p.device_mac, AUTO_DEVICE);
        }
        let reloaded = ConfigStore::load_or_create(&store_dir).unwrap();
        assert_eq!(reloaded.config().profiles.len(), 2);
        assert_eq!(reloaded.config().active_profile, "imported-b");
        let _ = fs::remove_dir_all(&dir);
    }
}
