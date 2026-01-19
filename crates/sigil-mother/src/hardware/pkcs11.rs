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
}

/// PKCS#11 HSM connection
pub struct Pkcs11Device {
    _ctx: Arc<Pkcs11>,
    session: Arc<Mutex<Session>>,
    key_handle: ObjectHandle,
    public_key: [u8; 65],
    config: Pkcs11Config,
}

impl Pkcs11Device {
    /// Connect to a PKCS#11 HSM
    pub fn connect(config: Pkcs11Config) -> Result<Self> {
        info!("Connecting to PKCS#11 HSM...");

        // Load the PKCS#11 library
        let ctx = Pkcs11::new(Path::new(&config.library_path)).map_err(|e| {
            MotherError::Crypto(format!("Failed to load PKCS#11 library: {}", e))
        })?;

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
    fn get_public_key_from_handle(session: &Session, key_handle: ObjectHandle) -> Result<[u8; 65]> {
        // Get the EC point (public key) attribute
        let attrs = session
            .get_attributes(key_handle, &[AttributeType::EcPoint])
            .map_err(|e| MotherError::Crypto(format!("Failed to get public key: {}", e)))?;

        for attr in attrs {
            if let Attribute::EcPoint(point) = attr {
                // The EC point is DER encoded, need to extract the actual point
                // Format: 04 || len || 04 || x || y (for uncompressed)
                let point_bytes = if point.len() > 65 && point[0] == 0x04 {
                    // DER encoded
                    &point[point.len() - 65..]
                } else if point.len() == 65 {
                    &point[..]
                } else {
                    return Err(MotherError::Crypto(format!(
                        "Invalid EC point length: {}",
                        point.len()
                    )));
                };

                let mut pubkey = [0u8; 65];
                pubkey.copy_from_slice(point_bytes);
                return Ok(pubkey);
            }
        }

        Err(MotherError::Crypto(
            "Could not retrieve public key from HSM".to_string(),
        ))
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
            if let Ok(recovered) =
                VerifyingKey::recover_from_prehash(hash, &signature, recovery_id)
            {
                let recovered_bytes = recovered.to_encoded_point(false);
                if recovered_bytes.as_bytes() == pubkey {
                    return Ok(v + 27); // Ethereum format
                }
            }
        }

        // Default to 27 if we can't determine
        warn!("Could not determine recovery ID, defaulting to 27");
        Ok(27)
    }
}

#[async_trait]
impl HardwareSigner for Pkcs11Device {
    async fn get_info(&self) -> Result<DeviceInfo> {
        // Get slot info
        let slot_info = format!("Key: {}", self.config.key_label);

        // Convert public key to address
        let address = Self::pubkey_to_eth_address(&self.public_key);

        Ok(DeviceInfo {
            model: "PKCS#11 HSM".to_string(),
            ready: true,
            public_key: Some(self.public_key),
            address: Some(address),
            extra: Some(slot_info),
        })
    }

    async fn get_public_key(&self, _path: &str) -> Result<([u8; 65], String)> {
        // PKCS#11 doesn't use BIP32 paths - we use the configured key
        let address = Self::pubkey_to_eth_address(&self.public_key);
        Ok((self.public_key, address))
    }

    async fn sign_message(&self, _path: &str, message: &[u8]) -> Result<[u8; 65]> {
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
