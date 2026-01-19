//! Ledger hardware wallet integration for secure key operations
//!
//! This module provides integration with Ledger Nano S/X devices for
//! secure master key generation and child key derivation.
//!
//! # Security Model
//!
//! The Ledger device is used to:
//! 1. Generate entropy from hardware TRNG for master seed
//! 2. Derive child keys using BIP32 paths
//! 3. Sign messages for verification (optional)
//!
//! Private keys never leave the Ledger secure element. For MPC operations,
//! we derive a deterministic seed from a Ledger signature that can be used
//! to generate the cold shard.

use crate::error::{MotherError, Result};

#[cfg(feature = "ledger")]
use sha2::{Digest, Sha256};
#[cfg(feature = "ledger")]
use tracing::{debug, info, warn};

#[cfg(feature = "ledger")]
use ledger_transport_hid::TransportNativeHID;

/// Ethereum app CLA (instruction class)
#[cfg(feature = "ledger")]
const ETH_CLA: u8 = 0xE0;

/// Ethereum app instructions
#[cfg(feature = "ledger")]
const ETH_INS_GET_PUBLIC_KEY: u8 = 0x02;
#[cfg(feature = "ledger")]
const ETH_INS_SIGN_PERSONAL_MESSAGE: u8 = 0x08;

/// BIP32 derivation path for Sigil mother device
/// Using m/44'/60'/0'/0/0 (Ethereum standard) as base
/// We'll derive Sigil-specific keys from signatures on known messages
pub const SIGIL_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

/// Ledger device connection
#[cfg(feature = "ledger")]
pub struct LedgerDevice {
    transport: TransportNativeHID,
}

/// Placeholder when ledger feature is disabled
#[cfg(not(feature = "ledger"))]
pub struct LedgerDevice {
    _private: (),
}

/// Information about a connected Ledger device
#[derive(Debug, Clone)]
pub struct LedgerInfo {
    /// Device model (if available)
    pub model: String,
    /// Whether Ethereum app is open
    pub eth_app_open: bool,
    /// Public key from derivation path
    pub public_key: Option<[u8; 65]>,
    /// Ethereum address
    pub address: Option<String>,
}

/// Output from Ledger-based master key generation
///
/// Both shards are deterministically derived from Ledger signatures,
/// allowing full recovery from the same Ledger seed.
#[derive(Debug)]
pub struct LedgerMasterKeyOutput {
    /// Cold master shard (derived from Ledger signature on cold message)
    pub cold_master_shard: [u8; 32],
    /// Agent master shard (derived from Ledger signature on agent message)
    pub agent_master_shard: [u8; 32],
    /// Combined master public key
    pub master_pubkey: sigil_core::PublicKey,
    /// Ledger's public key (for verification)
    pub ledger_pubkey: [u8; 65],
}

#[cfg(feature = "ledger")]
impl LedgerDevice {
    /// Connect to a Ledger device
    pub fn connect() -> Result<Self> {
        info!("Searching for Ledger device...");

        let hidapi = ledger_transport_hid::hidapi::HidApi::new()
            .map_err(|e| MotherError::Crypto(format!("Failed to initialize HID: {}", e)))?;

        let transport = TransportNativeHID::new(&hidapi)
            .map_err(|e| MotherError::Crypto(format!("Failed to connect to Ledger: {}", e)))?;

        info!("Ledger device connected");
        Ok(Self { transport })
    }

    /// Get device information and verify Ethereum app is running
    pub async fn get_info(&self) -> Result<LedgerInfo> {
        // Try to get public key to verify Ethereum app is open
        match self.get_public_key(SIGIL_DERIVATION_PATH).await {
            Ok((pubkey, address)) => Ok(LedgerInfo {
                model: "Ledger Nano".to_string(),
                eth_app_open: true,
                public_key: Some(pubkey),
                address: Some(address),
            }),
            Err(_) => {
                warn!("Ethereum app may not be open on Ledger");
                Ok(LedgerInfo {
                    model: "Ledger Nano".to_string(),
                    eth_app_open: false,
                    public_key: None,
                    address: None,
                })
            }
        }
    }

    /// Get public key from Ledger at specified BIP32 path
    pub async fn get_public_key(&self, path: &str) -> Result<([u8; 65], String)> {
        let path_bytes = encode_bip32_path(path)?;

        // Build APDU: CLA INS P1 P2 Lc Data
        // P1=0x00: return address, P2=0x00: no chain code
        let mut data = vec![path_bytes.len() as u8 / 4]; // Number of path components
        data.extend_from_slice(&path_bytes);

        let command = ledger_apdu::APDUCommand {
            cla: ETH_CLA,
            ins: ETH_INS_GET_PUBLIC_KEY,
            p1: 0x00,
            p2: 0x00,
            data,
        };

        let response = self
            .transport
            .exchange(&command)
            .map_err(|e| MotherError::Crypto(format!("Ledger communication error: {}", e)))?;

        if response.retcode() != 0x9000 {
            return Err(MotherError::Crypto(format!(
                "Ledger returned error: 0x{:04X}",
                response.retcode()
            )));
        }

        let data = response.data();
        if data.len() < 67 {
            return Err(MotherError::Crypto("Invalid response length".to_string()));
        }

        // Response format: pubkey_len (1) + pubkey (65) + address_len (1) + address (40)
        let pubkey_len = data[0] as usize;
        if pubkey_len != 65 {
            return Err(MotherError::Crypto("Invalid public key length".to_string()));
        }

        let mut pubkey = [0u8; 65];
        pubkey.copy_from_slice(&data[1..66]);

        let addr_len = data[66] as usize;
        let address = String::from_utf8(data[67..67 + addr_len].to_vec())
            .map_err(|_| MotherError::Crypto("Invalid address encoding".to_string()))?;

        debug!("Got public key from Ledger, address: 0x{}", address);
        Ok((pubkey, format!("0x{}", address)))
    }

    /// Sign a personal message using the Ledger
    /// This uses EIP-191 personal_sign format
    pub async fn sign_personal_message(&self, path: &str, message: &[u8]) -> Result<[u8; 65]> {
        let path_bytes = encode_bip32_path(path)?;

        // Build message with path
        let mut data = vec![path_bytes.len() as u8 / 4];
        data.extend_from_slice(&path_bytes);
        data.extend_from_slice(&(message.len() as u32).to_be_bytes());
        data.extend_from_slice(message);

        let command = ledger_apdu::APDUCommand {
            cla: ETH_CLA,
            ins: ETH_INS_SIGN_PERSONAL_MESSAGE,
            p1: 0x00,
            p2: 0x00,
            data,
        };

        info!("Please confirm the signature on your Ledger device...");

        let response = self
            .transport
            .exchange(&command)
            .map_err(|e| MotherError::Crypto(format!("Ledger communication error: {}", e)))?;

        if response.retcode() != 0x9000 {
            return Err(MotherError::Crypto(format!(
                "Ledger returned error: 0x{:04X} (user rejected or app error)",
                response.retcode()
            )));
        }

        let data = response.data();
        if data.len() != 65 {
            return Err(MotherError::Crypto("Invalid signature length".to_string()));
        }

        let mut signature = [0u8; 65];
        signature.copy_from_slice(data);

        debug!("Got signature from Ledger");
        Ok(signature)
    }

    /// Generate master key using Ledger as entropy source
    ///
    /// This works by:
    /// 1. Signing a fixed "cold shard" message to derive the cold master shard
    /// 2. Signing a fixed "agent shard" message to derive the agent master shard
    /// 3. Combining public keys for the master public key
    ///
    /// Both shards are deterministically recoverable from the same Ledger seed,
    /// as long as the same BIP32 path and messages are used.
    pub async fn generate_master_key(&self) -> Result<LedgerMasterKeyOutput> {
        info!("Generating master key using Ledger...");

        // Fixed derivation messages for deterministic recovery
        // These messages MUST NOT change to ensure recoverability
        const COLD_SHARD_MESSAGE: &str = "Sigil MPC Cold Master Shard Derivation v1";
        const AGENT_SHARD_MESSAGE: &str = "Sigil MPC Agent Master Shard Derivation v1";

        // Get Ledger's public key first
        let (ledger_pubkey, _address) = self.get_public_key(SIGIL_DERIVATION_PATH).await?;

        // Sign for cold shard derivation
        info!("Signing cold shard derivation message...");
        let cold_signature = self
            .sign_personal_message(SIGIL_DERIVATION_PATH, COLD_SHARD_MESSAGE.as_bytes())
            .await?;

        // Sign for agent shard derivation
        info!("Signing agent shard derivation message...");
        let agent_signature = self
            .sign_personal_message(SIGIL_DERIVATION_PATH, AGENT_SHARD_MESSAGE.as_bytes())
            .await?;

        // Derive both shards deterministically from signatures
        let cold_master_shard = derive_shard_from_signature(&cold_signature, b"cold_master_shard");
        let agent_master_shard =
            derive_shard_from_signature(&agent_signature, b"agent_master_shard");

        // Derive public keys and combine
        let cold_pubkey = derive_public_key(&cold_master_shard)?;
        let agent_pubkey = derive_public_key(&agent_master_shard)?;
        let master_pubkey = combine_public_keys(&cold_pubkey, &agent_pubkey)?;

        info!("Master key generated successfully using Ledger entropy");

        Ok(LedgerMasterKeyOutput {
            cold_master_shard,
            agent_master_shard,
            master_pubkey,
            ledger_pubkey,
        })
    }
}

#[cfg(not(feature = "ledger"))]
impl LedgerDevice {
    /// Stub when ledger feature is disabled
    pub fn connect() -> Result<Self> {
        Err(MotherError::Crypto(
            "Ledger support not compiled. Rebuild with --features ledger".to_string(),
        ))
    }

    /// Stub when ledger feature is disabled
    pub async fn get_info(&self) -> Result<LedgerInfo> {
        Err(MotherError::Crypto(
            "Ledger support not compiled".to_string(),
        ))
    }

    /// Stub when ledger feature is disabled
    pub async fn get_public_key(&self, _path: &str) -> Result<([u8; 65], String)> {
        Err(MotherError::Crypto(
            "Ledger support not compiled".to_string(),
        ))
    }

    /// Stub when ledger feature is disabled
    pub async fn sign_personal_message(&self, _path: &str, _message: &[u8]) -> Result<[u8; 65]> {
        Err(MotherError::Crypto(
            "Ledger support not compiled".to_string(),
        ))
    }

    /// Stub when ledger feature is disabled
    pub async fn generate_master_key(&self) -> Result<LedgerMasterKeyOutput> {
        Err(MotherError::Crypto(
            "Ledger support not compiled".to_string(),
        ))
    }
}

/// Encode a BIP32 path string to bytes
/// e.g., "m/44'/60'/0'/0/0" -> [0x80000002C, 0x8000003C, 0x80000000, 0x00000000, 0x00000000]
#[cfg(feature = "ledger")]
fn encode_bip32_path(path: &str) -> Result<Vec<u8>> {
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

/// Derive a 32-byte shard from a signature using domain separation
#[cfg(feature = "ledger")]
fn derive_shard_from_signature(signature: &[u8; 65], domain: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(signature);
    hasher.finalize().into()
}

/// Derive a public key from a 32-byte secret
#[cfg(feature = "ledger")]
fn derive_public_key(secret: &[u8; 32]) -> Result<[u8; 33]> {
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
#[cfg(feature = "ledger")]
fn combine_public_keys(pk1: &[u8; 33], pk2: &[u8; 33]) -> Result<sigil_core::PublicKey> {
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

#[cfg(all(test, feature = "ledger"))]
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
