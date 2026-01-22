//! Floppy disk operations for air-gapped mother device
//!
//! Provides mount, unmount, and format operations for floppy disks.
//! Designed for use on air-gapped systems where automatic mounting
//! may not be configured.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

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

/// Standard 1.44MB floppy disk size in bytes (2880 sectors * 512 bytes)
pub const FLOPPY_SIZE_144MB: u64 = 1_474_560;

/// Tolerance for floppy size detection (allows for filesystem overhead variance)
pub const FLOPPY_SIZE_TOLERANCE: u64 = 10_240;

/// Information about a detected block device
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDevice {
    /// Full device path (e.g., "/dev/sda")
    pub path: String,
    /// Device name without path (e.g., "sda")
    pub name: String,
    /// Device size in bytes
    pub size: u64,
    /// Human-readable size (e.g., "1.4M")
    pub size_human: String,
    /// Whether the device is removable
    pub removable: bool,
    /// Filesystem label (if any)
    pub label: Option<String>,
    /// Filesystem type (e.g., "ext2", "vfat")
    pub fstype: Option<String>,
    /// Current mount point (if mounted)
    pub mountpoint: Option<PathBuf>,
    /// Whether this device is approximately floppy-sized (~1.44MB)
    pub is_floppy_size: bool,
    /// Device model/vendor info (if available)
    pub model: Option<String>,
}

impl BlockDevice {
    /// Check if this device is currently mounted
    pub fn is_mounted(&self) -> bool {
        self.mountpoint.is_some()
    }

    /// Get a display-friendly description of the device
    pub fn display_name(&self) -> String {
        if let Some(label) = &self.label {
            format!("{} ({})", self.path, label)
        } else if let Some(model) = &self.model {
            format!("{} ({})", self.path, model)
        } else {
            self.path.clone()
        }
    }
}

/// JSON structure returned by lsblk
#[derive(Debug, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<LsblkDevice>,
}

/// Individual device in lsblk JSON output
#[derive(Debug, Deserialize)]
struct LsblkDevice {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    rm: bool,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    fstype: Option<String>,
    #[serde(default)]
    mountpoint: Option<String>,
    #[serde(rename = "type", default)]
    device_type: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    #[allow(dead_code)] // May be used for partition handling in future
    children: Option<Vec<LsblkDevice>>,
}

/// Check if a size is approximately that of a 1.44MB floppy disk
fn is_floppy_size(size: u64) -> bool {
    size >= FLOPPY_SIZE_144MB.saturating_sub(FLOPPY_SIZE_TOLERANCE)
        && size <= FLOPPY_SIZE_144MB + FLOPPY_SIZE_TOLERANCE
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// List all removable block devices using lsblk
///
/// Returns a list of removable devices suitable for floppy disk operations.
/// Devices are sorted with floppy-sized devices first.
pub fn list_removable_devices() -> Result<Vec<BlockDevice>> {
    let output = Command::new("lsblk")
        .args([
            "-J",
            "-b",
            "-o",
            "NAME,SIZE,RM,LABEL,FSTYPE,MOUNTPOINT,TYPE,MODEL",
        ])
        .output()
        .map_err(|e| {
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute lsblk: {}", e),
            ))
        })?;

    if !output.status.success() {
        return Err(MotherError::Io(std::io::Error::other(
            format!("lsblk failed: {}", String::from_utf8_lossy(&output.stderr)),
        )));
    }

    let parsed: LsblkOutput = serde_json::from_slice(&output.stdout).map_err(|e| {
        MotherError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse lsblk output: {}", e),
        ))
    })?;

    let mut devices: Vec<BlockDevice> = parsed
        .blockdevices
        .into_iter()
        .filter(|dev| {
            // Only include removable devices that are disks (not partitions)
            dev.rm && dev.device_type.as_deref() == Some("disk")
        })
        .map(|dev| {
            let size = dev.size.unwrap_or(0);
            BlockDevice {
                path: format!("/dev/{}", dev.name),
                name: dev.name,
                size,
                size_human: format_size(size),
                removable: dev.rm,
                label: dev.label,
                fstype: dev.fstype,
                mountpoint: dev.mountpoint.map(PathBuf::from),
                is_floppy_size: is_floppy_size(size),
                model: dev.model,
            }
        })
        .collect();

    // Sort: floppy-sized devices first, then by device name
    devices.sort_by(|a, b| match (a.is_floppy_size, b.is_floppy_size) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.path.cmp(&b.path),
    });

    Ok(devices)
}

/// List all block devices (including non-removable) for testing/debugging
pub fn list_all_block_devices() -> Result<Vec<BlockDevice>> {
    let output = Command::new("lsblk")
        .args([
            "-J",
            "-b",
            "-o",
            "NAME,SIZE,RM,LABEL,FSTYPE,MOUNTPOINT,TYPE,MODEL",
        ])
        .output()
        .map_err(|e| {
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute lsblk: {}", e),
            ))
        })?;

    if !output.status.success() {
        return Err(MotherError::Io(std::io::Error::other(
            format!("lsblk failed: {}", String::from_utf8_lossy(&output.stderr)),
        )));
    }

    let parsed: LsblkOutput = serde_json::from_slice(&output.stdout).map_err(|e| {
        MotherError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse lsblk output: {}", e),
        ))
    })?;

    let devices: Vec<BlockDevice> = parsed
        .blockdevices
        .into_iter()
        .filter(|dev| dev.device_type.as_deref() == Some("disk"))
        .map(|dev| {
            let size = dev.size.unwrap_or(0);
            BlockDevice {
                path: format!("/dev/{}", dev.name),
                name: dev.name,
                size,
                size_human: format_size(size),
                removable: dev.rm,
                label: dev.label,
                fstype: dev.fstype,
                mountpoint: dev.mountpoint.map(PathBuf::from),
                is_floppy_size: is_floppy_size(size),
                model: dev.model,
            }
        })
        .collect();

    Ok(devices)
}

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

/// Mount method preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MountMethod {
    /// Try udisksctl first, fall back to sudo mount
    #[default]
    Auto,
    /// Use udisksctl (doesn't require root, user-space mounting)
    Udisksctl,
    /// Use traditional mount command (may require sudo)
    Traditional,
}

/// Floppy disk manager
pub struct FloppyManager {
    /// Device path (e.g., /dev/fd0)
    device: String,
    /// Mount point (e.g., /mnt/floppy) - used for traditional mount
    mount_point: PathBuf,
    /// Use sudo for mount operations (when using traditional mount)
    use_sudo: bool,
    /// Preferred mount method
    mount_method: MountMethod,
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
            mount_method: MountMethod::Auto,
        }
    }

    /// Create a floppy manager with custom device and mount point
    pub fn with_paths(device: impl Into<String>, mount_point: impl Into<PathBuf>) -> Self {
        Self {
            device: device.into(),
            mount_point: mount_point.into(),
            use_sudo: true,
            mount_method: MountMethod::Auto,
        }
    }

    /// Set whether to use sudo for privileged operations
    pub fn use_sudo(mut self, use_sudo: bool) -> Self {
        self.use_sudo = use_sudo;
        self
    }

    /// Set the mount method preference
    pub fn with_mount_method(mut self, method: MountMethod) -> Self {
        self.mount_method = method;
        self
    }

    /// Set the device path
    pub fn set_device(&mut self, device: impl Into<String>) {
        self.device = device.into();
    }

    /// Set the mount point
    pub fn set_mount_point(&mut self, mount_point: impl Into<PathBuf>) {
        self.mount_point = mount_point.into();
    }

    /// Set the mount method
    pub fn set_mount_method(&mut self, method: MountMethod) {
        self.mount_method = method;
    }

    /// Get the configured device path
    pub fn device(&self) -> &str {
        &self.device
    }

    /// Get the configured mount point
    pub fn mount_point(&self) -> &Path {
        &self.mount_point
    }

    /// Get the mount method
    pub fn mount_method(&self) -> MountMethod {
        self.mount_method
    }

    /// Check if udisksctl is available on the system
    pub fn is_udisksctl_available() -> bool {
        Command::new("which")
            .arg("udisksctl")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
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
    ///
    /// Uses the configured mount method (Auto, Udisksctl, or Traditional).
    /// In Auto mode, tries udisksctl first if available, then falls back to traditional mount.
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
                return Err(MotherError::Io(std::io::Error::other(
                    e,
                )));
            }
            DiskStatus::Unmounted { device } => {
                // Proceed with mount based on configured method
                match self.mount_method {
                    MountMethod::Auto => {
                        // Try udisksctl first, fall back to traditional
                        if Self::is_udisksctl_available() {
                            match self.mount_with_udisksctl(&device) {
                                Ok(path) => return Ok(path),
                                Err(_) => {
                                    // Fall back to traditional mount
                                    self.do_mount(&device)?;
                                }
                            }
                        } else {
                            self.do_mount(&device)?;
                        }
                    }
                    MountMethod::Udisksctl => {
                        return self.mount_with_udisksctl(&device);
                    }
                    MountMethod::Traditional => {
                        self.do_mount(&device)?;
                    }
                }
            }
        }

        // For traditional mount, return the configured mount point
        // For udisksctl, the mount point is returned directly
        Ok(self.mount_point.clone())
    }

    /// Mount using udisksctl (doesn't require root)
    ///
    /// udisksctl mounts to a system-determined location (usually /media/<user>/<label>)
    pub fn mount_with_udisksctl(&self, device: &str) -> Result<PathBuf> {
        let output = Command::new("udisksctl")
            .args(["mount", "-b", device])
            .output()
            .map_err(|e| {
                MotherError::Io(std::io::Error::other(
                    format!("Failed to execute udisksctl: {}", e),
                ))
            })?;

        if output.status.success() {
            // Parse output like: "Mounted /dev/sda at /media/user/SIGIL."
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(idx) = stdout.find(" at ") {
                let mount_point = stdout[idx + 4..].trim().trim_end_matches('.').trim();
                return Ok(PathBuf::from(mount_point));
            }
            // If we can't parse, try to get mount point from lsblk
            if let Ok(devices) = list_all_block_devices() {
                for dev in devices {
                    if dev.path == device {
                        if let Some(mp) = dev.mountpoint {
                            return Ok(mp);
                        }
                    }
                }
            }
            // Last resort: return default mount point
            Ok(self.mount_point.clone())
        } else {
            Err(MotherError::Io(std::io::Error::other(
                format!(
                    "udisksctl mount failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )))
        }
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
                MotherError::Io(std::io::Error::other(
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
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute mount: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::other(
                format!("Mount failed: {}", String::from_utf8_lossy(&output.stderr)),
            )));
        }

        Ok(())
    }

    /// Unmount the floppy disk
    ///
    /// Uses the configured mount method. In Auto mode, tries udisksctl first if available.
    pub fn unmount(&self) -> Result<()> {
        let status = self.check_status();

        match status {
            DiskStatus::Mounted {
                mount_point,
                device,
                ..
            } => {
                match self.mount_method {
                    MountMethod::Auto => {
                        // Try udisksctl first, fall back to traditional
                        if Self::is_udisksctl_available() {
                            match self.unmount_with_udisksctl(&device) {
                                Ok(()) => return Ok(()),
                                Err(_) => {
                                    // Fall back to traditional unmount
                                    self.do_unmount(&mount_point)?;
                                }
                            }
                        } else {
                            self.do_unmount(&mount_point)?;
                        }
                    }
                    MountMethod::Udisksctl => {
                        return self.unmount_with_udisksctl(&device);
                    }
                    MountMethod::Traditional => {
                        self.do_unmount(&mount_point)?;
                    }
                }
            }
            DiskStatus::Unmounted { .. } | DiskStatus::NoDisk => {
                // Already unmounted or no disk
                return Ok(());
            }
            DiskStatus::Error(e) => {
                return Err(MotherError::Io(std::io::Error::other(
                    e,
                )));
            }
        }

        Ok(())
    }

    /// Unmount using udisksctl (doesn't require root)
    pub fn unmount_with_udisksctl(&self, device: &str) -> Result<()> {
        // Sync first to ensure all data is written
        let _ = Command::new("sync").output();

        let output = Command::new("udisksctl")
            .args(["unmount", "-b", device])
            .output()
            .map_err(|e| {
                MotherError::Io(std::io::Error::other(
                    format!("Failed to execute udisksctl: {}", e),
                ))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(MotherError::Io(std::io::Error::other(
                format!(
                    "udisksctl unmount failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )))
        }
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
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute umount: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::other(
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
                return Err(MotherError::Io(std::io::Error::other(
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
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute mkfs.ext2: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::other(
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
                return Err(MotherError::Io(std::io::Error::other(
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
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute mkfs.vfat: {}", e),
            ))
        })?;

        if !output.status.success() {
            return Err(MotherError::Io(std::io::Error::other(
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
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute eject: {}", e),
            ))
        })?;

        if !output.status.success() {
            // Eject may not be available or supported - not a fatal error
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("not found") {
                return Err(MotherError::Io(std::io::Error::other(
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

/// Get detailed information about a specific block device
pub fn get_device_info(device_path: &str) -> Result<Option<BlockDevice>> {
    let devices = list_all_block_devices()?;
    Ok(devices.into_iter().find(|d| d.path == device_path))
}

/// Query the current mount point for a device using lsblk
pub fn get_mount_point(device_path: &str) -> Result<Option<PathBuf>> {
    let output = Command::new("lsblk")
        .args(["-J", "-o", "NAME,MOUNTPOINT", device_path])
        .output()
        .map_err(|e| {
            MotherError::Io(std::io::Error::other(
                format!("Failed to execute lsblk: {}", e),
            ))
        })?;

    if !output.status.success() {
        return Ok(None);
    }

    let parsed: LsblkOutput = serde_json::from_slice(&output.stdout).map_err(|e| {
        MotherError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse lsblk output: {}", e),
        ))
    })?;

    Ok(parsed
        .blockdevices
        .into_iter()
        .next()
        .and_then(|d| d.mountpoint)
        .map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floppy_manager_creation() {
        let manager = FloppyManager::new();
        assert_eq!(manager.device(), DEFAULT_FLOPPY_DEVICE);
        assert_eq!(manager.mount_point(), Path::new(DEFAULT_MOUNT_POINT));
        assert_eq!(manager.mount_method(), MountMethod::Auto);
    }

    #[test]
    fn test_custom_paths() {
        let manager = FloppyManager::with_paths("/dev/sda", "/media/sigil");
        assert_eq!(manager.device(), "/dev/sda");
        assert_eq!(manager.mount_point(), Path::new("/media/sigil"));
    }

    #[test]
    fn test_set_device() {
        let mut manager = FloppyManager::new();
        manager.set_device("/dev/sdb");
        assert_eq!(manager.device(), "/dev/sdb");
    }

    #[test]
    fn test_mount_method() {
        let manager = FloppyManager::new().with_mount_method(MountMethod::Udisksctl);
        assert_eq!(manager.mount_method(), MountMethod::Udisksctl);

        let mut manager2 = FloppyManager::new();
        manager2.set_mount_method(MountMethod::Traditional);
        assert_eq!(manager2.mount_method(), MountMethod::Traditional);
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

    #[test]
    fn test_is_floppy_size() {
        // Exact 1.44MB floppy size
        assert!(is_floppy_size(FLOPPY_SIZE_144MB));
        // Within tolerance
        assert!(is_floppy_size(FLOPPY_SIZE_144MB - 1000));
        assert!(is_floppy_size(FLOPPY_SIZE_144MB + 1000));
        // Outside tolerance
        assert!(!is_floppy_size(1_000_000)); // Too small
        assert!(!is_floppy_size(2_000_000)); // Too large
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1536), "1.5K");
        assert_eq!(format_size(1_048_576), "1.0M");
        assert_eq!(format_size(FLOPPY_SIZE_144MB), "1.4M");
        assert_eq!(format_size(1_073_741_824), "1.0G");
    }

    #[test]
    fn test_block_device_display_name() {
        let dev_with_label = BlockDevice {
            path: "/dev/sda".to_string(),
            name: "sda".to_string(),
            size: FLOPPY_SIZE_144MB,
            size_human: "1.4M".to_string(),
            removable: true,
            label: Some("SIGIL".to_string()),
            fstype: Some("ext2".to_string()),
            mountpoint: None,
            is_floppy_size: true,
            model: None,
        };
        assert_eq!(dev_with_label.display_name(), "/dev/sda (SIGIL)");

        let dev_with_model = BlockDevice {
            path: "/dev/sda".to_string(),
            name: "sda".to_string(),
            size: FLOPPY_SIZE_144MB,
            size_human: "1.4M".to_string(),
            removable: true,
            label: None,
            fstype: None,
            mountpoint: None,
            is_floppy_size: true,
            model: Some("USB Floppy".to_string()),
        };
        assert_eq!(dev_with_model.display_name(), "/dev/sda (USB Floppy)");

        let dev_plain = BlockDevice {
            path: "/dev/sda".to_string(),
            name: "sda".to_string(),
            size: FLOPPY_SIZE_144MB,
            size_human: "1.4M".to_string(),
            removable: true,
            label: None,
            fstype: None,
            mountpoint: None,
            is_floppy_size: true,
            model: None,
        };
        assert_eq!(dev_plain.display_name(), "/dev/sda");
    }
}
