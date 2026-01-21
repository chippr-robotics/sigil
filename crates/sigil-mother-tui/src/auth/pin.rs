//! PIN management with secure hashing

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::LockoutPolicy;

/// Minimum PIN length
pub const MIN_PIN_LENGTH: usize = 6;
/// Maximum PIN length
pub const MAX_PIN_LENGTH: usize = 12;

/// PIN storage format
#[derive(Serialize, Deserialize)]
struct PinStorage {
    /// Argon2 hash of the PIN
    hash: String,
    /// Number of failed attempts
    failed_attempts: u32,
    /// Timestamp of last failed attempt (Unix epoch seconds)
    last_failed_attempt: Option<u64>,
}

/// PIN manager handles PIN storage, verification, and lockout
pub struct PinManager {
    /// Path to the PIN storage file
    storage_path: PathBuf,
    /// Current storage state
    storage: Option<PinStorage>,
    /// Lockout policy
    lockout: LockoutPolicy,
    /// Lockout until (if locked)
    lockout_until: Option<Instant>,
}

impl PinManager {
    /// Load existing PIN or create new manager
    pub fn load_or_create() -> Result<Self> {
        let storage_path = Self::storage_path()?;
        let storage = if storage_path.exists() {
            let contents =
                fs::read_to_string(&storage_path).context("Failed to read PIN storage")?;
            Some(serde_json::from_str(&contents).context("Failed to parse PIN storage")?)
        } else {
            None
        };

        let mut manager = Self {
            storage_path,
            storage,
            lockout: LockoutPolicy::default(),
            lockout_until: None,
        };

        // Check if currently locked out
        manager.check_lockout();

        Ok(manager)
    }

    /// Get the storage path for PIN data
    fn storage_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sigil-mother");

        // Create directory if it doesn't exist
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        Ok(config_dir.join("pin.json"))
    }

    /// Check if a PIN has been set
    pub fn is_pin_set(&self) -> bool {
        self.storage.is_some()
    }

    /// Set a new PIN
    pub fn set_pin(&mut self, pin: &str) -> Result<()> {
        // Validate PIN
        if pin.len() < MIN_PIN_LENGTH || pin.len() > MAX_PIN_LENGTH {
            anyhow::bail!(
                "PIN must be between {} and {} digits",
                MIN_PIN_LENGTH,
                MAX_PIN_LENGTH
            );
        }

        if !pin.chars().all(|c| c.is_ascii_digit()) {
            anyhow::bail!("PIN must contain only digits");
        }

        // Hash the PIN using Argon2id
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let pin_bytes = Zeroizing::new(pin.as_bytes().to_vec());

        let hash = argon2
            .hash_password(&pin_bytes, &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash PIN: {}", e))?
            .to_string();

        // Create storage
        let storage = PinStorage {
            hash,
            failed_attempts: 0,
            last_failed_attempt: None,
        };

        // Save to file
        let contents = serde_json::to_string_pretty(&storage)?;
        fs::write(&self.storage_path, contents).context("Failed to write PIN storage")?;

        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.storage_path, fs::Permissions::from_mode(0o600))?;
        }

        self.storage = Some(storage);
        self.lockout_until = None;

        Ok(())
    }

    /// Verify a PIN
    pub fn verify_pin(&mut self, pin: &str) -> Result<bool> {
        // Check if locked out
        if let Some(until) = self.lockout_until {
            if Instant::now() < until {
                return Ok(false);
            } else {
                self.lockout_until = None;
            }
        }

        let storage = self
            .storage
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No PIN has been set"))?;

        // Parse the stored hash
        let parsed_hash = PasswordHash::new(&storage.hash)
            .map_err(|e| anyhow::anyhow!("Invalid stored hash: {}", e))?;

        // Verify with constant-time comparison
        let pin_bytes = Zeroizing::new(pin.as_bytes().to_vec());
        let argon2 = Argon2::default();

        let result = argon2.verify_password(&pin_bytes, &parsed_hash).is_ok();

        if result {
            // Reset failed attempts on success
            storage.failed_attempts = 0;
            storage.last_failed_attempt = None;
            self.save_storage()?;
        } else {
            // Increment failed attempts
            storage.failed_attempts += 1;
            storage.last_failed_attempt = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            self.save_storage()?;

            // Check for lockout
            self.check_lockout();
        }

        Ok(result)
    }

    /// Check and update lockout status
    fn check_lockout(&mut self) {
        if let Some(storage) = &self.storage {
            if let Some(duration) = self.lockout.lockout_duration(storage.failed_attempts) {
                self.lockout_until = Some(Instant::now() + duration);
            }
        }
    }

    /// Save storage to file
    fn save_storage(&self) -> Result<()> {
        if let Some(storage) = &self.storage {
            let contents = serde_json::to_string_pretty(storage)?;
            fs::write(&self.storage_path, contents).context("Failed to write PIN storage")?;
        }
        Ok(())
    }

    /// Get the number of remaining attempts before lockout
    pub fn attempts_remaining(&self) -> u32 {
        match &self.storage {
            Some(storage) => {
                let max = self.lockout.max_attempts();
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
        self.lockout_until()
            .map(|until| until.duration_since(Instant::now()).as_secs())
    }

    /// Change PIN (requires old PIN verification first)
    pub fn change_pin(&mut self, old_pin: &str, new_pin: &str) -> Result<()> {
        // Verify old PIN first
        if !self.verify_pin(old_pin)? {
            anyhow::bail!("Current PIN is incorrect");
        }

        // Set new PIN
        self.set_pin(new_pin)?;

        Ok(())
    }

    /// Reset PIN storage (factory reset)
    pub fn factory_reset(&mut self) -> Result<()> {
        if self.storage_path.exists() {
            fs::remove_file(&self.storage_path).context("Failed to remove PIN storage")?;
        }
        self.storage = None;
        self.lockout_until = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pin_set_and_verify() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("pin.json");

        let mut manager = PinManager {
            storage_path,
            storage: None,
            lockout: LockoutPolicy::default(),
            lockout_until: None,
        };

        // Set PIN
        manager.set_pin("123456").unwrap();
        assert!(manager.is_pin_set());

        // Verify correct PIN
        assert!(manager.verify_pin("123456").unwrap());

        // Verify incorrect PIN
        assert!(!manager.verify_pin("654321").unwrap());
    }

    #[test]
    fn test_pin_validation() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("pin.json");

        let mut manager = PinManager {
            storage_path,
            storage: None,
            lockout: LockoutPolicy::default(),
            lockout_until: None,
        };

        // Too short
        assert!(manager.set_pin("123").is_err());

        // Too long
        assert!(manager.set_pin("1234567890123").is_err());

        // Non-digits
        assert!(manager.set_pin("12345a").is_err());
    }
}
