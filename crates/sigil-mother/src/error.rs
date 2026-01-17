//! Error types for the Sigil mother device

use thiserror::Error;

/// Result type alias for mother operations
pub type Result<T> = std::result::Result<T, MotherError>;

/// Errors that can occur in mother operations
#[derive(Debug, Error)]
pub enum MotherError {
    /// Core library error
    #[error("Core error: {0}")]
    Core(#[from] sigil_core::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Master key not initialized
    #[error("Master key not initialized - run 'sigil-mother init' first")]
    MasterKeyNotInitialized,

    /// Child not found in registry
    #[error("Child not found: {0}")]
    ChildNotFound(String),

    /// Child already exists
    #[error("Child already exists with ID: {0}")]
    ChildAlreadyExists(String),

    /// Child is nullified
    #[error("Child is nullified: {0}")]
    ChildNullified(String),

    /// Derivation path already used
    #[error("Derivation path already used: {0}")]
    DerivationPathUsed(String),

    /// Invalid disk format
    #[error("Invalid disk format: {0}")]
    InvalidDiskFormat(String),

    /// Reconciliation failed
    #[error("Reconciliation failed: {0}")]
    ReconciliationFailed(String),

    /// Presig generation failed
    #[error("Presig generation failed: {0}")]
    PresigGenerationFailed(String),

    /// Cryptographic error
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// MPC protocol error
    #[error("MPC error: {0}")]
    Mpc(String),
}

impl From<bincode::Error> for MotherError {
    fn from(e: bincode::Error) -> Self {
        MotherError::Serialization(e.to_string())
    }
}

impl From<serde_json::Error> for MotherError {
    fn from(e: serde_json::Error) -> Self {
        MotherError::Serialization(e.to_string())
    }
}
