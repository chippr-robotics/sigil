//! Cryptographic primitives for Sigil

use k256::{
    ecdsa::{signature::Verifier, Signature as K256Signature, VerifyingKey},
    elliptic_curve::sec1::ToEncodedPoint,
    AffinePoint, ProjectivePoint,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::types::hex_bytes_33;

use crate::error::{Error, Result};
use crate::types::{ChildId, MessageHash, Signature};

/// Compressed public key (33 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(#[serde(with = "hex_bytes_33")] pub [u8; 33]);

impl PublicKey {
    /// Create a new PublicKey from compressed bytes
    pub fn new(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }

    /// Get the compressed bytes
    pub fn as_bytes(&self) -> &[u8; 33] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from hex string
    pub fn from_hex(s: &str) -> Result<Self> {
        let mut bytes = [0u8; 33];
        hex::decode_to_slice(s, &mut bytes).map_err(|e| Error::Crypto(e.to_string()))?;
        Ok(Self(bytes))
    }

    /// Compute the ChildId (SHA256 hash of public key)
    pub fn to_child_id(&self) -> ChildId {
        let mut hasher = Sha256::new();
        hasher.update(self.0);
        let hash = hasher.finalize();
        ChildId::new(hash.into())
    }

    /// Verify a signature against this public key
    pub fn verify(&self, message_hash: &MessageHash, signature: &Signature) -> Result<()> {
        let verifying_key = VerifyingKey::from_sec1_bytes(&self.0)
            .map_err(|e| Error::Crypto(format!("Invalid public key: {}", e)))?;

        let sig = K256Signature::from_slice(signature.as_bytes())
            .map_err(|e| Error::Crypto(format!("Invalid signature format: {}", e)))?;

        verifying_key
            .verify(message_hash.as_bytes(), &sig)
            .map_err(|_| Error::SignatureVerificationFailed)
    }

    /// Convert to k256 AffinePoint
    pub fn to_affine_point(&self) -> Result<AffinePoint> {
        let point = k256::PublicKey::from_sec1_bytes(&self.0)
            .map_err(|e| Error::Crypto(format!("Invalid public key: {}", e)))?;
        Ok(*point.as_affine())
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Zeroize for PublicKey {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

/// HD derivation path (serialized as 32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationPath {
    /// Path components (e.g., [44', 60', 0', i'])
    /// Hardened indices have the high bit set
    pub components: [u32; 5],
    /// Number of valid components
    pub depth: u8,
}

impl DerivationPath {
    /// BIP44 constant for hardened derivation
    pub const HARDENED: u32 = 0x80000000;

    /// Create a new derivation path from components
    pub fn new(components: &[u32]) -> Result<Self> {
        if components.len() > 5 {
            return Err(Error::InvalidDerivationPath(
                "Path too long (max 5 components)".to_string(),
            ));
        }
        let mut path = Self {
            components: [0; 5],
            depth: components.len() as u8,
        };
        path.components[..components.len()].copy_from_slice(components);
        Ok(path)
    }

    /// Create a BIP44 Ethereum path: m/44'/60'/0'/0/i
    pub fn ethereum(child_index: u32) -> Self {
        Self {
            components: [
                44 | Self::HARDENED, // purpose
                60 | Self::HARDENED, // coin type (ETH)
                Self::HARDENED,      // account
                child_index,         // child index (not hardened for derivation)
                0,
            ],
            depth: 4,
        }
    }

    /// Create a hardened BIP44 path: m/44'/60'/0'/i'
    pub fn ethereum_hardened(child_index: u32) -> Self {
        Self {
            components: [
                44 | Self::HARDENED,
                60 | Self::HARDENED,
                Self::HARDENED,
                child_index | Self::HARDENED,
                0,
            ],
            depth: 4,
        }
    }

    /// Serialize to bytes for disk storage
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0] = self.depth;
        for (i, component) in self.components.iter().enumerate() {
            let offset = 1 + i * 4;
            if offset + 4 <= 32 {
                bytes[offset..offset + 4].copy_from_slice(&component.to_le_bytes());
            }
        }
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let depth = bytes[0];
        if depth > 5 {
            return Err(Error::InvalidDerivationPath(format!(
                "Invalid depth: {}",
                depth
            )));
        }
        let mut components = [0u32; 5];
        for (i, component) in components.iter_mut().enumerate().take(depth as usize) {
            let offset = 1 + i * 4;
            *component = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        }
        Ok(Self { components, depth })
    }

    /// Get path as a string (e.g., "m/44'/60'/0'/0")
    pub fn to_string_path(&self) -> String {
        let mut path = String::from("m");
        for i in 0..self.depth as usize {
            let component = self.components[i];
            if component & Self::HARDENED != 0 {
                path.push_str(&format!("/{}'", component & !Self::HARDENED));
            } else {
                path.push_str(&format!("/{}", component));
            }
        }
        path
    }
}

impl Zeroize for DerivationPath {
    fn zeroize(&mut self) {
        self.components.zeroize();
        self.depth.zeroize();
    }
}

/// Child key pair (cold shard + public key)
/// Note: The full key is split between cold and agent shards
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct ChildKeyPair {
    /// Cold shard of the private key (32 bytes)
    #[zeroize(skip)]
    pub cold_shard: [u8; 32],
    /// Combined public key (compressed, 33 bytes)
    pub public_key: PublicKey,
    /// Derivation path used
    pub derivation_path: DerivationPath,
}

impl ChildKeyPair {
    /// Create a new child key pair
    pub fn new(
        cold_shard: [u8; 32],
        public_key: PublicKey,
        derivation_path: DerivationPath,
    ) -> Self {
        Self {
            cold_shard,
            public_key,
            derivation_path,
        }
    }

    /// Get the child ID
    pub fn child_id(&self) -> ChildId {
        self.public_key.to_child_id()
    }
}

/// Combine two public key points (for computing child_pubkey from shards)
pub fn point_add(pk1: &PublicKey, pk2: &PublicKey) -> Result<PublicKey> {
    let point1 = pk1.to_affine_point()?;
    let point2 = pk2.to_affine_point()?;

    let sum = ProjectivePoint::from(point1) + ProjectivePoint::from(point2);
    let affine = sum.to_affine();
    let encoded = affine.to_encoded_point(true);
    let bytes: [u8; 33] = encoded
        .as_bytes()
        .try_into()
        .map_err(|_| Error::Crypto("Failed to encode combined public key".to_string()))?;

    Ok(PublicKey::new(bytes))
}

/// Hash data using SHA256
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Hash multiple pieces of data using SHA256
pub fn sha256_multi(data: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for d in data {
        hasher.update(d);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derivation_path_roundtrip() {
        let path = DerivationPath::ethereum_hardened(42);
        let bytes = path.to_bytes();
        let recovered = DerivationPath::from_bytes(&bytes).unwrap();
        assert_eq!(path, recovered);
    }

    #[test]
    fn test_derivation_path_string() {
        let path = DerivationPath::ethereum_hardened(0);
        assert_eq!(path.to_string_path(), "m/44'/60'/0'/0'");
    }

    #[test]
    fn test_child_id_short() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0x7a;
        bytes[1] = 0x3f;
        bytes[2] = 0xbc;
        bytes[3] = 0x12;
        let id = ChildId::new(bytes);
        // The short form should be the first 4 bytes
        assert!(id.short().starts_with("7a3f"));
    }
}
