//! Agent shard encryption for secure QR code display
//!
//! When generating child disks, the agent shard (containing the agent's presig shares)
//! must be securely transferred to the agent. This module provides encryption using:
//!
//! - **ChaCha20-Poly1305** for authenticated encryption
//! - **Argon2id** for key derivation from a passcode
//!
//! The workflow:
//! 1. Generate a random 24-character passcode (high entropy)
//! 2. Derive encryption key from passcode using Argon2id
//! 3. Encrypt the shard data with ChaCha20-Poly1305
//! 4. Display QR code containing encrypted data
//! 5. Display passcode separately (optionally on different screen/printout)
//!
//! QR format: `SIGIL:ESHARD:1:<base64-encrypted-package>`

use argon2::Argon2;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{MotherError, Result};

/// Encrypted shard format version
pub const ENCRYPTED_SHARD_VERSION: u8 = 1;

/// Prefix for encrypted shard QR codes
pub const ENCRYPTED_SHARD_PREFIX: &str = "SIGIL:ESHARD:1:";

/// Length of generated passcode in characters
pub const PASSCODE_LENGTH: usize = 24;

/// Characters used in passcode generation (alphanumeric, no ambiguous chars)
const PASSCODE_CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Argon2id parameters for key derivation
/// These are tuned for air-gapped device with limited resources
const ARGON2_MEMORY_KB: u32 = 64 * 1024; // 64 MB
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 1;

/// Encrypted agent shard package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedAgentShard {
    /// Format version
    pub version: u8,

    /// Salt used for key derivation (16 bytes)
    #[serde(with = "base64_bytes")]
    pub salt: [u8; 16],

    /// Nonce for ChaCha20-Poly1305 (12 bytes)
    #[serde(with = "base64_bytes_12")]
    pub nonce: [u8; 12],

    /// Encrypted shard data (includes auth tag)
    #[serde(with = "base64_bytes_vec")]
    pub ciphertext: Vec<u8>,

    /// Child ID this shard belongs to (for verification)
    pub child_id_short: String,

    /// Number of presigs in this shard
    pub presig_count: u32,
}

/// Agent shard data before encryption
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct AgentShardData {
    /// Child ID (hex)
    pub child_id: String,

    /// The presignature agent shares
    #[zeroize(skip)]
    pub presig_shares: Vec<sigil_core::presig::PresigAgentShare>,

    /// Creation timestamp
    pub created_at: u64,

    /// Derivation path used
    pub derivation_path: String,
}

/// Generated passcode (zeroized on drop)
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Passcode(String);

impl Passcode {
    /// Get the passcode string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Format passcode for display (groups of 4 characters)
    pub fn display_formatted(&self) -> String {
        self.0
            .chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("-")
    }
}

impl std::fmt::Debug for Passcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Passcode([REDACTED])")
    }
}

/// Encrypt agent shard data for QR code display
///
/// Returns the encrypted package and the passcode to display separately.
pub fn encrypt_agent_shard(shard_data: &AgentShardData) -> Result<(EncryptedAgentShard, Passcode)> {
    let mut rng = rand::thread_rng();

    // Generate random passcode
    let passcode = generate_passcode(&mut rng);

    // Generate salt for key derivation
    let mut salt = [0u8; 16];
    rng.fill_bytes(&mut salt);

    // Derive encryption key using Argon2id
    let key = derive_key(passcode.as_str(), &salt)?;

    // Generate nonce for ChaCha20-Poly1305
    let mut nonce_bytes = [0u8; 12];
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = &Nonce::from(nonce_bytes);

    // Serialize shard data
    let plaintext =
        serde_json::to_vec(shard_data).map_err(|e| MotherError::Serialization(e.to_string()))?;

    // Encrypt with ChaCha20-Poly1305
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| MotherError::Crypto(format!("Failed to create cipher: {}", e)))?;

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_slice())
        .map_err(|e| MotherError::Crypto(format!("Encryption failed: {}", e)))?;

    let encrypted = EncryptedAgentShard {
        version: ENCRYPTED_SHARD_VERSION,
        salt,
        nonce: nonce_bytes,
        ciphertext,
        child_id_short: shard_data.child_id.chars().take(8).collect(),
        presig_count: shard_data.presig_shares.len() as u32,
    };

    Ok((encrypted, passcode))
}

/// Decrypt agent shard data using the passcode
pub fn decrypt_agent_shard(
    encrypted: &EncryptedAgentShard,
    passcode: &str,
) -> Result<AgentShardData> {
    // Derive key from passcode
    let key = derive_key(passcode, &encrypted.salt)?;

    // Create cipher and nonce
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| MotherError::Crypto(format!("Failed to create cipher: {}", e)))?;
    let nonce = &Nonce::from(encrypted.nonce);

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, encrypted.ciphertext.as_slice())
        .map_err(|_| MotherError::Crypto("Decryption failed - invalid passcode".to_string()))?;

    // Deserialize
    let shard_data: AgentShardData = serde_json::from_slice(&plaintext)
        .map_err(|e| MotherError::Serialization(e.to_string()))?;

    Ok(shard_data)
}

/// Encode encrypted shard to QR-ready string
pub fn encode_for_qr(encrypted: &EncryptedAgentShard) -> Result<String> {
    let json =
        serde_json::to_vec(encrypted).map_err(|e| MotherError::Serialization(e.to_string()))?;
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &json);
    Ok(format!("{}{}", ENCRYPTED_SHARD_PREFIX, encoded))
}

/// Decode encrypted shard from QR string
pub fn decode_from_qr(qr_data: &str) -> Result<EncryptedAgentShard> {
    let data = qr_data
        .strip_prefix(ENCRYPTED_SHARD_PREFIX)
        .ok_or_else(|| {
            MotherError::InvalidDiskFormat("Invalid encrypted shard prefix".to_string())
        })?;

    let json = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
        .map_err(|e| MotherError::Serialization(format!("Base64 decode failed: {}", e)))?;

    let encrypted: EncryptedAgentShard =
        serde_json::from_slice(&json).map_err(|e| MotherError::Serialization(e.to_string()))?;

    Ok(encrypted)
}

/// Generate a random passcode
fn generate_passcode<R: Rng>(rng: &mut R) -> Passcode {
    let passcode: String = (0..PASSCODE_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..PASSCODE_CHARS.len());
            PASSCODE_CHARS[idx] as char
        })
        .collect();
    Passcode(passcode)
}

/// Derive encryption key from passcode using Argon2id
fn derive_key(passcode: &str, salt: &[u8; 16]) -> Result<[u8; 32]> {
    // Create Argon2id hasher with custom parameters
    let params = argon2::Params::new(
        ARGON2_MEMORY_KB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| MotherError::Crypto(format!("Invalid Argon2 params: {}", e)))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    // Derive key
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passcode.as_bytes(), salt, &mut key)
        .map_err(|e| MotherError::Crypto(format!("Key derivation failed: {}", e)))?;

    Ok(key)
}

/// Estimate the QR code size needed for an encrypted shard
pub fn estimate_qr_size(presig_count: u32) -> usize {
    // Each presig share is ~100 bytes, encryption adds ~16 bytes overhead
    // Base64 encoding adds ~33% overhead
    // Prefix adds ~15 bytes
    (presig_count as usize * 110 + 100) * 4 / 3 + 50
}

// =============================================================================
// Serde helpers for byte arrays
// =============================================================================

mod base64_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 16], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 16], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("Invalid length"))
    }
}

mod base64_bytes_12 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 12], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 12], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("Invalid length"))
    }
}

mod base64_bytes_vec {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_shard_data() -> AgentShardData {
        AgentShardData {
            child_id: "deadbeef".repeat(8),
            presig_shares: vec![sigil_core::presig::PresigAgentShare::new(
                [0x01; 33], [0x02; 32], [0x03; 32],
            )],
            created_at: 1700000000,
            derivation_path: "m/44'/60'/0'/0'".to_string(),
        }
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let shard_data = create_test_shard_data();

        let (encrypted, passcode) = encrypt_agent_shard(&shard_data).unwrap();

        let decrypted = decrypt_agent_shard(&encrypted, passcode.as_str()).unwrap();

        assert_eq!(shard_data.child_id, decrypted.child_id);
        assert_eq!(shard_data.created_at, decrypted.created_at);
        assert_eq!(
            shard_data.presig_shares.len(),
            decrypted.presig_shares.len()
        );
    }

    #[test]
    fn test_wrong_passcode_fails() {
        let shard_data = create_test_shard_data();

        let (encrypted, _passcode) = encrypt_agent_shard(&shard_data).unwrap();

        let result = decrypt_agent_shard(&encrypted, "WRONG-PASSCODE-HERE123");
        assert!(result.is_err());
    }

    #[test]
    fn test_qr_encode_decode_roundtrip() {
        let shard_data = create_test_shard_data();

        let (encrypted, _passcode) = encrypt_agent_shard(&shard_data).unwrap();

        let qr_string = encode_for_qr(&encrypted).unwrap();
        assert!(qr_string.starts_with(ENCRYPTED_SHARD_PREFIX));

        let decoded = decode_from_qr(&qr_string).unwrap();
        assert_eq!(encrypted.child_id_short, decoded.child_id_short);
        assert_eq!(encrypted.presig_count, decoded.presig_count);
    }

    #[test]
    fn test_passcode_generation() {
        let mut rng = rand::thread_rng();
        let passcode = generate_passcode(&mut rng);

        assert_eq!(passcode.as_str().len(), PASSCODE_LENGTH);

        // All characters should be alphanumeric
        assert!(passcode.as_str().chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_passcode_formatted_display() {
        let passcode = Passcode("ABCD1234EFGH5678IJKL9012".to_string());
        let formatted = passcode.display_formatted();
        assert_eq!(formatted, "ABCD-1234-EFGH-5678-IJKL-9012");
    }

    #[test]
    fn test_derive_key_deterministic() {
        let salt = [0x42u8; 16];
        let key1 = derive_key("test-passcode", &salt).unwrap();
        let key2 = derive_key("test-passcode", &salt).unwrap();
        assert_eq!(key1, key2);

        // Different passcode gives different key
        let key3 = derive_key("other-passcode", &salt).unwrap();
        assert_ne!(key1, key3);
    }
}
