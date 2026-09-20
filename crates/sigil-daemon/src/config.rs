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

    /// Unix permission bits applied to the IPC socket after bind.
    ///
    /// Defaults to `0o660` (owner + group, nothing for `other`), matching the
    /// `root:sigil` model the systemd unit installs. Ignored on Windows.
    /// Loosening this exposes the signer to every local user; see
    /// Constitution Principle VII.
    #[serde(default = "DaemonConfig::default_ipc_socket_mode")]
    pub ipc_socket_mode: u32,

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
            ipc_socket_mode: Self::default_ipc_socket_mode(),
            enable_zkvm_proving: true,
            disk_mount_pattern: Self::default_disk_pattern(),
            signing_timeout_secs: 60,
            dev_mode: false,
        }
    }
}

impl DaemonConfig {
    /// Default permission bits for the IPC socket.
    fn default_ipc_socket_mode() -> u32 {
        crate::ipc::connection::DEFAULT_SOCKET_MODE
    }

    /// Platform-appropriate default IPC path.
    ///
    /// Never falls back to `/tmp`. A world-writable parent directory lets any
    /// local user squat the socket path before the daemon starts, and leaves a
    /// briefly-permissive socket reachable between `bind` and `chmod`. The
    /// fallback is `/run/sigil`, which the daemon creates `0700` when absent;
    /// if `/run` is not writable the daemon fails loudly, which is the correct
    /// outcome. See Constitution Principle VII.
    #[cfg(unix)]
    fn default_ipc_path() -> PathBuf {
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("sigil.sock"))
            .unwrap_or_else(|| PathBuf::from("/run/sigil/sigil.sock"))
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// The IPC socket is an unauthenticated path to the signer. Parking it in
    /// a world-writable directory lets any local user squat the path before
    /// the daemon starts. See Constitution Principle VII.
    #[test]
    fn default_ipc_path_is_never_world_writable() {
        // Simulate a daemon started by systemd, which sets no XDG_RUNTIME_DIR.
        let previous = std::env::var_os("XDG_RUNTIME_DIR");
        std::env::remove_var("XDG_RUNTIME_DIR");

        let path = DaemonConfig::default_ipc_path();

        if let Some(previous) = previous {
            std::env::set_var("XDG_RUNTIME_DIR", previous);
        }

        assert!(
            !path.starts_with("/tmp"),
            "default IPC socket must not live under /tmp, got {}",
            path.display()
        );
        assert_eq!(path, PathBuf::from("/run/sigil/sigil.sock"));
    }

    #[test]
    fn default_socket_mode_excludes_other() {
        let mode = DaemonConfig::default().ipc_socket_mode;
        assert_eq!(
            mode & 0o007,
            0,
            "IPC socket default must grant nothing to `other`, got {mode:o}"
        );
    }

    #[test]
    fn socket_mode_is_defaulted_when_absent_from_config_file() {
        // Existing on-disk configs predate `ipc_socket_mode`; they must
        // deserialize into the hardened default rather than failing or
        // defaulting to 0.
        let json = r#"{
            "agent_store_path": "/var/lib/sigil/agent",
            "ipc_socket_path": "/run/sigil/sigil.sock",
            "enable_zkvm_proving": false,
            "disk_mount_pattern": "/media/*/SIGIL*",
            "signing_timeout_secs": 60,
            "dev_mode": false
        }"#;

        let config: DaemonConfig = serde_json::from_str(json).expect("legacy config must parse");
        assert_eq!(
            config.ipc_socket_mode,
            crate::ipc::connection::DEFAULT_SOCKET_MODE
        );
    }
}
