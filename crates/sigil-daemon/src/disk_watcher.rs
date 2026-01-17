//! Disk detection and monitoring via udev
//!
//! Watches for Sigil floppy disk insertion and removal.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

use sigil_core::{DiskFormat, DiskHeader, DISK_MAGIC};

use crate::error::{DaemonError, Result};

/// Event emitted when a disk is detected or removed
#[derive(Debug, Clone)]
pub enum DiskEvent {
    /// A Sigil disk was inserted
    Inserted {
        path: PathBuf,
        header: DiskHeader,
    },
    /// A disk was removed
    Removed {
        path: PathBuf,
    },
    /// Disk validation failed
    ValidationFailed {
        path: PathBuf,
        reason: String,
    },
}

/// Watches for Sigil disk insertion/removal
pub struct DiskWatcher {
    /// Currently detected disk (if any)
    current_disk: Arc<RwLock<Option<DetectedDisk>>>,

    /// Event broadcast channel
    event_tx: broadcast::Sender<DiskEvent>,

    /// Mount point pattern to watch
    mount_pattern: String,
}

/// A detected Sigil disk
#[derive(Debug, Clone)]
pub struct DetectedDisk {
    /// Path to the disk
    pub path: PathBuf,
    /// Disk header
    pub header: DiskHeader,
    /// Full disk data (loaded on demand)
    pub format: Option<DiskFormat>,
}

impl DiskWatcher {
    /// Create a new disk watcher
    pub fn new(mount_pattern: String) -> Self {
        let (event_tx, _) = broadcast::channel(16);

        Self {
            current_disk: Arc::new(RwLock::new(None)),
            event_tx,
            mount_pattern,
        }
    }

    /// Subscribe to disk events
    pub fn subscribe(&self) -> broadcast::Receiver<DiskEvent> {
        self.event_tx.subscribe()
    }

    /// Get the currently detected disk
    pub async fn current_disk(&self) -> Option<DetectedDisk> {
        self.current_disk.read().await.clone()
    }

    /// Check if a disk is currently inserted
    pub async fn has_disk(&self) -> bool {
        self.current_disk.read().await.is_some()
    }

    /// Start watching for disk events (blocking)
    pub async fn watch(&self) -> Result<()> {
        info!("Starting disk watcher with pattern: {}", self.mount_pattern);

        // Initial scan
        self.scan_for_disks().await?;

        // Watch for changes using udev
        #[cfg(target_os = "linux")]
        {
            use tokio_udev::{AsyncMonitorSocket, MonitorBuilder};

            let builder = MonitorBuilder::new()
                .map_err(|e| DaemonError::Udev(e.to_string()))?
                .match_subsystem("block")
                .map_err(|e| DaemonError::Udev(e.to_string()))?;

            let monitor = builder.listen().map_err(|e| DaemonError::Udev(e.to_string()))?;

            let mut socket = AsyncMonitorSocket::new(monitor)
                .map_err(|e| DaemonError::Udev(e.to_string()))?;

            loop {
                use futures_util::StreamExt;

                match socket.next().await {
                    Some(Ok(event)) => {
                        let action = event.action().map(|a| a.to_string_lossy().to_string());
                        debug!("Udev event: {:?} for {:?}", action, event.devpath());

                        match action.as_deref() {
                            Some("add") | Some("change") => {
                                // Small delay to allow mount
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                self.scan_for_disks().await?;
                            }
                            Some("remove") => {
                                self.handle_disk_removal().await?;
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        error!("Udev error: {}", e);
                    }
                    None => break,
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback: poll for changes on non-Linux systems
            warn!("Udev not available, using polling fallback");
            loop {
                self.scan_for_disks().await?;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }

        Ok(())
    }

    /// Scan for Sigil disks
    async fn scan_for_disks(&self) -> Result<()> {
        let paths = glob::glob(&self.mount_pattern)
            .map_err(|e| DaemonError::Config(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        if paths.is_empty() {
            // Check if disk was removed
            if self.current_disk.read().await.is_some() {
                self.handle_disk_removal().await?;
            }
            return Ok(());
        }

        // Look for actual Sigil disk file
        for base_path in paths {
            let disk_file = base_path.join("sigil.disk");
            if disk_file.exists() {
                match self.try_load_disk(&disk_file).await {
                    Ok(disk) => {
                        let header = disk.header.clone();
                        let path = disk_file.clone();

                        let mut current = self.current_disk.write().await;
                        *current = Some(disk);

                        let _ = self.event_tx.send(DiskEvent::Inserted { path, header });
                        info!("Sigil disk detected");
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to load disk at {:?}: {}", disk_file, e);
                        let _ = self.event_tx.send(DiskEvent::ValidationFailed {
                            path: disk_file,
                            reason: e.to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Try to load and validate a disk file
    async fn try_load_disk(&self, path: &PathBuf) -> Result<DetectedDisk> {
        let bytes = tokio::fs::read(path).await?;

        // Quick magic check
        if bytes.len() < 8 || &bytes[0..8] != DISK_MAGIC {
            return Err(DaemonError::DiskValidationFailed(
                "Invalid magic bytes".to_string(),
            ));
        }

        let format = DiskFormat::from_bytes(&bytes)?;

        // Validate at current time
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        format.validate(current_time)?;

        Ok(DetectedDisk {
            path: path.clone(),
            header: format.header.clone(),
            format: Some(format),
        })
    }

    /// Handle disk removal
    async fn handle_disk_removal(&self) -> Result<()> {
        let mut current = self.current_disk.write().await;
        if let Some(disk) = current.take() {
            info!("Sigil disk removed");
            let _ = self.event_tx.send(DiskEvent::Removed { path: disk.path });
        }
        Ok(())
    }

    /// Load the full disk format (for signing operations)
    pub async fn load_full_disk(&self) -> Result<DiskFormat> {
        let current = self.current_disk.read().await;
        let disk = current
            .as_ref()
            .ok_or(DaemonError::NoDiskDetected)?;

        if let Some(format) = &disk.format {
            Ok(format.clone())
        } else {
            let bytes = tokio::fs::read(&disk.path).await?;
            let format = DiskFormat::from_bytes(&bytes)?;
            Ok(format)
        }
    }

    /// Write updated disk data back to disk
    pub async fn write_disk(&self, format: &DiskFormat) -> Result<()> {
        let current = self.current_disk.read().await;
        let disk = current
            .as_ref()
            .ok_or(DaemonError::NoDiskDetected)?;

        let bytes = format.to_bytes();
        tokio::fs::write(&disk.path, &bytes).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_disk_watcher_creation() {
        let watcher = DiskWatcher::new("/tmp/test_sigil*".to_string());
        assert!(!watcher.has_disk().await);
    }
}
