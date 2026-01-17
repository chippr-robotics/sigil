//! Error types for FROST operations

use thiserror::Error;

/// Result type for FROST operations
pub type Result<T> = std::result::Result<T, FrostError>;

/// Errors that can occur during FROST operations
#[derive(Debug, Error)]
pub enum FrostError {
    /// Invalid number of participants
    #[error("Invalid participant count: need at least {min}, got {got}")]
    InvalidParticipantCount { min: usize, got: usize },

    /// Invalid threshold
    #[error("Invalid threshold: {threshold} must be <= {participants} and >= 2")]
    InvalidThreshold {
        threshold: usize,
        participants: usize,
    },

    /// Key generation failed
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),

    /// Nonce generation failed
    #[error("Nonce generation failed: {0}")]
    NonceGeneration(String),

    /// Signing failed
    #[error("Signing failed: {0}")]
    Signing(String),

    /// Signature aggregation failed
    #[error("Signature aggregation failed: {0}")]
    Aggregation(String),

    /// Invalid signature
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Invalid key share
    #[error("Invalid key share: {0}")]
    InvalidKeyShare(String),

    /// Nonce reuse detected
    #[error("Nonce reuse detected - this would compromise security")]
    NonceReuse,

    /// No more presignatures available
    #[error("No presignatures remaining on disk")]
    NoPresigsRemaining,

    /// Presignature index mismatch
    #[error("Presignature index mismatch: expected {expected}, got {got}")]
    PresigIndexMismatch { expected: u32, got: u32 },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Unsupported signature scheme
    #[error("Unsupported signature scheme: {0}")]
    UnsupportedScheme(String),

    /// Feature not enabled
    #[error("Feature not enabled: {0}. Rebuild with --features {0}")]
    FeatureNotEnabled(String),

    /// Internal error
    #[error("Internal FROST error: {0}")]
    Internal(String),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

impl From<bitcode::Error> for FrostError {
    fn from(e: bitcode::Error) -> Self {
        FrostError::Serialization(e.to_string())
    }
}
