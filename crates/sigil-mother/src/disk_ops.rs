//! Floppy disk operations for air-gapped mother device
//!
//! Provides mount, unmount, and format operations for floppy disks.
//! Designed for use on air-gapped systems where automatic mounting
//! may not be configured.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{MotherError, Result};

/// Default floppy device path on Linux
pub const DEFAULT_FLOPPY_DEVICE: &str = "/dev/fd0";

/// Default mount point for floppy disks
pub const DEFAULT_MOUNT_POINT: &str = "/mnt/floppy";

/// Alternative floppy devices to check (USB floppy drives)
pub const ALTERNATIVE_DEVICES: &[&str] = &[
    "/dev/fd0",
    "/dev/sda", // USB floppy often appears as sda on systems without other drives
    "/dev/sdb",
    "/dev/disk/by-id/usb-*floppy*",
];

/// Disk status information
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskStatus {
    /// No floppy disk detected in drive
    NoDisk,
    /// Disk detected but not mounted
    Unmounted { device: String },
    /// Disk is mounted and ready
    Mounted {
        device: String,
        mount_point: PathBuf,
        filesystem: String,
        is_sigil_disk: bool,
    },
    /// Error detecting disk status
    Error(String),
}

impl DiskStatus {
    /// Check if a disk is available for operations
    pub fn is_available(&self) -> bool {
        matches!(self, DiskStatus::Mounted { .. })
    }

    /// Check if this is a Sigil-formatted disk
    pub fn is_sigil_disk(&self) -> bool {
        matches!(
            self,
            DiskStatus::Mounted {
                is_sigil_disk: true,
                ..
            }
        )
    }

    /// Get the mount point if mounted
    pub fn mount_point(&self) -> Option<&Path> {
        match self {
            DiskStatus::Mounted { mount_point, .. } => Some(mount_point),
            _ => None,
        }
    }

    /// Get the device path
    pub fn device(&self) -> Option<&str> {
        match self {
            DiskStatus::Unmounted { device } | DiskStatus::Mounted { device, .. } => {
                Some(device.as_str())
            }
            _ => None,
        }
    }
}

/// Floppy disk manager
pub struct FloppyManager {
    /// Device path (e.g., /dev/fd0)
    device: String,
    /// Mount point (e.g., /mnt/floppy)
    mount_point: PathBuf,
    /// Use sudo for mount operations
    use_sudo: bool,
}

impl Default for FloppyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FloppyManager {
    /// Create a new floppy manager with default settings
    pub fn new() -> Self {
        Self {
            device: DEFAULT_FLOPPY_DEVICE.to_string(),
            mount_point: PathBuf::from(DEFAULT_MOUNT_POINT),
            use_sudo: true, // Most systems require root for mount
        }
    }

    /// Create a floppy manager with custom device and mount point
    pub fn with_paths(device: impl Into<String>, mount_point: impl Into<PathBuf>) -> Self {
        Self {
            device: device.into(),
            mount_point: mount_point.into(),
            use_sudo: true,
        }
    }

    /// Set whether to use sudo for privileged operations
    pub fn use_sudo(mut self, use_sudo: bool) -> Self {
        self.use_sudo = use_sudo;
        self
    }

    /// Get the configured device path
    pub fn device(&self) -> &str {
        &self.device
    }

    /// Get the configured mount point
    pub fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    /// Check current disk status
    pub fn check_status(&self) -> DiskStatus {
        // First check if device exists
        if !Path::new(&self.device).exists() {
            // Try to find alternative devices
            if let Some(alt_device) = self.find_floppy_device() {
                return self.check_device_status(&alt_device);
            }
            return DiskStatus::NoDisk;
        }

        self.check_device_status(&self.device)
    }

    /// Check status of a specific device
    fn check_device_status(&self, device: &str) -> DiskStatus {
        // Check if already mounted by reading /proc/mounts
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 && parts[0] == device {
                    let mount_point = PathBuf::from(parts[1]);
                    let filesystem = parts[2].to_string();
                    let is_sigil = self.check_sigil_disk(&mount_point);
                    return DiskStatus::Mounted {
                        device: device.to_string(),
                        mount_point,
                        filesystem,
                        is_sigil_disk: is_sigil,
                    };
                }
            }
        }

        // Check if device has media inserted (for floppy drives)
        if self.has_media(device) {
            DiskStatus::Unmounted {
                device: device.to_string(),
            }
        } else {
            DiskStatus::NoDisk
        }
    }

    /// Check if the floppy drive has media inserted
    fn has_media(&self, device: &str) -> bool {
        // Try to read device block size - this fails if no media
        let output = Command::new("blockdev")
            .arg("--getsize64")
            .arg(device)
            .output();

        match output {
            Ok(o) => o.status.success(),
            Err(_) => {
                // Fallback: try to stat the device
                Path::new(device).exists()
            }
        }
    }

    /// Find a floppy device on the system
    fn find_floppy_device(&self) -> Option<String> {
        for pattern in ALTERNATIVE_DEVICES {
            if pattern.contains('*') {
                // Glob pattern - skip for now
                continue;
            }
            if Path::new(pattern).exists() {
                return Some(pattern.to_string());
            }
        }
        None
    }

    /// Check if a mounted disk is a Sigil-formatted disk
    fn check_sigil_disk(&self, mount_point: &Path) -> bool {
        let sigil_file = mount_point.join("sigil.disk");
        sigil_file.exists()
    }

    /// Mount the floppy disk
    pub fn mount(&self) -> Result<PathBuf> {
        let status = self.check_status();

        match status {
            DiskStatus::NoDisk => {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No floppy disk detected. Please insert a disk.",
                )));
            }
            DiskStatus::Mounted { mount_point, .. } => {
                // Already mounted
                return Ok(mount_point);
            }
            DiskStatus::Error(e) => {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )));
            }
            DiskStatus::Unmounted { device } => {
                // Proceed with mount
                self.do_mount(&device)?;
            }
        }

        Ok(self.mount_point.clone())
    }

    /// Perform the actual mount operation
    fn do_mount(&self, device: &str) -> Result<()> {
        // Ensure mount point exists
        if !self.mount_point.exists() {
            let mut cmd = if self.use_sudo {
                let mut c = Command::new("sudo");
                c.arg("mkdir").arg("-p").arg(&self.mount_point);
                c
            } else {
                let mut c = Command::new("mkdir");
                c.arg("-p").arg(&self.mount_point);
                c
            };

            let output = cmd.output().map_err(|e| {
                MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create mount point: {}", e),
                ))
            })?;

            if !output.status.success() {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!(
                        "Failed to create mount point: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                )));
            }
        }

        // Mount the device
        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("mount")
                .arg("-o")
                .arg("rw,sync,noatime")
                .arg(device)
                .arg(&self.mount_point);
            c
        } else {
            let mut c = Command::new("mount");
            c.arg("-o")
                .arg("rw,sync,noatime")
                .arg(device)
                .arg(&self.mount_point);
            c
        };

        let output = cmd.output().map_err(|e| {
            MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to execute mount: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Mount failed: {}", String::from_utf8_lossy(&output.stderr)),
            )));
        }

        Ok(())
    }

    /// Unmount the floppy disk
    pub fn unmount(&self) -> Result<()> {
        let status = self.check_status();

        match status {
            DiskStatus::Mounted { mount_point, .. } => {
                self.do_unmount(&mount_point)?;
            }
            DiskStatus::Unmounted { .. } | DiskStatus::NoDisk => {
                // Already unmounted or no disk
                return Ok(());
            }
            DiskStatus::Error(e) => {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )));
            }
        }

        Ok(())
    }

    /// Perform the actual unmount operation
    fn do_unmount(&self, mount_point: &Path) -> Result<()> {
        // Sync first to ensure all data is written
        let _ = Command::new("sync").output();

        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("umount").arg(mount_point);
            c
        } else {
            let mut c = Command::new("umount");
            c.arg(mount_point);
            c
        };

        let output = cmd.output().map_err(|e| {
            MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to execute umount: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Unmount failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )));
        }

        Ok(())
    }

    /// Format the floppy disk with ext2 filesystem
    ///
    /// WARNING: This will destroy all data on the disk!
    pub fn format(&self, label: Option<&str>) -> Result<()> {
        let status = self.check_status();

        let device = match &status {
            DiskStatus::Unmounted { device } => device.clone(),
            DiskStatus::Mounted { device, .. } => {
                // Must unmount first
                self.unmount()?;
                device.clone()
            }
            DiskStatus::NoDisk => {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No floppy disk detected. Please insert a disk.",
                )));
            }
            DiskStatus::Error(e) => {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.clone(),
                )));
            }
        };

        // Format with ext2 (good for floppies, no journal overhead)
        let volume_label = label.unwrap_or("SIGIL");

        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("mkfs.ext2")
                .arg("-L")
                .arg(volume_label)
                .arg("-m")
                .arg("0") // No reserved blocks
                .arg("-q") // Quiet
                .arg(&device);
            c
        } else {
            let mut c = Command::new("mkfs.ext2");
            c.arg("-L")
                .arg(volume_label)
                .arg("-m")
                .arg("0")
                .arg("-q")
                .arg(&device);
            c
        };

        let output = cmd.output().map_err(|e| {
            MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to execute mkfs.ext2: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Format failed: {}", String::from_utf8_lossy(&output.stderr)),
            )));
        }

        Ok(())
    }

    /// Format the floppy disk with FAT12 filesystem (more compatible)
    ///
    /// WARNING: This will destroy all data on the disk!
    pub fn format_fat(&self, label: Option<&str>) -> Result<()> {
        let status = self.check_status();

        let device = match &status {
            DiskStatus::Unmounted { device } => device.clone(),
            DiskStatus::Mounted { device, .. } => {
                // Must unmount first
                self.unmount()?;
                device.clone()
            }
            DiskStatus::NoDisk => {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No floppy disk detected. Please insert a disk.",
                )));
            }
            DiskStatus::Error(e) => {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.clone(),
                )));
            }
        };

        // Format with FAT12 (maximum compatibility)
        let volume_label = label.unwrap_or("SIGIL");

        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("mkfs.vfat")
                .arg("-n")
                .arg(volume_label)
                .arg("-F")
                .arg("12") // FAT12 for floppies
                .arg(&device);
            c
        } else {
            let mut c = Command::new("mkfs.vfat");
            c.arg("-n")
                .arg(volume_label)
                .arg("-F")
                .arg("12")
                .arg(&device);
            c
        };

        let output = cmd.output().map_err(|e| {
            MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to execute mkfs.vfat: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Format failed: {}", String::from_utf8_lossy(&output.stderr)),
            )));
        }

        Ok(())
    }

    /// Eject the floppy disk (useful for USB floppy drives)
    pub fn eject(&self) -> Result<()> {
        // First unmount if mounted
        self.unmount()?;

        let device = self.find_floppy_device().unwrap_or(self.device.clone());

        let mut cmd = if self.use_sudo {
            let mut c = Command::new("sudo");
            c.arg("eject").arg(&device);
            c
        } else {
            let mut c = Command::new("eject");
            c.arg(&device);
            c
        };

        let output = cmd.output().map_err(|e| {
            MotherError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to execute eject: {}", e),
            ))
        })?;

        if !output.status.success() {
            // Eject may not be available or supported - not a fatal error
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("not found") {
                return Err(MotherError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Eject failed: {}", stderr),
                )));
            }
        }

        Ok(())
    }
}

/// Format type for disk formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatType {
    /// ext2 filesystem (Linux native, good for Sigil)
    #[default]
    Ext2,
    /// FAT12 filesystem (maximum compatibility)
    Fat12,
}

impl FormatType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            FormatType::Ext2 => "ext2",
            FormatType::Fat12 => "FAT12",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            FormatType::Ext2 => "Linux ext2 (recommended for Sigil)",
            FormatType::Fat12 => "FAT12 (maximum compatibility)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floppy_manager_creation() {
        let manager = FloppyManager::new();
        assert_eq!(manager.device(), DEFAULT_FLOPPY_DEVICE);
        assert_eq!(manager.mount_point(), Path::new(DEFAULT_MOUNT_POINT));
    }

    #[test]
    fn test_custom_paths() {
        let manager = FloppyManager::with_paths("/dev/sda", "/media/sigil");
        assert_eq!(manager.device(), "/dev/sda");
        assert_eq!(manager.mount_point(), Path::new("/media/sigil"));
    }

    #[test]
    fn test_disk_status_methods() {
        let mounted = DiskStatus::Mounted {
            device: "/dev/fd0".to_string(),
            mount_point: PathBuf::from("/mnt/floppy"),
            filesystem: "ext2".to_string(),
            is_sigil_disk: true,
        };

        assert!(mounted.is_available());
        assert!(mounted.is_sigil_disk());
        assert_eq!(mounted.mount_point(), Some(Path::new("/mnt/floppy")));
        assert_eq!(mounted.device(), Some("/dev/fd0"));

        let unmounted = DiskStatus::Unmounted {
            device: "/dev/fd0".to_string(),
        };
        assert!(!unmounted.is_available());
        assert!(!unmounted.is_sigil_disk());

        let no_disk = DiskStatus::NoDisk;
        assert!(!no_disk.is_available());
        assert!(no_disk.device().is_none());
    }
}
