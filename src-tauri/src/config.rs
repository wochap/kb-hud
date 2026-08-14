use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const AUTO_DEVICE: &str = "auto";
const PROFILES_FILE: &str = "profiles.json";

pub const THEME_LATTE: &str = "latte";
pub const THEME_FRAPPE: &str = "frappe";
pub const THEME_MACCHIATO: &str = "macchiato";
pub const THEME_MOCHA: &str = "mocha";

pub fn is_valid_theme(id: &str) -> bool {
    matches!(
        id,
        THEME_LATTE | THEME_FRAPPE | THEME_MACCHIATO | THEME_MOCHA
    )
}

fn default_true() -> bool {
    true
}

fn default_light_theme() -> String {
    THEME_LATTE.to_string()
}

fn default_dark_theme() -> String {
    THEME_MOCHA.to_string()
}

fn default_label_opacity() -> f64 {
    0.92
}

fn default_idle_key_background_opacity() -> f64 {
    0.68
}

fn default_key_border_opacity() -> f64 {
    0.16
}

fn default_active_key_background_opacity() -> f64 {
    0.75
}

fn default_top_bar_pill_background_opacity() -> f64 {
    0.68
}

/// Returns `true` when the opacity is a finite value in the inclusive range 0..=1.
fn is_valid_opacity(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudVisibility {
    #[serde(default = "default_true")]
    pub layer: bool,
    #[serde(default = "default_true")]
    pub connection: bool,
    #[serde(default = "default_true")]
    pub gaps: bool,
    #[serde(default = "default_true")]
    pub firmware_drops: bool,
    #[serde(default = "default_true")]
    pub battery: bool,
    #[serde(default = "default_true")]
    pub transport: bool,
    #[serde(default = "default_true")]
    pub modifiers: bool,
}

impl Default for HudVisibility {
    fn default() -> Self {
        Self {
            layer: true,
            connection: true,
            gaps: true,
            firmware_drops: true,
            battery: true,
            transport: true,
            modifiers: true,
        }
    }
}

/// Global application appearance: which bundled palette to use for light and
/// dark system appearances. Stored at the configuration root, outside any
/// profile, because it is an application-wide preference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    #[serde(default = "default_light_theme")]
    pub light_theme: String,
    #[serde(default = "default_dark_theme")]
    pub dark_theme: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            light_theme: THEME_LATTE.to_string(),
            dark_theme: THEME_MOCHA.to_string(),
        }
    }
}

impl Appearance {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !is_valid_theme(&self.light_theme) {
            return Err(ConfigError::InvalidTheme(self.light_theme.clone()));
        }
        if !is_valid_theme(&self.dark_theme) {
            return Err(ConfigError::InvalidTheme(self.dark_theme.clone()));
        }
        Ok(())
    }
}

/// Per-profile overlay visibility and opacity preferences. Opacity values are
/// normalized to the inclusive range 0..=1. Defaults preserve the pre-change
/// overlay appearance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayAppearance {
    #[serde(default = "default_true")]
    pub show_idle_key_backgrounds: bool,
    #[serde(default = "default_label_opacity")]
    pub label_opacity: f64,
    #[serde(default = "default_idle_key_background_opacity")]
    pub idle_key_background_opacity: f64,
    #[serde(default = "default_key_border_opacity")]
    pub key_border_opacity: f64,
    #[serde(default = "default_active_key_background_opacity")]
    pub active_key_background_opacity: f64,
    #[serde(default = "default_top_bar_pill_background_opacity")]
    pub top_bar_pill_background_opacity: f64,
}

impl Default for OverlayAppearance {
    fn default() -> Self {
        Self {
            show_idle_key_backgrounds: true,
            label_opacity: default_label_opacity(),
            idle_key_background_opacity: default_idle_key_background_opacity(),
            key_border_opacity: default_key_border_opacity(),
            active_key_background_opacity: default_active_key_background_opacity(),
            top_bar_pill_background_opacity: default_top_bar_pill_background_opacity(),
        }
    }
}

impl OverlayAppearance {
    pub fn validate(&self) -> Result<(), ConfigError> {
        for (field, value) in [
            ("labelOpacity", self.label_opacity),
            ("idleKeyBackgroundOpacity", self.idle_key_background_opacity),
            ("keyBorderOpacity", self.key_border_opacity),
            (
                "activeKeyBackgroundOpacity",
                self.active_key_background_opacity,
            ),
            (
                "topBarPillBackgroundOpacity",
                self.top_bar_pill_background_opacity,
            ),
        ] {
            if !is_valid_opacity(value) {
                return Err(ConfigError::InvalidOpacity(field.to_string(), value));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub svg_path: String,
    /// Bluetooth MAC address, or `auto` to auto-detect the paired keyboard.
    pub device_mac: String,
    pub scale: f64,
    #[serde(default)]
    pub hud: HudVisibility,
    #[serde(default)]
    pub overlay_appearance: OverlayAppearance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub active_profile: String,
    #[serde(default)]
    pub appearance: Appearance,
    pub profiles: Vec<Profile>,
}

impl Config {
    pub fn first_launch_default() -> Self {
        Self {
            active_profile: "default".to_string(),
            appearance: Appearance::default(),
            profiles: vec![Profile {
                name: "default".to_string(),
                svg_path: String::new(),
                device_mac: AUTO_DEVICE.to_string(),
                scale: 1.0,
                hud: HudVisibility::default(),
                overlay_appearance: OverlayAppearance::default(),
            }],
        }
    }

    pub fn active(&self) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == self.active_profile)
    }

    fn find_mut(&mut self, name: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePatch {
    pub name: Option<String>,
    pub svg_path: Option<String>,
    pub device_mac: Option<String>,
    pub scale: Option<f64>,
    pub hud: Option<HudVisibility>,
    pub overlay_appearance: Option<OverlayAppearance>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearancePatch {
    pub light_theme: Option<String>,
    pub dark_theme: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    Io(String),
    Parse(String),
    DuplicateName(String),
    NotFound(String),
    EmptyName,
    DeleteActiveProfile,
    InvalidTheme(String),
    InvalidOpacity(String, f64),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(msg) => write!(f, "config io error: {msg}"),
            ConfigError::Parse(msg) => write!(f, "config parse error: {msg}"),
            ConfigError::DuplicateName(name) => write!(f, "profile '{name}' already exists"),
            ConfigError::NotFound(name) => write!(f, "profile '{name}' not found"),
            ConfigError::EmptyName => write!(f, "profile name must not be empty"),
            ConfigError::DeleteActiveProfile => {
                write!(
                    f,
                    "cannot delete the active profile; activate another first"
                )
            }
            ConfigError::InvalidTheme(id) => write!(f, "unknown theme id '{id}'"),
            ConfigError::InvalidOpacity(field, value) => {
                write!(f, "{field} must be a finite value in 0..=1, got {value}")
            }
        }
    }
}

pub struct ConfigStore {
    config: Config,
    path: PathBuf,
}

impl ConfigStore {
    /// Loads `profiles.json` from the given config directory, creating and
    /// persisting the first-launch default when absent.
    pub fn load_or_create(config_dir: &Path) -> Result<Self, ConfigError> {
        let path = config_dir.join(PROFILES_FILE);
        if path.exists() {
            let raw = fs::read_to_string(&path).map_err(|e| ConfigError::Io(e.to_string()))?;
            let config: Config =
                serde_json::from_str(&raw).map_err(|e| ConfigError::Parse(e.to_string()))?;
            Ok(Self { config, path })
        } else {
            let store = Self {
                config: Config::first_launch_default(),
                path,
            };
            store.save()?;
            Ok(store)
        }
    }

    fn save(&self) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
        }
        let raw = serde_json::to_string_pretty(&self.config)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        fs::write(&self.path, raw).map_err(|e| ConfigError::Io(e.to_string()))
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn active_profile(&self) -> Option<&Profile> {
        self.config.active()
    }

    fn validate_name(name: &str) -> Result<(), ConfigError> {
        if name.trim().is_empty() {
            return Err(ConfigError::EmptyName);
        }
        Ok(())
    }

    pub fn create_profile(&mut self, name: &str) -> Result<Profile, ConfigError> {
        Self::validate_name(name)?;
        if self.config.find_mut(name).is_some() {
            return Err(ConfigError::DuplicateName(name.to_string()));
        }
        let profile = Profile {
            name: name.to_string(),
            svg_path: String::new(),
            device_mac: AUTO_DEVICE.to_string(),
            scale: 1.0,
            hud: HudVisibility::default(),
            overlay_appearance: OverlayAppearance::default(),
        };
        self.config.profiles.push(profile.clone());
        self.save()?;
        Ok(profile)
    }

    pub fn rename_profile(&mut self, name: &str, new_name: &str) -> Result<(), ConfigError> {
        Self::validate_name(new_name)?;
        if name != new_name && self.config.find_mut(new_name).is_some() {
            return Err(ConfigError::DuplicateName(new_name.to_string()));
        }
        let profile = self
            .config
            .find_mut(name)
            .ok_or_else(|| ConfigError::NotFound(name.to_string()))?;
        profile.name = new_name.to_string();
        if self.config.active_profile == name {
            self.config.active_profile = new_name.to_string();
        }
        self.save()
    }

    pub fn delete_profile(&mut self, name: &str) -> Result<(), ConfigError> {
        if self.config.active_profile == name {
            return Err(ConfigError::DeleteActiveProfile);
        }
        let before = self.config.profiles.len();
        self.config.profiles.retain(|p| p.name != name);
        if self.config.profiles.len() == before {
            return Err(ConfigError::NotFound(name.to_string()));
        }
        self.save()
    }

    pub fn set_active(&mut self, name: &str) -> Result<(), ConfigError> {
        if self.config.find_mut(name).is_none() {
            return Err(ConfigError::NotFound(name.to_string()));
        }
        self.config.active_profile = name.to_string();
        self.save()
    }

    pub fn update_profile(
        &mut self,
        name: &str,
        patch: ProfilePatch,
    ) -> Result<Profile, ConfigError> {
        if let Some(new_name) = &patch.name {
            Self::validate_name(new_name)?;
            if new_name != name && self.config.find_mut(new_name).is_some() {
                return Err(ConfigError::DuplicateName(new_name.clone()));
            }
        }
        if let Some(overlay) = &patch.overlay_appearance {
            overlay.validate()?;
        }
        let was_active = self.config.active_profile == name;
        let profile = self
            .config
            .find_mut(name)
            .ok_or_else(|| ConfigError::NotFound(name.to_string()))?;
        if let Some(new_name) = patch.name {
            profile.name = new_name;
        }
        if let Some(svg_path) = patch.svg_path {
            profile.svg_path = svg_path;
        }
        if let Some(device_mac) = patch.device_mac {
            profile.device_mac = device_mac;
        }
        if let Some(scale) = patch.scale {
            profile.scale = scale;
        }
        if let Some(hud) = patch.hud {
            profile.hud = hud;
        }
        if let Some(overlay_appearance) = patch.overlay_appearance {
            profile.overlay_appearance = overlay_appearance;
        }
        let updated = profile.clone();
        if was_active {
            self.config.active_profile = updated.name.clone();
        }
        self.save()?;
        Ok(updated)
    }

    /// Returns the global appearance configuration.
    pub fn appearance(&self) -> &Appearance {
        &self.config.appearance
    }

    /// Returns the directory containing `profiles.json`.
    pub fn config_dir(&self) -> Option<&Path> {
        self.path.parent()
    }

    /// Atomically replaces the entire configuration with `new_config`. The
    /// replacement is serialized to a temporary file and renamed over the
    /// active configuration last, so a failure before the rename leaves the
    /// previous configuration untouched.
    pub fn replace_all(&mut self, new_config: Config) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
        }
        let raw = serde_json::to_string_pretty(&new_config)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, raw).map_err(|e| ConfigError::Io(e.to_string()))?;
        fs::rename(&temp, &self.path).map_err(|e| {
            let _ = fs::remove_file(&temp);
            ConfigError::Io(e.to_string())
        })?;
        self.config = new_config;
        Ok(())
    }

    /// Applies a partial appearance update, validates theme ids, persists, and
    /// returns the updated appearance.
    pub fn update_appearance(&mut self, patch: AppearancePatch) -> Result<Appearance, ConfigError> {
        let mut candidate = self.config.appearance.clone();
        if let Some(light) = patch.light_theme {
            candidate.light_theme = light;
        }
        if let Some(dark) = patch.dark_theme {
            candidate.dark_theme = dark;
        }
        candidate.validate()?;
        self.config.appearance = candidate.clone();
        self.save()?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("kb-hud-config-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn first_launch_creates_default_profile() {
        let dir = temp_config_dir();
        let store = ConfigStore::load_or_create(&dir).unwrap();
        let config = store.config();
        assert_eq!(config.active_profile, "default");
        assert_eq!(config.profiles.len(), 1);
        let profile = &config.profiles[0];
        assert_eq!(profile.device_mac, "auto");
        assert_eq!(profile.scale, 1.0);
        assert!(dir.join("profiles.json").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn round_trips_config() {
        let dir = temp_config_dir();
        {
            let mut store = ConfigStore::load_or_create(&dir).unwrap();
            store.create_profile("second").unwrap();
            store
                .update_profile(
                    "second",
                    ProfilePatch {
                        svg_path: Some("/tmp/corne.svg".to_string()),
                        device_mac: Some("AA:BB:CC:DD:EE:FF".to_string()),
                        scale: Some(1.5),
                        ..Default::default()
                    },
                )
                .unwrap();
            store.set_active("second").unwrap();
        }
        let store = ConfigStore::load_or_create(&dir).unwrap();
        let config = store.config();
        assert_eq!(config.active_profile, "second");
        assert_eq!(config.profiles.len(), 2);
        let second = config.active().unwrap();
        assert_eq!(second.svg_path, "/tmp/corne.svg");
        assert_eq!(second.device_mac, "AA:BB:CC:DD:EE:FF");
        assert_eq!(second.scale, 1.5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_duplicate_and_missing() {
        let dir = temp_config_dir();
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        assert_eq!(
            store.create_profile("default"),
            Err(ConfigError::DuplicateName("default".to_string()))
        );
        assert_eq!(
            store.set_active("nope"),
            Err(ConfigError::NotFound("nope".to_string()))
        );
        assert_eq!(store.create_profile("  "), Err(ConfigError::EmptyName));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_follows_active_marker() {
        let dir = temp_config_dir();
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        store.rename_profile("default", "main").unwrap();
        assert_eq!(store.config().active_profile, "main");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_refuses_active_profile() {
        let dir = temp_config_dir();
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        store.create_profile("spare").unwrap();
        assert_eq!(
            store.delete_profile("default"),
            Err(ConfigError::DeleteActiveProfile)
        );
        store.set_active("spare").unwrap();
        store.delete_profile("default").unwrap();
        assert_eq!(store.config().profiles.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn serializes_with_camel_case_fields() {
        let config = Config::first_launch_default();
        let raw = serde_json::to_string(&config).unwrap();
        assert!(raw.contains("\"activeProfile\""));
        assert!(raw.contains("\"svgPath\""));
        assert!(raw.contains("\"deviceMac\""));
    }

    #[test]
    fn profile_without_hud_deserializes_all_enabled() {
        let raw = r#"{
            "activeProfile": "default",
            "profiles": [{
                "name": "default",
                "svgPath": "/tmp/k.svg",
                "deviceMac": "auto",
                "scale": 1.0
            }]
        }"#;
        let config: Config = serde_json::from_str(raw).unwrap();
        let hud = &config.profiles[0].hud;
        assert!(hud.layer);
        assert!(hud.connection);
        assert!(hud.gaps);
        assert!(hud.firmware_drops);
        assert!(hud.battery);
        assert!(hud.transport);
        assert!(hud.modifiers);
    }

    #[test]
    fn patch_with_partial_hud_defaults_missing_fields_to_true() {
        let dir = temp_config_dir();
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        let hud: HudVisibility = serde_json::from_str(r#"{"battery": false}"#).unwrap();
        assert!(!hud.battery);
        assert!(hud.layer);
        assert!(hud.connection);
        assert!(hud.gaps);
        assert!(hud.firmware_drops);
        assert!(hud.transport);
        assert!(hud.modifiers);
        let updated = store
            .update_profile(
                "default",
                ProfilePatch {
                    hud: Some(hud),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(!updated.hud.battery);
        assert!(updated.hud.layer);
        let reloaded = ConfigStore::load_or_create(&dir).unwrap();
        assert!(!reloaded.active_profile().unwrap().hud.battery);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn first_launch_appearance_defaults_to_latte_and_mocha() {
        let dir = temp_config_dir();
        let store = ConfigStore::load_or_create(&dir).unwrap();
        let appearance = store.appearance();
        assert_eq!(appearance.light_theme, THEME_LATTE);
        assert_eq!(appearance.dark_theme, THEME_MOCHA);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn first_launch_overlay_appearance_matches_pre_change_look() {
        let dir = temp_config_dir();
        let store = ConfigStore::load_or_create(&dir).unwrap();
        let overlay = &store.active_profile().unwrap().overlay_appearance;
        assert!(overlay.show_idle_key_backgrounds);
        assert!((overlay.label_opacity - 0.92).abs() < f64::EPSILON);
        assert!((overlay.idle_key_background_opacity - 0.68).abs() < f64::EPSILON);
        assert!((overlay.key_border_opacity - 0.16).abs() < f64::EPSILON);
        assert!((overlay.active_key_background_opacity - 0.75).abs() < f64::EPSILON);
        assert!((overlay.top_bar_pill_background_opacity - 0.68).abs() < f64::EPSILON);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_config_without_appearance_defaults_to_latte_mocha() {
        let raw = r#"{
            "activeProfile": "default",
            "profiles": [{
                "name": "default",
                "svgPath": "",
                "deviceMac": "auto",
                "scale": 1.0
            }]
        }"#;
        let config: Config = serde_json::from_str(raw).unwrap();
        assert_eq!(config.appearance.light_theme, THEME_LATTE);
        assert_eq!(config.appearance.dark_theme, THEME_MOCHA);
    }

    #[test]
    fn legacy_profile_without_overlay_appearance_gets_defaults() {
        let raw = r#"{
            "activeProfile": "default",
            "profiles": [{
                "name": "default",
                "svgPath": "",
                "deviceMac": "auto",
                "scale": 1.0
            }]
        }"#;
        let config: Config = serde_json::from_str(raw).unwrap();
        let overlay = &config.profiles[0].overlay_appearance;
        assert_eq!(*overlay, OverlayAppearance::default());
    }

    #[test]
    fn round_trips_appearance_and_overlay_appearance() {
        let dir = temp_config_dir();
        {
            let mut store = ConfigStore::load_or_create(&dir).unwrap();
            store
                .update_appearance(AppearancePatch {
                    light_theme: Some(THEME_FRAPPE.to_string()),
                    dark_theme: Some(THEME_MACCHIATO.to_string()),
                })
                .unwrap();
            let mut overlay = OverlayAppearance::default();
            overlay.show_idle_key_backgrounds = false;
            overlay.label_opacity = 0.5;
            store
                .update_profile(
                    "default",
                    ProfilePatch {
                        overlay_appearance: Some(overlay),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        let store = ConfigStore::load_or_create(&dir).unwrap();
        assert_eq!(store.appearance().light_theme, THEME_FRAPPE);
        assert_eq!(store.appearance().dark_theme, THEME_MACCHIATO);
        let overlay = &store.active_profile().unwrap().overlay_appearance;
        assert!(!overlay.show_idle_key_backgrounds);
        assert!((overlay.label_opacity - 0.5).abs() < f64::EPSILON);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_invalid_theme_ids() {
        let dir = temp_config_dir();
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        let result = store.update_appearance(AppearancePatch {
            light_theme: Some("solarized".to_string()),
            dark_theme: None,
        });
        assert_eq!(
            result,
            Err(ConfigError::InvalidTheme("solarized".to_string()))
        );
        let result = store.update_appearance(AppearancePatch {
            light_theme: None,
            dark_theme: Some("dracula".to_string()),
        });
        assert_eq!(
            result,
            Err(ConfigError::InvalidTheme("dracula".to_string()))
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_out_of_range_opacity() {
        let dir = temp_config_dir();
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        let mut overlay = OverlayAppearance::default();
        overlay.label_opacity = 1.5;
        let result = store.update_profile(
            "default",
            ProfilePatch {
                overlay_appearance: Some(overlay),
                ..Default::default()
            },
        );
        assert!(matches!(result, Err(ConfigError::InvalidOpacity(_, _))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_negative_and_non_finite_opacity() {
        let dir = temp_config_dir();
        let mut store = ConfigStore::load_or_create(&dir).unwrap();
        for bad in [-0.1, f64::NAN, f64::INFINITY] {
            let mut overlay = OverlayAppearance::default();
            overlay.key_border_opacity = bad;
            let result = store.update_profile(
                "default",
                ProfilePatch {
                    overlay_appearance: Some(overlay),
                    ..Default::default()
                },
            );
            assert!(
                matches!(result, Err(ConfigError::InvalidOpacity(_, _))),
                "expected rejection for {bad}"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
