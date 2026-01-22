//! Authentication module for Sigil Mother
//!
//! Provides PIN-based authentication and encrypted storage for the master shard.
//! ALL access to the mother device MUST go through this module.
//!
//! # Security Model
//!
//! - PIN is hashed using Argon2id (memory-hard)
//! - Master shard is encrypted at rest using ChaCha20-Poly1305
//! - Encryption key is derived from PIN using Argon2id
//! - Progressive lockout protects against brute force
//! - Session timeout limits exposure time

mod encrypted_storage;
mod lockout;
mod pin;
mod session;

pub use encrypted_storage::EncryptedMotherStorage;
pub use lockout::LockoutPolicy;
pub use pin::{PinConfig, PinManager, MAX_PIN_LENGTH, MIN_PIN_LENGTH};
pub use session::{Session, SessionConfig};

use std::time::Instant;

/// Authentication state for the mother device
#[derive(Clone, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum AuthState {
    /// PIN needs to be set up (first run)
    SetupRequired,
    /// PIN required for authentication
    #[default]
    RequiresPin,
    /// Successfully authenticated with active session
    Authenticated,
    /// Account is locked out until the specified time
    LockedOut(Instant),
}


/// Authentication error types
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("PIN not set up - run initialization first")]
    PinNotSetUp,

    #[error("Incorrect PIN ({0} attempts remaining)")]
    IncorrectPin(u32),

    #[error("Account locked out for {0} seconds")]
    LockedOut(u64),

    #[error("Session expired - please re-authenticate")]
    SessionExpired,

    #[error("PIN must be {0}-{1} digits")]
    InvalidPinLength(usize, usize),

    #[error("PIN must contain only digits")]
    InvalidPinFormat,

    #[error("PINs do not match")]
    PinMismatch,

    #[error("Decryption failed - wrong PIN or corrupted data")]
    DecryptionFailed,

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),
}

impl From<std::io::Error> for AuthError {
    fn from(e: std::io::Error) -> Self {
        AuthError::StorageError(e.to_string())
    }
}
