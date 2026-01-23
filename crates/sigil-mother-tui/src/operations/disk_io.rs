//! Disk I/O operations for the TUI

use anyhow::Result;
use std::path::PathBuf;

/// Detected disk information
#[derive(Clone, Debug)]
pub struct DetectedDisk {
    /// Mount path
    pub path: PathBuf,
    /// Device name
    pub device: String,
    /// Whether it's a valid Sigil disk
    pub is_sigil: bool,
    /// Child ID if valid Sigil disk
    pub child_id: Option<String>,
    /// Free space in bytes
    pub free_space: u64,
}

/// Disk detector for finding floppy drives
pub struct DiskDetector {
    /// Mount path pattern to search
    #[allow(dead_code)] // Reserved for future use with custom patterns
    pattern: String,
}

impl DiskDetector {
    /// Create a new detector with default pattern
    pub fn new() -> Self {
        Self {
            pattern: "/media/*/SIGIL*".to_string(),
        }
    }

    /// Create with custom pattern
    pub fn with_pattern(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }

    /// Check for detected disks
    pub fn detect(&self) -> Result<Option<DetectedDisk>> {
        // In a real implementation, this would use glob and check for SIGIL magic
        // For now, return None
        Ok(None)
    }

    /// Check if a specific path is a valid Sigil disk
    pub fn validate_disk(&self, _path: &PathBuf) -> Result<bool> {
        // Would read first bytes and check for SIGILDSK magic
        Ok(false)
    }
}

impl Default for DiskDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// USB drive detector for report export
pub struct UsbDetector;

impl UsbDetector {
    /// Detect USB drives
    pub fn detect() -> Result<Vec<DetectedDisk>> {
        // In a real implementation, scan /media for USB drives
        Ok(vec![])
    }
}

/// Safe disk ejection
pub fn eject_disk(_path: &PathBuf) -> Result<()> {
    // Would use udisksctl or similar
    Ok(())
}
