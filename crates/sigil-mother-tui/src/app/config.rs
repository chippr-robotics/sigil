//! TUI configuration persistence
//!
//! Saves and loads user preferences such as selected device and mount settings.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sigil_mother::MountMethod;

/// Configuration file name
const CONFIG_FILE_NAME: &str = "config.json";

/// Configuration directory under ~/.config
const CONFIG_DIR_NAME: &str = "sigil-mother";

/// TUI configuration that persists across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Selected device path (e.g., "/dev/sda")
    #[serde(default)]
    pub selected_device: Option<String>,

    /// Custom mount point (if using traditional mount)
    #[serde(default = "default_mount_point")]
    pub mount_point: PathBuf,

    /// Whether to use udisksctl for mounting
    #[serde(default)]
    pub use_udisksctl: bool,

    /// Mount method preference
    #[serde(default)]
    pub mount_method: MountMethodConfig,
}

/// Mount method configuration (serializable version)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MountMethodConfig {
    /// Try udisksctl first, fall back to sudo mount
    #[default]
    Auto,
    /// Use udisksctl (doesn't require root)
    Udisksctl,
    /// Use traditional mount command (may require sudo)
    Traditional,
}

impl From<MountMethodConfig> for MountMethod {
    fn from(config: MountMethodConfig) -> Self {
        match config {
            MountMethodConfig::Auto => MountMethod::Auto,
            MountMethodConfig::Udisksctl => MountMethod::Udisksctl,
            MountMethodConfig::Traditional => MountMethod::Traditional,
        }
    }
}

impl From<MountMethod> for MountMethodConfig {
    fn from(method: MountMethod) -> Self {
        match method {
            MountMethod::Auto => MountMethodConfig::Auto,
            MountMethod::Udisksctl => MountMethodConfig::Udisksctl,
            MountMethod::Traditional => MountMethodConfig::Traditional,
        }
    }
}

fn default_mount_point() -> PathBuf {
    PathBuf::from("/mnt/floppy")
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            selected_device: None,
            mount_point: default_mount_point(),
            use_udisksctl: true, // Prefer udisksctl by default (doesn't require root)
            mount_method: MountMethodConfig::Auto,
        }
    }
}

impl TuiConfig {
    /// Get the configuration directory path
    pub fn config_dir() -> Option<PathBuf> {
        // Try XDG_CONFIG_HOME first, then fall back to ~/.config
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(xdg_config).join(CONFIG_DIR_NAME);
            return Some(path);
        }

        // Fall back to ~/.config
        dirs::config_dir().map(|p| p.join(CONFIG_DIR_NAME))
    }

    /// Get the full config file path
    pub fn config_file_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join(CONFIG_FILE_NAME))
    }

    /// Load configuration from disk
    ///
    /// Returns default configuration if file doesn't exist or can't be parsed.
    pub fn load() -> Self {
        let path = match Self::config_file_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse config file: {}", e);
                Self::default()
            }),
            Err(e) => {
                tracing::warn!("Failed to read config file: {}", e);
                Self::default()
            }
        }
    }

    /// Save configuration to disk
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_dir = Self::config_dir().ok_or(ConfigError::NoConfigDir)?;
        let config_file = config_dir.join(CONFIG_FILE_NAME);

        // Ensure config directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| ConfigError::Io(e.to_string()))?;
        }

        // Serialize and write
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;

        fs::write(&config_file, contents).map_err(|e| ConfigError::Io(e.to_string()))?;

        tracing::debug!("Saved config to {:?}", config_file);
        Ok(())
    }

    /// Update selected device and save
    pub fn set_selected_device(&mut self, device: Option<String>) -> Result<(), ConfigError> {
        self.selected_device = device;
        self.save()
    }

    /// Update mount point and save
    pub fn set_mount_point(&mut self, mount_point: PathBuf) -> Result<(), ConfigError> {
        self.mount_point = mount_point;
        self.save()
    }

    /// Update mount method and save
    pub fn set_mount_method(&mut self, method: MountMethodConfig) -> Result<(), ConfigError> {
        self.mount_method = method;
        self.save()
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Could not determine config directory")]
    NoConfigDir,

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialize(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TuiConfig::default();
        assert!(config.selected_device.is_none());
        assert_eq!(config.mount_point, PathBuf::from("/mnt/floppy"));
        assert!(config.use_udisksctl);
        assert_eq!(config.mount_method, MountMethodConfig::Auto);
    }

    #[test]
    fn test_config_serialization() {
        let config = TuiConfig {
            selected_device: Some("/dev/sda".to_string()),
            mount_point: PathBuf::from("/media/sigil"),
            use_udisksctl: false,
            mount_method: MountMethodConfig::Traditional,
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: TuiConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.selected_device, Some("/dev/sda".to_string()));
        assert_eq!(parsed.mount_point, PathBuf::from("/media/sigil"));
        assert!(!parsed.use_udisksctl);
        assert_eq!(parsed.mount_method, MountMethodConfig::Traditional);
    }

    #[test]
    fn test_mount_method_conversion() {
        assert_eq!(
            MountMethod::from(MountMethodConfig::Auto),
            MountMethod::Auto
        );
        assert_eq!(
            MountMethod::from(MountMethodConfig::Udisksctl),
            MountMethod::Udisksctl
        );
        assert_eq!(
            MountMethod::from(MountMethodConfig::Traditional),
            MountMethod::Traditional
        );
    }
}
