//! Daemon configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Path to store agent shards
    pub agent_store_path: PathBuf,

    /// Unix socket path for IPC
    pub ipc_socket_path: PathBuf,

    /// Whether to enable zkVM proving (can be disabled for testing)
    pub enable_zkvm_proving: bool,

    /// Disk mount point pattern (for detecting Sigil disks)
    pub disk_mount_pattern: String,

    /// Timeout for signing operations (seconds)
    pub signing_timeout_secs: u64,

    /// Whether to run in development mode
    pub dev_mode: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            agent_store_path: Self::default_agent_store_path(),
            ipc_socket_path: Self::default_ipc_path(),
            enable_zkvm_proving: true,
            disk_mount_pattern: Self::default_disk_pattern(),
            signing_timeout_secs: 60,
            dev_mode: false,
        }
    }
}

impl DaemonConfig {
    /// Platform-appropriate default IPC path
    #[cfg(unix)]
    fn default_ipc_path() -> PathBuf {
        // Use XDG_RUNTIME_DIR if available, fallback to /tmp
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("sigil.sock"))
            .unwrap_or_else(|| PathBuf::from("/tmp/sigil.sock"))
    }

    #[cfg(windows)]
    fn default_ipc_path() -> PathBuf {
        // Windows named pipes use special path syntax
        PathBuf::from(r"\\.\pipe\sigil")
    }

    #[cfg(unix)]
    fn default_disk_pattern() -> String {
        "/media/*/SIGIL*".to_string()
    }

    #[cfg(windows)]
    fn default_disk_pattern() -> String {
        // Windows: look for removable drives with SIGIL label
        r"*:\SIGIL*".to_string()
    }

    fn default_agent_store_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| {
                #[cfg(unix)]
                {
                    PathBuf::from("/var/lib")
                }
                #[cfg(windows)]
                {
                    PathBuf::from(r"C:\ProgramData")
                }
            })
            .join("sigil")
            .join("agent_store")
    }

    /// Load configuration from file
    pub fn load(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Create directories if they don't exist
    pub fn ensure_directories(&self) -> crate::Result<()> {
        std::fs::create_dir_all(&self.agent_store_path)?;

        // Only create parent directory for IPC path on Unix
        // Windows named pipes don't use filesystem paths
        #[cfg(unix)]
        {
            if let Some(parent) = self.ipc_socket_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        Ok(())
    }
}

/// Helper module for dirs crate functionality
mod dirs {
    use std::path::PathBuf;

    pub fn data_local_dir() -> Option<PathBuf> {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
    }
}
