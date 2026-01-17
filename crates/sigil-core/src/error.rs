//! Error types for Sigil operations

use thiserror::Error;

/// Result type alias using the Sigil error type
pub type Result<T> = core::result::Result<T, Error>;

/// Errors that can occur during Sigil operations
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid disk magic bytes
    #[error("Invalid disk magic: expected 'SIGILDSK'")]
    InvalidMagic,

    /// Unsupported disk format version
    #[error("Unsupported disk version: {0}")]
    UnsupportedVersion(u32),

    /// Disk has expired
    #[error("Disk has expired at timestamp {0}")]
    DiskExpired(u64),

    /// Reconciliation deadline passed
    #[error("Reconciliation deadline passed at timestamp {0}")]
    ReconciliationDeadlinePassed(u64),

    /// Maximum uses before reconciliation exceeded
    #[error("Maximum uses before reconciliation exceeded: {used}/{max}")]
    MaxUsesExceeded { used: u32, max: u32 },

    /// No presignatures available
    #[error("No presignatures available (used: {used}, total: {total})")]
    NoPresigsAvailable { used: u32, total: u32 },

    /// Presignature already used
    #[error("Presignature at index {0} already used")]
    PresigAlreadyUsed(u32),

    /// Presignature voided
    #[error("Presignature at index {0} has been voided")]
    PresigVoided(u32),

    /// Invalid presignature index
    #[error("Invalid presignature index: {index} (max: {max})")]
    InvalidPresigIndex { index: u32, max: u32 },

    /// Invalid mother signature
    #[error("Invalid mother signature on disk header")]
    InvalidMotherSignature,

    /// Child disk nullified
    #[error("Child disk has been nullified: {reason}")]
    ChildNullified { reason: String },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Cryptographic error
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// IO error (only available with std feature)
    #[cfg(feature = "std")]
    #[error("IO error: {0}")]
    Io(String),

    /// Disk not found
    #[error("Disk not found at path: {0}")]
    DiskNotFound(String),

    /// Invalid derivation path
    #[error("Invalid derivation path: {0}")]
    InvalidDerivationPath(String),

    /// Signature verification failed
    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    /// Presignature R point mismatch
    #[error("Presignature R points do not match")]
    PresigRPointMismatch,

    /// Usage log full
    #[error("Usage log is full")]
    UsageLogFull,

    /// Usage log anomaly detected
    #[error("Usage log anomaly: {0}")]
    UsageLogAnomaly(String),
}

impl From<bincode::Error> for Error {
    fn from(e: bincode::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}
