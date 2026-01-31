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
    Inserted { path: PathBuf, header: DiskHeader },
    /// A disk was removed
    Removed { path: PathBuf },
    /// Disk validation failed
    ValidationFailed { path: PathBuf, reason: String },
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

    /// Get the current disk path for memory management
    pub async fn get_current_disk_path(&self) -> Option<PathBuf> {
        self.current_disk.read().await.as_ref().map(|d| d.path.clone())
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

            let monitor = builder
                .listen()
                .map_err(|e| DaemonError::Udev(e.to_string()))?;

            let mut socket =
                AsyncMonitorSocket::new(monitor).map_err(|e| DaemonError::Udev(e.to_string()))?;

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
                                // Always clear cache on remove event, then rescan
                                // to handle cases where kernel caching shows stale data
                                self.handle_disk_removal().await?;
                                // Small delay then rescan to detect any new disk
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                self.scan_for_disks().await?;
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        error!("Udev error: {}", e);
                    }
                    None => break Ok(()),
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback: poll for changes on non-Linux systems
            // This loop runs indefinitely on non-Linux platforms
            warn!("Udev not available, using polling fallback");
            loop {
                self.scan_for_disks().await?;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }

    /// Scan for Sigil disks
    async fn scan_for_disks(&self) -> Result<()> {
        // First, verify any currently cached disk is still valid
        // This handles the case where a disk was physically removed but
        // the mount point or cached data still exists
        if self.verify_current_disk().await.is_err() {
            self.handle_disk_removal().await?;
        }

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
                        // Check if this is a different disk than currently cached
                        let is_new_disk = {
                            let current = self.current_disk.read().await;
                            match current.as_ref() {
                                Some(current_disk) => {
                                    current_disk.header.child_id != disk.header.child_id
                                }
                                None => true,
                            }
                        };

                        if is_new_disk {
                            let header = disk.header.clone();
                            let path = disk_file.clone();

                            let mut current = self.current_disk.write().await;
                            *current = Some(disk);

                            let _ = self.event_tx.send(DiskEvent::Inserted { path, header });
                            info!("Sigil disk detected");
                        }
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

        // No valid disk found - clear any stale cache
        if self.current_disk.read().await.is_some() {
            self.handle_disk_removal().await?;
        }

        Ok(())
    }

    /// Verify the currently cached disk is still accessible and valid
    /// Returns Ok(()) if disk is valid, Err if disk should be invalidated
    async fn verify_current_disk(&self) -> Result<()> {
        let current = self.current_disk.read().await;
        let disk = match current.as_ref() {
            Some(d) => d,
            None => return Ok(()), // No disk cached, nothing to verify
        };

        // Try to read the first few bytes to verify disk is still accessible
        // Use a fresh read to bypass any kernel caching
        match tokio::fs::read(&disk.path).await {
            Ok(bytes) => {
                // Verify magic bytes are still valid
                if bytes.len() < 8 || &bytes[0..8] != DISK_MAGIC {
                    debug!("Cached disk failed magic check - disk removed or corrupted");
                    return Err(DaemonError::DiskValidationFailed(
                        "Disk no longer valid".to_string(),
                    ));
                }
                // Verify it's the same disk by checking child_id
                if let Ok(format) = DiskFormat::from_bytes(&bytes) {
                    if format.header.child_id != disk.header.child_id {
                        debug!("Disk child_id changed - different disk inserted");
                        return Err(DaemonError::DiskValidationFailed(
                            "Different disk detected".to_string(),
                        ));
                    }
                }
                Ok(())
            }
            Err(e) => {
                debug!(
                    "Failed to read cached disk path: {} - treating as removed",
                    e
                );
                Err(DaemonError::DiskValidationFailed(e.to_string()))
            }
        }
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
    /// This always re-reads from disk to ensure we have fresh data
    /// and the disk is still physically present
    pub async fn load_full_disk(&self) -> Result<DiskFormat> {
        // First verify the disk is still valid
        self.verify_current_disk()
            .await
            .map_err(|_| DaemonError::NoDiskDetected)?;

        let current = self.current_disk.read().await;
        let disk = current.as_ref().ok_or(DaemonError::NoDiskDetected)?;

        // Always read fresh from disk for signing operations
        // This ensures we catch any disk removal between operations
        let bytes = tokio::fs::read(&disk.path).await?;
        let format = DiskFormat::from_bytes(&bytes)?;

        // Verify this is still the expected disk
        if format.header.child_id != disk.header.child_id {
            return Err(DaemonError::DiskValidationFailed(
                "Disk changed during operation".to_string(),
            ));
        }

        Ok(format)
    }

    /// Force re-verification of the current disk
    /// Returns true if a valid disk is present, false otherwise
    pub async fn force_verify(&self) -> bool {
        match self.verify_current_disk().await {
            Ok(()) => true,
            Err(_) => {
                // Clear stale cache
                let _ = self.handle_disk_removal().await;
                false
            }
        }
    }

    /// Write updated disk data back to disk
    pub async fn write_disk(&self, format: &DiskFormat) -> Result<()> {
        let path = {
            let current = self.current_disk.read().await;
            let disk = current.as_ref().ok_or(DaemonError::NoDiskDetected)?;
            disk.path.clone()
        };

        let bytes = format.to_bytes();
        tokio::fs::write(&path, &bytes).await?;

        // Sync to ensure data is flushed to physical disk (important for floppies)
        if let Ok(file) = std::fs::File::open(&path) {
            let _ = file.sync_all();
        }

        // Update the cached disk with the new header
        let mut current = self.current_disk.write().await;
        if let Some(ref mut disk) = *current {
            disk.header = format.header.clone();
            disk.format = Some(format.clone());
        }

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
