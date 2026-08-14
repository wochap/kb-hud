use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const AUTO_DEVICE: &str = "auto";
const PROFILES_FILE: &str = "profiles.json";

fn default_true() -> bool {
    true
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub active_profile: String,
    pub profiles: Vec<Profile>,
}

impl Config {
    pub fn first_launch_default() -> Self {
        Self {
            active_profile: "default".to_string(),
            profiles: vec![Profile {
                name: "default".to_string(),
                svg_path: String::new(),
                device_mac: AUTO_DEVICE.to_string(),
                scale: 1.0,
                hud: HudVisibility::default(),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Io(String),
    Parse(String),
    DuplicateName(String),
    NotFound(String),
    EmptyName,
    DeleteActiveProfile,
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
        let updated = profile.clone();
        if was_active {
            self.config.active_profile = updated.name.clone();
        }
        self.save()?;
        Ok(updated)
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
}
