//! Ledger hardware wallet integration
//!
//! Supports Ledger Nano S, S Plus, X, Stax, and Flex devices.
//! Requires the Ethereum app to be open on the device.

use crate::error::{MotherError, Result};
use crate::hardware::{encode_bip32_path, DeviceInfo, HardwareSigner};
use async_trait::async_trait;
use ledger_transport_hid::TransportNativeHID;
use tracing::{debug, info, warn};

/// Ethereum app CLA (instruction class)
const ETH_CLA: u8 = 0xE0;

/// Ethereum app instructions
const ETH_INS_GET_PUBLIC_KEY: u8 = 0x02;
const ETH_INS_SIGN_PERSONAL_MESSAGE: u8 = 0x08;

/// Default BIP32 derivation path for Sigil
pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/60'/0'/0/0";

/// Ledger device connection
pub struct LedgerDevice {
    transport: TransportNativeHID,
}

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

    /// Internal: Get public key from Ledger
    async fn get_public_key_internal(&self, path: &str) -> Result<([u8; 65], String)> {
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

    /// Internal: Sign a personal message using the Ledger
    async fn sign_personal_message_internal(&self, path: &str, message: &[u8]) -> Result<[u8; 65]> {
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
}

#[async_trait]
impl HardwareSigner for LedgerDevice {
    async fn get_info(&self) -> Result<DeviceInfo> {
        // Try to get public key to verify Ethereum app is open
        match self.get_public_key_internal(DEFAULT_DERIVATION_PATH).await {
            Ok((pubkey, address)) => Ok(DeviceInfo {
                model: "Ledger Nano".to_string(),
                ready: true,
                public_key: Some(pubkey),
                address: Some(address),
                extra: Some("Ethereum app open".to_string()),
            }),
            Err(_) => {
                warn!("Ethereum app may not be open on Ledger");
                Ok(DeviceInfo {
                    model: "Ledger Nano".to_string(),
                    ready: false,
                    public_key: None,
                    address: None,
                    extra: Some("Please open Ethereum app".to_string()),
                })
            }
        }
    }

    async fn get_public_key(&self, path: &str) -> Result<([u8; 65], String)> {
        self.get_public_key_internal(path).await
    }

    async fn sign_message(&self, path: &str, message: &[u8]) -> Result<[u8; 65]> {
        self.sign_personal_message_internal(path, message).await
    }

    fn device_type(&self) -> &'static str {
        "ledger"
    }
}
