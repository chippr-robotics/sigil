//! Error types for the Sigil daemon

use thiserror::Error;

/// Result type alias for daemon operations
pub type Result<T> = std::result::Result<T, DaemonError>;

/// Errors that can occur in the daemon
#[derive(Debug, Error)]
pub enum DaemonError {
    /// Core library error
    #[error("Core error: {0}")]
    Core(#[from] sigil_core::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// No disk detected
    #[error("No signing disk detected")]
    NoDiskDetected,

    /// Multiple disks detected
    #[error("Multiple signing disks detected - please insert only one")]
    MultipleDisksDetected,

    /// Disk validation failed
    #[error("Disk validation failed: {0}")]
    DiskValidationFailed(String),

    /// Agent shard not found
    #[error("Agent shard not found for child: {0}")]
    AgentShardNotFound(String),

    /// Presignature mismatch
    #[error("Presignature mismatch: {0}")]
    PresigMismatch(String),

    /// Signing failed
    #[error("Signing failed: {0}")]
    SigningFailed(String),

    /// zkVM proof generation failed
    #[error("zkVM proof generation failed: {0}")]
    ZkProofFailed(String),

    /// IPC error
    #[error("IPC error: {0}")]
    Ipc(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Udev error
    #[error("Udev error: {0}")]
    Udev(String),

    /// Store error
    #[error("Store error: {0}")]
    Store(String),

    /// Timeout
    #[error("Operation timed out")]
    Timeout,

    /// Operation cancelled
    #[error("Operation cancelled")]
    Cancelled,
}

impl From<bitcode::Error> for DaemonError {
    fn from(e: bitcode::Error) -> Self {
        DaemonError::Serialization(e.to_string())
    }
}

impl From<serde_json::Error> for DaemonError {
    fn from(e: serde_json::Error) -> Self {
        DaemonError::Serialization(e.to_string())
    }
}
