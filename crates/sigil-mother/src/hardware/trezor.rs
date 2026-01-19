//! Trezor hardware wallet integration
//!
//! Supports Trezor Model One, Model T, Safe 3, Safe 5, and Safe 7 devices.
//!
//! # Requirements
//! - Trezor Bridge must be running, OR
//! - Direct USB access via udev rules
//! - Device must be unlocked with PIN
//!
//! # Note
//! This implementation uses the trezor-client crate which requires mutable access
//! to the client. The client is wrapped in a Mutex for thread safety.

use crate::error::{MotherError, Result};
use crate::hardware::{DeviceInfo, HardwareSigner};
use async_trait::async_trait;
use std::sync::Mutex;
use trezor_client::client::Trezor;
use tracing::{debug, info, warn};

/// Default BIP32 derivation path for Sigil (Ethereum)
pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

/// Trezor device connection
pub struct TrezorDevice {
    client: Mutex<Trezor>,
}

impl TrezorDevice {
    /// Connect to a Trezor device
    pub fn connect() -> Result<Self> {
        info!("Searching for Trezor device...");

        // Find available devices
        let devices = trezor_client::find_devices(false);

        if devices.is_empty() {
            return Err(MotherError::Crypto(
                "Trezor device not found. Ensure device is connected and unlocked.".to_string(),
            ));
        }

        // Connect to the first available device
        let device = devices
            .into_iter()
            .next()
            .ok_or_else(|| MotherError::Crypto("No Trezor device found".to_string()))?;

        let client = device
            .connect()
            .map_err(|e| MotherError::Crypto(format!("Failed to connect to Trezor: {}", e)))?;

        info!("Trezor device connected");
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    /// Parse BIP32 path string to vector of u32 components
    fn parse_path(path: &str) -> Result<Vec<u32>> {
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
            result.push(component);
        }

        Ok(result)
    }
}

#[async_trait]
impl HardwareSigner for TrezorDevice {
    async fn get_info(&self) -> Result<DeviceInfo> {
        let client = self
            .client
            .lock()
            .map_err(|e| MotherError::Crypto(format!("Failed to lock client: {}", e)))?;

        // Get device features
        let features = client.features();

        match features {
            Some(feat) => {
                let model = feat
                    .model
                    .as_ref()
                    .map(|m| format!("Trezor {}", m))
                    .unwrap_or_else(|| "Trezor".to_string());

                let initialized = feat.initialized.unwrap_or(false);
                let pin_protection = feat.pin_protection.unwrap_or(false);
                let unlocked = feat.unlocked.unwrap_or(true);

                let ready = initialized && (!pin_protection || unlocked);

                let extra = if !initialized {
                    Some("Device not initialized".to_string())
                } else if pin_protection && !unlocked {
                    Some("Device locked - enter PIN".to_string())
                } else {
                    Some("Device ready".to_string())
                };

                Ok(DeviceInfo {
                    model,
                    ready,
                    public_key: None, // Would need another call to get this
                    address: None,
                    extra,
                })
            }
            None => {
                warn!("Could not get Trezor device features");
                Ok(DeviceInfo {
                    model: "Trezor".to_string(),
                    ready: false,
                    public_key: None,
                    address: None,
                    extra: Some("Could not read device features".to_string()),
                })
            }
        }
    }

    async fn get_public_key(&self, path: &str) -> Result<([u8; 65], String)> {
        let path_components = Self::parse_path(path)?;

        let mut client = self
            .client
            .lock()
            .map_err(|e| MotherError::Crypto(format!("Failed to lock client: {}", e)))?;

        // Get Ethereum address
        let address = client
            .ethereum_get_address(path_components.clone())
            .map_err(|e| MotherError::Crypto(format!("Failed to get Ethereum address: {}", e)))?;

        // For public key, we need to derive it from the address response
        // The Trezor API doesn't directly expose the public key for Ethereum
        // We'll return a placeholder that would need to be filled from a different API call
        // or by using bitcoin_get_public_key with appropriate path conversion

        // Return placeholder - in production, would need proper implementation
        let pubkey = [0u8; 65];
        warn!("Trezor public key retrieval not fully implemented yet");

        debug!("Got address from Trezor: {}", address);
        Ok((pubkey, address))
    }

    async fn sign_message(&self, path: &str, message: &[u8]) -> Result<[u8; 65]> {
        let path_components = Self::parse_path(path)?;

        info!("Please confirm the signature on your Trezor device...");

        let mut client = self
            .client
            .lock()
            .map_err(|e| MotherError::Crypto(format!("Failed to lock client: {}", e)))?;

        // Sign using Ethereum personal message signing
        let signature = client
            .ethereum_sign_message(message.to_vec(), path_components)
            .map_err(|e| MotherError::Crypto(format!("Trezor signing error: {}", e)))?;

        // Signature format: r (32 bytes) + s (32 bytes) + v (1 byte)
        let mut sig = [0u8; 65];

        // Trezor returns r, s, v separately
        if signature.r.len() != 32 || signature.s.len() != 32 {
            return Err(MotherError::Crypto(format!(
                "Unexpected signature component lengths: r={}, s={}",
                signature.r.len(),
                signature.s.len()
            )));
        }

        sig[0..32].copy_from_slice(&signature.r);
        sig[32..64].copy_from_slice(&signature.s);
        sig[64] = signature.v as u8;

        debug!("Got signature from Trezor");
        Ok(sig)
    }

    fn device_type(&self) -> &'static str {
        "trezor"
    }
}
