//! Hardware signer abstraction for secure key operations
//!
//! This module provides a unified interface for different hardware signing devices:
//! - Ledger hardware wallets (Nano S, S Plus, X)
//! - Trezor hardware wallets (Model One, Model T, Safe 3/5/7)
//! - PKCS#11 compatible HSMs (YubiHSM, SoftHSM, NetHSM, etc.)
//!
//! # Security Model
//!
//! All implementations derive master shards deterministically from device signatures,
//! enabling recovery from the device's seed/key material.

#[cfg(feature = "ledger")]
pub mod ledger;

#[cfg(feature = "trezor")]
pub mod trezor;

#[cfg(feature = "pkcs11")]
pub mod pkcs11;

use crate::error::Result;
use async_trait::async_trait;

/// Fixed derivation messages for deterministic recovery
/// These messages MUST NOT change to ensure recoverability across versions
pub const COLD_SHARD_MESSAGE: &str = "Sigil MPC Cold Master Shard Derivation v1";
pub const AGENT_SHARD_MESSAGE: &str = "Sigil MPC Agent Master Shard Derivation v1";

/// Information about a connected hardware signer
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device type/model name
    pub model: String,
    /// Whether the device is ready for signing
    pub ready: bool,
    /// Public key from the signing key (if available)
    pub public_key: Option<[u8; 65]>,
    /// Device-specific address (e.g., Ethereum address for Ledger)
    pub address: Option<String>,
    /// Additional device-specific info
    pub extra: Option<String>,
}

/// Output from hardware-based master key generation
#[derive(Debug)]
pub struct HardwareMasterKeyOutput {
    /// Cold master shard (derived from device signature on cold message)
    pub cold_master_shard: [u8; 32],
    /// Agent master shard (derived from device signature on agent message)
    pub agent_master_shard: [u8; 32],
    /// Combined master public key
    pub master_pubkey: sigil_core::PublicKey,
    /// Device's public key (for verification/recovery)
    pub device_pubkey: [u8; 65],
}

/// Trait for hardware-backed signing devices
///
/// All implementations must provide deterministic signing, meaning the same
/// message signed with the same key will always produce the same signature.
/// This enables recovery of derived shards from the device's seed.
#[async_trait]
pub trait HardwareSigner: Send + Sync {
    /// Get device information and verify readiness
    async fn get_info(&self) -> Result<DeviceInfo>;

    /// Get public key at the specified derivation path
    ///
    /// # Arguments
    /// * `path` - BIP32 derivation path (e.g., "m/44'/60'/0'/0/0")
    async fn get_public_key(&self, path: &str) -> Result<([u8; 65], String)>;

    /// Sign a message deterministically
    ///
    /// The signature MUST be deterministic (RFC6979 or equivalent) to enable
    /// recovery of derived keys.
    ///
    /// # Arguments
    /// * `path` - BIP32 derivation path
    /// * `message` - Message bytes to sign
    async fn sign_message(&self, path: &str, message: &[u8]) -> Result<[u8; 65]>;

    /// Generate master key using the hardware device as entropy source
    ///
    /// This signs two fixed messages to derive both shards deterministically:
    /// 1. Cold shard from COLD_SHARD_MESSAGE
    /// 2. Agent shard from AGENT_SHARD_MESSAGE
    ///
    /// Both shards are recoverable from the same device seed.
    async fn generate_master_key(&self, path: &str) -> Result<HardwareMasterKeyOutput> {

        // Get device's public key
        let (device_pubkey, _address) = self.get_public_key(path).await?;

        // Sign for cold shard derivation
        let cold_signature = self.sign_message(path, COLD_SHARD_MESSAGE.as_bytes()).await?;

        // Sign for agent shard derivation
        let agent_signature = self.sign_message(path, AGENT_SHARD_MESSAGE.as_bytes()).await?;

        // Derive both shards deterministically from signatures
        let cold_master_shard = derive_shard_from_signature(&cold_signature, b"cold_master_shard");
        let agent_master_shard =
            derive_shard_from_signature(&agent_signature, b"agent_master_shard");

        // Derive public keys and combine
        let cold_pubkey = derive_public_key(&cold_master_shard)?;
        let agent_pubkey = derive_public_key(&agent_master_shard)?;
        let master_pubkey = combine_public_keys(&cold_pubkey, &agent_pubkey)?;

        Ok(HardwareMasterKeyOutput {
            cold_master_shard,
            agent_master_shard,
            master_pubkey,
            device_pubkey,
        })
    }

    /// Get the device type name
    fn device_type(&self) -> &'static str;
}

/// Derive a 32-byte shard from a signature using domain separation
fn derive_shard_from_signature(signature: &[u8; 65], domain: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(signature);
    hasher.finalize().into()
}

/// Derive a public key from a 32-byte secret
fn derive_public_key(secret: &[u8; 32]) -> Result<[u8; 33]> {
    use crate::error::MotherError;
    use k256::elliptic_curve::sec1::ToEncodedPoint;
    use k256::SecretKey;

    let secret_key = SecretKey::from_bytes(secret.into())
        .map_err(|e| MotherError::Crypto(format!("Invalid secret key: {}", e)))?;

    let public_key = secret_key.public_key();
    let encoded = public_key.to_encoded_point(true);

    let mut result = [0u8; 33];
    result.copy_from_slice(encoded.as_bytes());
    Ok(result)
}

/// Combine two public keys (point addition)
fn combine_public_keys(pk1: &[u8; 33], pk2: &[u8; 33]) -> Result<sigil_core::PublicKey> {
    use crate::error::MotherError;
    use k256::elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint};
    use k256::{AffinePoint, EncodedPoint, ProjectivePoint};

    let point1 = EncodedPoint::from_bytes(pk1)
        .map_err(|e| MotherError::Crypto(format!("Invalid public key 1: {}", e)))?;
    let point2 = EncodedPoint::from_bytes(pk2)
        .map_err(|e| MotherError::Crypto(format!("Invalid public key 2: {}", e)))?;

    let affine1 = AffinePoint::from_encoded_point(&point1);
    let affine2 = AffinePoint::from_encoded_point(&point2);

    if affine1.is_none().into() || affine2.is_none().into() {
        return Err(MotherError::Crypto("Invalid curve point".to_string()));
    }

    let proj1 = ProjectivePoint::from(affine1.unwrap());
    let proj2 = ProjectivePoint::from(affine2.unwrap());

    let combined = proj1 + proj2;
    let combined_affine = AffinePoint::from(combined);
    let encoded = combined_affine.to_encoded_point(true);

    let mut result = [0u8; 33];
    result.copy_from_slice(encoded.as_bytes());
    Ok(sigil_core::PublicKey::new(result))
}

/// Encode a BIP32 path string to bytes
/// e.g., "m/44'/60'/0'/0/0" -> [0x8000002C, 0x8000003C, 0x80000000, 0x00000000, 0x00000000]
pub fn encode_bip32_path(path: &str) -> Result<Vec<u8>> {
    use crate::error::MotherError;

    let parts: Vec<&str> = path.split('/').collect();
    let mut result = Vec::new();

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            if *part != "m" {
                return Err(MotherError::Crypto("Path must start with 'm'".to_string()));
            }
            continue;
        }

        let (num_str, hardened) = if part.ends_with('\'') || part.ends_with('h') {
            (&part[..part.len() - 1], true)
        } else {
            (*part, false)
        };

        let num: u32 = num_str
            .parse()
            .map_err(|_| MotherError::Crypto(format!("Invalid path component: {}", part)))?;

        let component = if hardened { num | 0x80000000 } else { num };

        result.extend_from_slice(&component.to_be_bytes());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_bip32_path() {
        let path = "m/44'/60'/0'/0/0";
        let encoded = encode_bip32_path(path).unwrap();

        // Should have 5 components * 4 bytes each = 20 bytes
        assert_eq!(encoded.len(), 20);

        // First component: 44' = 44 + 0x80000000 = 0x8000002C
        assert_eq!(&encoded[0..4], &[0x80, 0x00, 0x00, 0x2C]);

        // Second component: 60' = 60 + 0x80000000 = 0x8000003C
        assert_eq!(&encoded[4..8], &[0x80, 0x00, 0x00, 0x3C]);
    }

    #[test]
    fn test_derive_shard_deterministic() {
        let sig = [0x42u8; 65];
        let shard1 = derive_shard_from_signature(&sig, b"test");
        let shard2 = derive_shard_from_signature(&sig, b"test");
        assert_eq!(shard1, shard2);

        // Different domain = different shard
        let shard3 = derive_shard_from_signature(&sig, b"other");
        assert_ne!(shard1, shard3);
    }
}
