//! Error types for the Sigil library

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SigilError>;

#[derive(Error, Debug)]
pub enum SigilError {
    #[error("Keyshard error: {0}")]
    Keyshard(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Blockchain error: {0}")]
    Blockchain(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid keyshard: {0}")]
    InvalidKeyshard(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),
}
