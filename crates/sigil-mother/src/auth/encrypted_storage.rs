//! Encrypted storage for master shard protection
//!
//! The master shard is encrypted at rest using ChaCha20-Poly1305 with a key
//! derived from the user's PIN via Argon2id.
//!
//! # Storage Format
//!
//! The encrypted file contains:
//! - 12-byte nonce
//! - Encrypted MasterShardData (JSON serialized, then encrypted)
//! - 16-byte authentication tag (appended by ChaCha20-Poly1305)

use std::path::PathBuf;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use super::{AuthError, Session};
use crate::registry::ChildRegistry;
use crate::storage::MasterShardData;

/// Size of the nonce for ChaCha20-Poly1305
const NONCE_SIZE: usize = 12;

/// Encrypted storage header (stored alongside encrypted data)
/// Reserved for future storage format migrations.
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
struct EncryptedHeader {
    /// Version for future migrations
    version: u32,
    /// Algorithm identifier
    algorithm: String,
}

/// Authenticated mother storage
///
/// This wraps the underlying storage and enforces PIN-based authentication.
/// The master shard is encrypted at rest and can only be accessed with a valid session.
pub struct EncryptedMotherStorage {
    /// Base path for storage
    base_path: PathBuf,
}

impl EncryptedMotherStorage {
    /// Create a new encrypted storage instance
    pub fn new(base_path: PathBuf) -> Result<Self, AuthError> {
        std::fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    /// Get the default storage path
    pub fn default_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sigil-mother")
    }

    /// Check if encrypted master shard exists
    pub fn has_master_shard(&self) -> bool {
        self.encrypted_shard_path().exists()
    }

    /// Save master shard with encryption
    pub fn save_master_shard(
        &self,
        data: &MasterShardData,
        encryption_key: &[u8; 32],
    ) -> Result<(), AuthError> {
        // Serialize the data
        let plaintext = serde_json::to_string(data)
            .map_err(|e| AuthError::StorageError(format!("Serialization failed: {}", e)))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let cipher = ChaCha20Poly1305::new_from_slice(encryption_key)
            .map_err(|e| AuthError::CryptoError(format!("Invalid key: {}", e)))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| AuthError::CryptoError(format!("Encryption failed: {}", e)))?;

        // Combine nonce + ciphertext
        let mut encrypted_data = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        encrypted_data.extend_from_slice(&nonce_bytes);
        encrypted_data.extend_from_slice(&ciphertext);

        // Write atomically
        let path = self.encrypted_shard_path();
        let temp_path = path.with_extension("enc.tmp");
        std::fs::write(&temp_path, &encrypted_data)?;
        std::fs::rename(&temp_path, &path)?;

        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Load master shard with decryption
    pub fn load_master_shard(&self, session: &Session) -> Result<MasterShardData, AuthError> {
        self.load_master_shard_with_key(session.encryption_key())
    }

    /// Load master shard with explicit encryption key
    ///
    /// Used during initial verification before session is created
    pub fn load_master_shard_with_key(
        &self,
        encryption_key: &[u8; 32],
    ) -> Result<MasterShardData, AuthError> {
        let path = self.encrypted_shard_path();
        if !path.exists() {
            return Err(AuthError::StorageError("Master shard not found".to_string()));
        }

        // Read encrypted data
        let encrypted_data = std::fs::read(&path)?;

        if encrypted_data.len() < NONCE_SIZE {
            return Err(AuthError::StorageError("Encrypted file too short".to_string()));
        }

        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&encrypted_data[..NONCE_SIZE]);
        let ciphertext = &encrypted_data[NONCE_SIZE..];

        // Decrypt
        let cipher = ChaCha20Poly1305::new_from_slice(encryption_key)
            .map_err(|e| AuthError::CryptoError(format!("Invalid key: {}", e)))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| AuthError::DecryptionFailed)?;

        // Deserialize
        let data: MasterShardData = serde_json::from_slice(&plaintext)
            .map_err(|e| AuthError::StorageError(format!("Deserialization failed: {}", e)))?;

        Ok(data)
    }

    /// Load child registry (not encrypted, but still requires session)
    pub fn load_registry(&self, _session: &Session) -> Result<ChildRegistry, AuthError> {
        let path = self.registry_path();
        if !path.exists() {
            return Ok(ChildRegistry::new());
        }

        let content = std::fs::read_to_string(&path)?;
        let registry: ChildRegistry = serde_json::from_str(&content)
            .map_err(|e| AuthError::StorageError(format!("Registry parse failed: {}", e)))?;

        Ok(registry)
    }

    /// Save child registry
    pub fn save_registry(
        &self,
        registry: &ChildRegistry,
        _session: &Session,
    ) -> Result<(), AuthError> {
        let path = self.registry_path();
        let content = serde_json::to_string_pretty(registry)
            .map_err(|e| AuthError::StorageError(format!("Serialization failed: {}", e)))?;

        let temp_path = path.with_extension("json.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &path)?;

        Ok(())
    }

    /// Update master shard (load, modify, save)
    pub fn update_master_shard<F>(
        &self,
        session: &Session,
        f: F,
    ) -> Result<MasterShardData, AuthError>
    where
        F: FnOnce(&mut MasterShardData),
    {
        let mut data = self.load_master_shard(session)?;
        f(&mut data);
        self.save_master_shard(&data, session.encryption_key())?;
        Ok(data)
    }

    /// Save reconciliation log entry
    pub fn save_reconciliation_log(
        &self,
        child_id: &str,
        log_entry: &str,
        _session: &Session,
    ) -> Result<(), AuthError> {
        let log_dir = self.base_path.join("reconciliation_logs");
        std::fs::create_dir_all(&log_dir)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_path = log_dir.join(format!("{}_{}.log", child_id, timestamp));
        std::fs::write(&log_path, log_entry)?;

        Ok(())
    }

    /// Get path to encrypted master shard file
    fn encrypted_shard_path(&self) -> PathBuf {
        self.base_path.join("master_shard.enc")
    }

    /// Get path to registry file
    fn registry_path(&self) -> PathBuf {
        self.base_path.join("child_registry.json")
    }

    /// Get the base path
    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Migrate unencrypted storage to encrypted (one-time operation during setup)
    pub fn migrate_from_unencrypted(
        &self,
        unencrypted_path: &PathBuf,
        encryption_key: &[u8; 32],
    ) -> Result<(), AuthError> {
        // Read unencrypted data
        let content = std::fs::read_to_string(unencrypted_path)?;
        let data: MasterShardData = serde_json::from_str(&content)
            .map_err(|e| AuthError::StorageError(format!("Parse failed: {}", e)))?;

        // Save encrypted
        self.save_master_shard(&data, encryption_key)?;

        // Remove unencrypted file (securely if possible)
        std::fs::remove_file(unencrypted_path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_key() -> [u8; 32] {
        [42u8; 32]
    }

    fn test_shard_data() -> MasterShardData {
        MasterShardData::new([1u8; 32], [2u8; 33])
    }

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let temp_dir = tempdir().unwrap();
        let storage = EncryptedMotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let key = test_key();
        let original = test_shard_data();

        // Save encrypted
        storage.save_master_shard(&original, &key).unwrap();
        assert!(storage.has_master_shard());

        // Load and verify
        let loaded = storage.load_master_shard_with_key(&key).unwrap();
        assert_eq!(loaded.cold_master_shard, original.cold_master_shard);
        assert_eq!(loaded.master_pubkey, original.master_pubkey);
    }

    #[test]
    fn test_wrong_key_fails() {
        let temp_dir = tempdir().unwrap();
        let storage = EncryptedMotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let key = test_key();
        let wrong_key = [99u8; 32];
        let original = test_shard_data();

        storage.save_master_shard(&original, &key).unwrap();

        // Wrong key should fail with DecryptionFailed
        let result = storage.load_master_shard_with_key(&wrong_key);
        assert!(matches!(result, Err(AuthError::DecryptionFailed)));
    }

    #[test]
    fn test_registry_operations() {
        let temp_dir = tempdir().unwrap();
        let storage = EncryptedMotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

        let key = test_key();
        let config = super::super::SessionConfig::default();
        let session = Session::new(key, config);

        // Initially empty
        let registry = storage.load_registry(&session).unwrap();
        assert!(registry.children.is_empty());
    }
}
