//! PKCS#11 HSM integration
//!
//! Supports any PKCS#11 compatible HSM including:
//! - YubiHSM 2
//! - SoftHSM 2 (for development/testing)
//! - AWS CloudHSM
//! - Thales Luna
//! - Nitrokey NetHSM
//! - And many others
//!
//! # Configuration
//!
//! Requires a PKCS#11 library path and credentials:
//! - `library_path`: Path to the PKCS#11 .so/.dylib file
//! - `slot`: HSM slot number (usually 0)
//! - `pin`: HSM user PIN
//! - `key_label`: Label of the signing key in the HSM
//!
//! # Key Requirements
//!
//! The HSM must contain an ECDSA key on the secp256k1 curve.
//! The key should be marked as:
//! - CKA_SIGN = true (can sign)
//! - CKA_EXTRACTABLE = false (for security)

use crate::error::{MotherError, Result};
use crate::hardware::{DeviceInfo, HardwareSigner};
use async_trait::async_trait;
use cryptoki::context::{CInitializeArgs, Pkcs11};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, ObjectClass, ObjectHandle};
use cryptoki::session::{Session, UserType};
use cryptoki::types::AuthPin;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// PKCS#11 device configuration
#[derive(Clone)]
pub struct Pkcs11Config {
    /// Path to the PKCS#11 library (.so, .dylib, or .dll)
    pub library_path: String,
    /// Slot number (usually 0)
    pub slot: u64,
    /// User PIN for the HSM
    pub pin: String,
    /// Label of the signing key
    pub key_label: String,
    /// Optional model name for device identification (e.g., "YubiHSM 2", "SoftHSM")
    /// If not provided, will attempt to use token info from the slot
    pub model_name: Option<String>,
}

/// PKCS#11 HSM connection
pub struct Pkcs11Device {
    _ctx: Arc<Pkcs11>,
    session: Arc<Mutex<Session>>,
    key_handle: ObjectHandle,
    public_key: [u8; 65],
    config: Pkcs11Config,
    /// Token label from slot info (for device identification)
    token_label: String,
}

impl Pkcs11Device {
    /// Connect to a PKCS#11 HSM
    pub fn connect(config: Pkcs11Config) -> Result<Self> {
        info!("Connecting to PKCS#11 HSM...");

        // Load the PKCS#11 library
        let ctx = Pkcs11::new(Path::new(&config.library_path))
            .map_err(|e| MotherError::Crypto(format!("Failed to load PKCS#11 library: {}", e)))?;

        // Initialize the library
        ctx.initialize(CInitializeArgs::OsThreads)
            .map_err(|e| MotherError::Crypto(format!("Failed to initialize PKCS#11: {}", e)))?;

        let ctx = Arc::new(ctx);

        // Get available slots
        let slots = ctx
            .get_slots_with_token()
            .map_err(|e| MotherError::Crypto(format!("Failed to get slots: {}", e)))?;

        if slots.is_empty() {
            return Err(MotherError::Crypto("No HSM tokens found".to_string()));
        }

        let slot = slots
            .get(config.slot as usize)
            .ok_or_else(|| MotherError::Crypto(format!("Slot {} not found", config.slot)))?;

        // Get token info for device identification
        let token_label = ctx
            .get_token_info(*slot)
            .map(|info| info.label().trim().to_string())
            .unwrap_or_else(|_| "Unknown Token".to_string());

        // Open a session
        let session = ctx
            .open_rw_session(*slot)
            .map_err(|e| MotherError::Crypto(format!("Failed to open session: {}", e)))?;

        // Login with PIN
        let pin = AuthPin::new(config.pin.clone());
        session
            .login(UserType::User, Some(&pin))
            .map_err(|e| MotherError::Crypto(format!("Failed to login: {}", e)))?;

        info!("PKCS#11 session established");

        // Find the signing key by label
        let key_handle = Self::find_key(&session, &config.key_label)?;

        // Get the public key
        let public_key = Self::get_public_key_from_handle(&session, key_handle)?;

        Ok(Self {
            _ctx: ctx,
            session: Arc::new(Mutex::new(session)),
            key_handle,
            public_key,
            config,
            token_label,
        })
    }

    /// Find a key by label
    fn find_key(session: &Session, label: &str) -> Result<ObjectHandle> {
        let template = vec![
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(label.as_bytes().to_vec()),
        ];

        let objects = session
            .find_objects(&template)
            .map_err(|e| MotherError::Crypto(format!("Failed to find key: {}", e)))?;

        objects
            .into_iter()
            .next()
            .ok_or_else(|| MotherError::Crypto(format!("Key '{}' not found in HSM", label)))
    }

    /// Get public key from a private key handle
    ///
    /// Parses the EC point from the HSM's DER-encoded response. Different HSMs
    /// may return EC points in various formats:
    /// - Raw uncompressed point (65 bytes): 04 || x (32 bytes) || y (32 bytes)
    /// - DER OCTET STRING wrapped: 04 || len || 04 || x || y
    /// - Some HSMs may use different DER structures
    ///
    /// This implementation handles the most common formats but may need
    /// extension for specific HSM implementations.
    fn get_public_key_from_handle(session: &Session, key_handle: ObjectHandle) -> Result<[u8; 65]> {
        // Get the EC point (public key) attribute
        let attrs = session
            .get_attributes(key_handle, &[AttributeType::EcPoint])
            .map_err(|e| MotherError::Crypto(format!("Failed to get public key: {}", e)))?;

        for attr in attrs {
            if let Attribute::EcPoint(point) = attr {
                let point_bytes = Self::parse_ec_point(&point)?;
                let mut pubkey = [0u8; 65];
                pubkey.copy_from_slice(point_bytes);
                return Ok(pubkey);
            }
        }

        Err(MotherError::Crypto(
            "Could not retrieve public key from HSM".to_string(),
        ))
    }

    /// Parse EC point from various DER encodings
    ///
    /// Handles multiple formats that HSMs may return:
    /// 1. Raw 65-byte uncompressed point (04 || x || y)
    /// 2. DER OCTET STRING: 04 (tag) || length || point_data
    /// 3. Nested structures where point is at the end
    fn parse_ec_point(data: &[u8]) -> Result<&[u8]> {
        // Case 1: Raw uncompressed point (65 bytes starting with 0x04)
        if data.len() == 65 && data[0] == 0x04 {
            return Ok(data);
        }

        // Case 2: DER OCTET STRING encoding
        // Format: 04 (OCTET STRING tag) || length || actual_point
        if data.len() > 2 && data[0] == 0x04 {
            let length = data[1] as usize;

            // Simple length encoding (length < 128)
            if data[1] < 0x80 && data.len() >= 2 + length {
                let point_start = 2;
                let point_data = &data[point_start..point_start + length];

                // The inner data should be an uncompressed point
                if point_data.len() == 65 && point_data[0] == 0x04 {
                    return Ok(point_data);
                }
            }

            // Long-form length encoding (length >= 128)
            // Format: 04 || 81 || actual_length || point_data
            if data[1] == 0x81 && data.len() > 3 {
                let length = data[2] as usize;
                if data.len() >= 3 + length {
                    let point_data = &data[3..3 + length];
                    if point_data.len() == 65 && point_data[0] == 0x04 {
                        return Ok(point_data);
                    }
                }
            }
        }

        // Case 3: Point at end of larger structure (fallback)
        // Some HSMs return extra metadata; try to find 65-byte sequence at end
        if data.len() > 65 {
            let potential_point = &data[data.len() - 65..];
            if potential_point[0] == 0x04 {
                warn!(
                    "Using fallback EC point extraction from {} byte response",
                    data.len()
                );
                return Ok(potential_point);
            }
        }

        Err(MotherError::Crypto(format!(
            "Unsupported EC point encoding: {} bytes, first byte: 0x{:02x}. \
             Expected uncompressed secp256k1 point (65 bytes starting with 0x04) \
             or DER-encoded OCTET STRING.",
            data.len(),
            data.first().copied().unwrap_or(0)
        )))
    }

    /// Sign data with the HSM key
    fn sign_with_hsm(&self, data: &[u8]) -> Result<[u8; 65]> {
        // Hash the data first (HSM typically expects pre-hashed data for ECDSA)
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(data);

        // Sign using ECDSA
        let mechanism = Mechanism::Ecdsa;

        let session = self
            .session
            .lock()
            .map_err(|e| MotherError::Crypto(format!("Failed to lock session: {}", e)))?;

        let signature = session
            .sign(&mechanism, self.key_handle, &hash)
            .map_err(|e| MotherError::Crypto(format!("HSM signing failed: {}", e)))?;

        // PKCS#11 returns r || s (64 bytes), need to add recovery byte
        // For deterministic recovery, we compute v from the public key
        if signature.len() != 64 {
            return Err(MotherError::Crypto(format!(
                "Unexpected signature length: {}",
                signature.len()
            )));
        }

        let mut sig = [0u8; 65];
        sig[0..64].copy_from_slice(&signature);
        sig[64] = Self::compute_recovery_id(&hash, &signature, &self.public_key)?;

        Ok(sig)
    }

    /// Compute the recovery ID (v) for an ECDSA signature
    fn compute_recovery_id(hash: &[u8], sig_rs: &[u8], pubkey: &[u8; 65]) -> Result<u8> {
        use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

        let signature = Signature::from_slice(sig_rs)
            .map_err(|e| MotherError::Crypto(format!("Invalid signature: {}", e)))?;

        // Try both possible recovery IDs
        for v in [0u8, 1u8] {
            let recovery_id = RecoveryId::from_byte(v).unwrap();
            if let Ok(recovered) = VerifyingKey::recover_from_prehash(hash, &signature, recovery_id)
            {
                let recovered_bytes = recovered.to_encoded_point(false);
                if recovered_bytes.as_bytes() == pubkey {
                    return Ok(v + 27); // Ethereum format
                }
            }
        }

        // If we can't determine the recovery ID, fail instead of defaulting
        Err(MotherError::Crypto(
            "Could not determine ECDSA recovery ID from signature and public key".to_string(),
        ))
    }
}

#[async_trait]
impl HardwareSigner for Pkcs11Device {
    async fn get_info(&self) -> Result<DeviceInfo> {
        // Get slot info
        let slot_info = format!("Key: {}", self.config.key_label);

        // Convert public key to address
        let address = Self::pubkey_to_eth_address(&self.public_key);

        // Use configured model name, fall back to token label
        let model = self
            .config
            .model_name
            .clone()
            .unwrap_or_else(|| self.token_label.clone());

        Ok(DeviceInfo {
            model,
            ready: true,
            public_key: Some(self.public_key),
            address: Some(address),
            extra: Some(slot_info),
        })
    }

    async fn get_public_key(&self, _path: &str) -> Result<([u8; 65], String)> {
        // PKCS#11 devices don't support BIP32 derivation paths.
        // The path parameter is ignored; we use the pre-configured key label instead.
        // HSMs typically manage keys by label/ID rather than hierarchical derivation.
        let address = Self::pubkey_to_eth_address(&self.public_key);
        Ok((self.public_key, address))
    }

    async fn sign_message(&self, _path: &str, message: &[u8]) -> Result<[u8; 65]> {
        // PKCS#11 devices don't support BIP32 derivation paths.
        // The path parameter is ignored; we use the pre-configured key label instead.
        info!("Signing with PKCS#11 HSM...");
        self.sign_with_hsm(message)
    }

    fn device_type(&self) -> &'static str {
        "pkcs11"
    }
}

impl Pkcs11Device {
    /// Convert uncompressed public key to Ethereum address
    fn pubkey_to_eth_address(pubkey: &[u8; 65]) -> String {
        use sha3::{Digest, Keccak256};

        // Skip the 0x04 prefix
        let hash = Keccak256::digest(&pubkey[1..]);
        let address = &hash[12..];
        format!("0x{}", hex::encode(address))
    }
}

impl Drop for Pkcs11Device {
    fn drop(&mut self) {
        // Logout when done
        if let Ok(session) = self.session.lock() {
            let _ = session.logout();
        }
        debug!("PKCS#11 session closed");
    }
}
