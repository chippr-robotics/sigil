//! Cryptographic utilities for keyshard operations
//!
//! This module provides encryption, decryption, and signing utilities
//! for secure keyshard management.

use crate::error::{Result, SigilError};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Size of encryption key in bytes (256 bits)
pub const KEY_SIZE: usize = 32;

/// Size of nonce in bytes (96 bits for AES-GCM)
pub const NONCE_SIZE: usize = 12;

/// Encrypt data using AES-256-GCM
pub fn encrypt(data: &[u8], key: &[u8; KEY_SIZE]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| SigilError::Crypto(format!("Encryption failed: {}", e)))?;

    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data using AES-256-GCM
pub fn decrypt(encrypted_data: &[u8], key: &[u8; KEY_SIZE]) -> Result<Vec<u8>> {
    if encrypted_data.len() < NONCE_SIZE {
        return Err(SigilError::Crypto(
            "Encrypted data too short".to_string()
        ));
    }

    let cipher = Aes256Gcm::new(key.into());

    // Extract nonce
    let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| SigilError::Crypto(format!("Decryption failed: {}", e)))?;

    Ok(plaintext)
}

/// Generate a random encryption key
pub fn generate_key() -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    OsRng.fill_bytes(&mut key);
    key
}

/// Derive a key from a password using SHA-256 (simplified KDF)
/// Note: In production, use a proper KDF like PBKDF2 or Argon2
pub fn derive_key_from_password(password: &str, salt: &[u8]) -> [u8; KEY_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    
    let hash = hasher.finalize();
    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&hash);
    key
}

/// Generate a cryptographic hash of data
pub fn hash_data(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Simple signature generation (for demonstration)
/// Note: In production, use proper signature algorithms like ECDSA
pub fn sign_data(data: &[u8], key: &[u8; KEY_SIZE]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.update(key);
    hex::encode(hasher.finalize())
}

/// Verify a signature (for demonstration)
pub fn verify_signature(data: &[u8], signature: &str, key: &[u8; KEY_SIZE]) -> bool {
    let expected = sign_data(data, key);
    expected == signature
}

/// Split data into n shares (simplified secret sharing)
/// Note: This is a simplified implementation. Production code should use
/// proper Shamir's Secret Sharing or similar algorithms.
pub fn split_secret(secret: &[u8], n: usize, threshold: usize) -> Result<Vec<Vec<u8>>> {
    if threshold > n {
        return Err(SigilError::Crypto(
            "Threshold cannot be greater than number of shares".to_string()
        ));
    }

    if threshold == 0 || n == 0 {
        return Err(SigilError::Crypto(
            "Threshold and n must be positive".to_string()
        ));
    }

    // Simplified implementation: XOR-based splitting for demonstration
    // This is NOT cryptographically secure for production use
    let mut shares = Vec::new();
    let mut rng = rand::thread_rng();

    // Generate n-1 random shares
    for _ in 0..(n - 1) {
        let mut share = vec![0u8; secret.len()];
        rng.fill_bytes(&mut share);
        shares.push(share);
    }

    // Calculate last share to ensure XOR of all shares equals secret
    let mut last_share = secret.to_vec();
    for share in &shares {
        for (i, byte) in share.iter().enumerate() {
            last_share[i] ^= byte;
        }
    }
    shares.push(last_share);

    Ok(shares)
}

/// Reconstruct secret from shares (simplified)
pub fn reconstruct_secret(shares: &[Vec<u8>]) -> Result<Vec<u8>> {
    if shares.is_empty() {
        return Err(SigilError::Crypto(
            "No shares provided".to_string()
        ));
    }

    let len = shares[0].len();
    let mut secret = vec![0u8; len];

    // XOR all shares together
    for share in shares {
        if share.len() != len {
            return Err(SigilError::Crypto(
                "Share length mismatch".to_string()
            ));
        }
        for (i, byte) in share.iter().enumerate() {
            secret[i] ^= byte;
        }
    }

    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let key = generate_key();
        let data = b"Hello, World!";

        let encrypted = encrypt(data, &key).unwrap();
        assert_ne!(encrypted.as_slice(), data);

        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted.as_slice(), data);
    }

    #[test]
    fn test_key_generation() {
        let key1 = generate_key();
        let key2 = generate_key();
        
        // Keys should be random and different
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_derivation() {
        let password = "test_password";
        let salt = b"salt123";
        
        let key1 = derive_key_from_password(password, salt);
        let key2 = derive_key_from_password(password, salt);
        
        // Same password and salt should produce same key
        assert_eq!(key1, key2);

        let key3 = derive_key_from_password("different", salt);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_hash_data() {
        let data = b"test data";
        let hash1 = hash_data(data);
        let hash2 = hash_data(data);
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_signing() {
        let key = generate_key();
        let data = b"message to sign";
        
        let signature = sign_data(data, &key);
        assert!(verify_signature(data, &signature, &key));
        
        // Wrong data should not verify
        assert!(!verify_signature(b"wrong data", &signature, &key));
    }

    #[test]
    fn test_secret_splitting_reconstruction() {
        let secret = b"my secret data";
        let n = 5;
        let threshold = 3;

        let shares = split_secret(secret, n, threshold).unwrap();
        assert_eq!(shares.len(), n);

        // Reconstruct with all shares
        let reconstructed = reconstruct_secret(&shares).unwrap();
        assert_eq!(reconstructed.as_slice(), secret);

        // Reconstruct with subset of shares
        let subset = &shares[0..threshold];
        let reconstructed2 = reconstruct_secret(subset).unwrap();
        assert_eq!(reconstructed2.as_slice(), secret);
    }

    #[test]
    fn test_invalid_secret_splitting() {
        let secret = b"test";
        
        // Threshold > n should fail
        let result = split_secret(secret, 3, 5);
        assert!(result.is_err());

        // Zero values should fail
        let result = split_secret(secret, 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_decryption_with_wrong_key() {
        let key1 = generate_key();
        let key2 = generate_key();
        let data = b"secret message";

        let encrypted = encrypt(data, &key1).unwrap();
        let result = decrypt(&encrypted, &key2);
        
        assert!(result.is_err());
    }
}
