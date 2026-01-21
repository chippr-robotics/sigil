//! PIN management with secure hashing
//!
//! The PIN protects access to all mother device operations.
//! It is hashed using Argon2id for secure storage.

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::{AuthError, LockoutPolicy};

/// Minimum PIN length
pub const MIN_PIN_LENGTH: usize = 6;
/// Maximum PIN length
pub const MAX_PIN_LENGTH: usize = 12;

/// PIN configuration options
#[derive(Clone, Debug)]
pub struct PinConfig {
    /// Lockout policy
    pub lockout_policy: LockoutPolicy,
    /// Storage path for PIN data
    pub storage_path: PathBuf,
}

impl Default for PinConfig {
    fn default() -> Self {
        Self {
            lockout_policy: LockoutPolicy::default(),
            storage_path: Self::default_path(),
        }
    }
}

impl PinConfig {
    /// Get the default PIN storage path
    pub fn default_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sigil-mother")
            .join("auth.json")
    }

    /// Create config with custom path
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            storage_path: path,
            ..Default::default()
        }
    }
}

/// PIN storage format (persisted to disk)
#[derive(Serialize, Deserialize)]
struct PinStorage {
    /// Argon2id hash of the PIN
    hash: String,
    /// Salt used for encryption key derivation (separate from hash salt)
    #[serde(with = "hex_salt")]
    encryption_salt: [u8; 32],
    /// Number of failed attempts
    failed_attempts: u32,
    /// Timestamp of last failed attempt (Unix epoch seconds)
    last_failed_attempt: Option<u64>,
    /// Version for future migrations
    version: u32,
}

mod hex_salt {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        bytes.try_into().map_err(|_| serde::de::Error::custom("invalid salt length"))
    }
}

/// PIN manager handles PIN storage, verification, and lockout
pub struct PinManager {
    /// Configuration
    config: PinConfig,
    /// Current storage state
    storage: Option<PinStorage>,
    /// Lockout until (if locked)
    lockout_until: Option<Instant>,
}

impl PinManager {
    /// Create a new PIN manager with default config
    pub fn new() -> Result<Self, AuthError> {
        Self::with_config(PinConfig::default())
    }

    /// Create a PIN manager with custom config
    pub fn with_config(config: PinConfig) -> Result<Self, AuthError> {
        // Ensure directory exists
        if let Some(parent) = config.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Load existing storage if present
        let storage = if config.storage_path.exists() {
            let contents = fs::read_to_string(&config.storage_path)?;
            Some(serde_json::from_str(&contents)
                .map_err(|e| AuthError::StorageError(format!("Failed to parse PIN storage: {}", e)))?)
        } else {
            None
        };

        let mut manager = Self {
            config,
            storage,
            lockout_until: None,
        };

        // Check if currently locked out
        manager.check_lockout();

        Ok(manager)
    }

    /// Check if a PIN has been set
    pub fn is_pin_set(&self) -> bool {
        self.storage.is_some()
    }

    /// Set a new PIN (first time setup or change)
    pub fn set_pin(&mut self, pin: &str) -> Result<(), AuthError> {
        // Validate PIN
        Self::validate_pin(pin)?;

        // Hash the PIN using Argon2id
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let pin_bytes = Zeroizing::new(pin.as_bytes().to_vec());

        let hash = argon2
            .hash_password(&pin_bytes, &salt)
            .map_err(|e| AuthError::CryptoError(format!("Failed to hash PIN: {}", e)))?
            .to_string();

        // Generate encryption salt (used for deriving encryption key)
        let mut encryption_salt = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut encryption_salt);

        // Create storage
        let storage = PinStorage {
            hash,
            encryption_salt,
            failed_attempts: 0,
            last_failed_attempt: None,
            version: 1,
        };

        // Save to file
        self.save_storage(&storage)?;
        self.storage = Some(storage);
        self.lockout_until = None;

        Ok(())
    }

    /// Verify a PIN and return the encryption key on success
    ///
    /// The encryption key is derived from the PIN and can be used to
    /// decrypt the master shard.
    pub fn verify_pin(&mut self, pin: &str) -> Result<[u8; 32], AuthError> {
        // Check if locked out
        if let Some(until) = self.lockout_until {
            if Instant::now() < until {
                let remaining = until.duration_since(Instant::now()).as_secs();
                return Err(AuthError::LockedOut(remaining));
            } else {
                self.lockout_until = None;
            }
        }

        // Get storage reference to check PIN
        let storage = self.storage.as_ref()
            .ok_or(AuthError::PinNotSetUp)?;

        // Parse the stored hash
        let parsed_hash = PasswordHash::new(&storage.hash)
            .map_err(|e| AuthError::CryptoError(format!("Invalid stored hash: {}", e)))?;

        // Copy encryption salt before verification (in case we need it for key derivation)
        let encryption_salt = storage.encryption_salt;

        // Verify with constant-time comparison
        let pin_bytes = Zeroizing::new(pin.as_bytes().to_vec());
        let argon2 = Argon2::default();
        let is_correct = argon2.verify_password(&pin_bytes, &parsed_hash).is_ok();

        if is_correct {
            // Reset failed attempts on success
            if let Some(storage) = self.storage.as_mut() {
                storage.failed_attempts = 0;
                storage.last_failed_attempt = None;
            }
            self.save_current_storage()?;

            // Derive encryption key from PIN + encryption salt
            let encryption_key = Self::derive_encryption_key_static(pin, &encryption_salt)?;
            Ok(encryption_key)
        } else {
            // Increment failed attempts
            if let Some(storage) = self.storage.as_mut() {
                storage.failed_attempts += 1;
                storage.last_failed_attempt = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );
            }
            self.save_current_storage()?;

            // Check for lockout
            self.check_lockout();

            let remaining = self.attempts_remaining();
            Err(AuthError::IncorrectPin(remaining))
        }
    }

    /// Derive the encryption key from PIN and salt (static method)
    fn derive_encryption_key_static(pin: &str, salt: &[u8; 32]) -> Result<[u8; 32], AuthError> {
        use argon2::Argon2;

        let mut key = [0u8; 32];
        let argon2 = Argon2::default();

        argon2
            .hash_password_into(pin.as_bytes(), salt, &mut key)
            .map_err(|e| AuthError::CryptoError(format!("Key derivation failed: {}", e)))?;

        Ok(key)
    }

    /// Get the encryption salt (for re-deriving key with correct PIN)
    pub fn encryption_salt(&self) -> Option<[u8; 32]> {
        self.storage.as_ref().map(|s| s.encryption_salt)
    }

    /// Save current storage state to file
    fn save_current_storage(&self) -> Result<(), AuthError> {
        if let Some(storage) = &self.storage {
            self.save_storage(storage)?;
        }
        Ok(())
    }

    /// Validate PIN format
    fn validate_pin(pin: &str) -> Result<(), AuthError> {
        if pin.len() < MIN_PIN_LENGTH || pin.len() > MAX_PIN_LENGTH {
            return Err(AuthError::InvalidPinLength(MIN_PIN_LENGTH, MAX_PIN_LENGTH));
        }

        if !pin.chars().all(|c| c.is_ascii_digit()) {
            return Err(AuthError::InvalidPinFormat);
        }

        Ok(())
    }

    /// Check and update lockout status
    fn check_lockout(&mut self) {
        if let Some(storage) = &self.storage {
            if let Some(duration) = self.config.lockout_policy.lockout_duration(storage.failed_attempts) {
                self.lockout_until = Some(Instant::now() + duration);
            }
        }
    }

    /// Save storage to file
    fn save_storage(&self, storage: &PinStorage) -> Result<(), AuthError> {
        let contents = serde_json::to_string_pretty(storage)
            .map_err(|e| AuthError::StorageError(format!("Failed to serialize: {}", e)))?;

        // Ensure directory exists
        if let Some(parent) = self.config.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write atomically
        let temp_path = self.config.storage_path.with_extension("json.tmp");
        fs::write(&temp_path, &contents)?;
        fs::rename(&temp_path, &self.config.storage_path)?;

        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.config.storage_path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Get the number of remaining attempts before lockout
    pub fn attempts_remaining(&self) -> u32 {
        match &self.storage {
            Some(storage) => {
                let max = self.config.lockout_policy.max_attempts();
                max.saturating_sub(storage.failed_attempts)
            }
            None => 3,
        }
    }

    /// Get the lockout end time if currently locked
    pub fn lockout_until(&self) -> Option<Instant> {
        self.lockout_until.filter(|&until| Instant::now() < until)
    }

    /// Get remaining lockout seconds
    pub fn lockout_remaining_seconds(&self) -> Option<u64> {
        self.lockout_until().map(|until| {
            until.duration_since(Instant::now()).as_secs()
        })
    }

    /// Change PIN (requires current PIN verification first)
    pub fn change_pin(&mut self, current_pin: &str, new_pin: &str) -> Result<(), AuthError> {
        // Verify current PIN first
        let _ = self.verify_pin(current_pin)?;

        // Set new PIN (this will generate new encryption salt)
        self.set_pin(new_pin)?;

        Ok(())
    }

    /// Factory reset - removes PIN and all auth data
    ///
    /// WARNING: This does NOT decrypt the master shard. After reset,
    /// the encrypted master shard will be unrecoverable without the old PIN.
    pub fn factory_reset(&mut self) -> Result<(), AuthError> {
        if self.config.storage_path.exists() {
            fs::remove_file(&self.config.storage_path)?;
        }
        self.storage = None;
        self.lockout_until = None;
        Ok(())
    }
}

impl Default for PinManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default PinManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_manager() -> PinManager {
        let temp_dir = tempdir().unwrap();
        let config = PinConfig::with_path(temp_dir.path().join("auth.json"));
        PinManager::with_config(config).unwrap()
    }

    #[test]
    fn test_pin_set_and_verify() {
        let mut manager = test_manager();

        // Set PIN
        manager.set_pin("123456").unwrap();
        assert!(manager.is_pin_set());

        // Verify correct PIN - should return encryption key
        let key = manager.verify_pin("123456").unwrap();
        assert_eq!(key.len(), 32);

        // Verify incorrect PIN
        assert!(manager.verify_pin("654321").is_err());
    }

    #[test]
    fn test_pin_validation() {
        let mut manager = test_manager();

        // Too short
        assert!(manager.set_pin("123").is_err());

        // Too long
        assert!(manager.set_pin("1234567890123").is_err());

        // Non-digits
        assert!(manager.set_pin("12345a").is_err());

        // Valid
        assert!(manager.set_pin("123456").is_ok());
    }

    #[test]
    fn test_encryption_key_derivation() {
        let mut manager = test_manager();
        manager.set_pin("123456").unwrap();

        // Same PIN should derive same key
        let key1 = manager.verify_pin("123456").unwrap();
        let key2 = manager.verify_pin("123456").unwrap();
        assert_eq!(key1, key2);
    }
}
