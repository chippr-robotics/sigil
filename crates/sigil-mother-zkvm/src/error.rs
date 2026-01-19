//! Error types for sigil-mother-zkvm

use thiserror::Error;

/// Result type for zkVM operations
pub type Result<T> = std::result::Result<T, ZkvmError>;

/// Errors that can occur during zkVM operations
#[derive(Debug, Error)]
pub enum ZkvmError {
    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// Invalid input data
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Proof generation failed
    #[error("Proof generation failed: {0}")]
    ProofGenerationFailed(String),

    /// Proof verification failed
    #[error("Proof verification failed: {0}")]
    ProofVerificationFailed(String),

    /// SP1 SDK error
    #[error("SP1 error: {0}")]
    Sp1Error(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Merkle tree error
    #[error("Merkle tree error: {0}")]
    MerkleTree(String),

    /// Program not found
    #[error("SP1 program not found: {0}")]
    ProgramNotFound(String),

    /// Feature not enabled
    #[error("Feature not enabled: {0}")]
    FeatureNotEnabled(String),
}

impl From<serde_json::Error> for ZkvmError {
    fn from(e: serde_json::Error) -> Self {
        ZkvmError::Serialization(e.to_string())
    }
}
