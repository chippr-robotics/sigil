//! Accumulator publication for daemon distribution
//!
//! The mother device exports the RSA accumulator state for distribution
//! to daemons. Daemons use this to verify agent non-membership witnesses
//! before allowing signing operations.
//!
//! Distribution methods:
//! - File export (USB transfer)
//! - QR code (for small updates)

use serde::{Deserialize, Serialize};
use sigil_core::{
    accumulator::{RsaAccumulator, StoredAccumulator, RSA_MODULUS_SIZE},
    Signature,
};

use crate::error::{MotherError, Result};

/// Accumulator export format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatorExport {
    /// Format version
    pub version: u8,

    /// The accumulator state
    pub accumulator: RsaAccumulator,

    /// Mother's signature over the export
    pub signature: Signature,

    /// Timestamp of export
    pub exported_at: u64,

    /// Optional notes/metadata
    pub notes: Option<String>,
}

impl AccumulatorExport {
    /// Create a new export from an accumulator
    pub fn new(accumulator: RsaAccumulator, mother_signature: Signature) -> Self {
        let exported_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: 1,
            accumulator,
            signature: mother_signature,
            exported_at,
            notes: None,
        }
    }

    /// Create with notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| MotherError::Serialization(e.to_string()))
    }

    /// Serialize to compact binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Version
        bytes.push(self.version);

        // Accumulator
        bytes.extend_from_slice(&self.accumulator.modulus);
        bytes.extend_from_slice(&self.accumulator.accumulator);
        bytes.extend_from_slice(&self.accumulator.generator);
        bytes.extend_from_slice(&self.accumulator.version.to_le_bytes());

        // Signature
        bytes.extend_from_slice(self.signature.as_bytes());

        // Timestamp
        bytes.extend_from_slice(&self.exported_at.to_le_bytes());

        // Notes length + notes
        if let Some(ref notes) = self.notes {
            let notes_bytes = notes.as_bytes();
            bytes.extend_from_slice(&(notes_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(notes_bytes);
        } else {
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }

        bytes
    }

    /// Deserialize from binary format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 1 + RSA_MODULUS_SIZE * 3 + 8 + 64 + 8 + 4 {
            return Err(MotherError::InvalidDiskFormat(
                "Export too short".to_string(),
            ));
        }

        let mut offset = 0;

        // Version
        let version = bytes[offset];
        offset += 1;

        // Accumulator
        let mut modulus = [0u8; RSA_MODULUS_SIZE];
        let mut accumulator_val = [0u8; RSA_MODULUS_SIZE];
        let mut generator = [0u8; RSA_MODULUS_SIZE];

        modulus.copy_from_slice(&bytes[offset..offset + RSA_MODULUS_SIZE]);
        offset += RSA_MODULUS_SIZE;

        accumulator_val.copy_from_slice(&bytes[offset..offset + RSA_MODULUS_SIZE]);
        offset += RSA_MODULUS_SIZE;

        generator.copy_from_slice(&bytes[offset..offset + RSA_MODULUS_SIZE]);
        offset += RSA_MODULUS_SIZE;

        let acc_version = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let accumulator = RsaAccumulator {
            modulus,
            accumulator: accumulator_val,
            generator,
            version: acc_version,
        };

        // Signature
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&bytes[offset..offset + 64]);
        let signature = Signature::new(sig_bytes);
        offset += 64;

        // Timestamp
        let exported_at = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        // Notes
        let notes_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        let notes = if notes_len > 0 && offset + notes_len <= bytes.len() {
            let notes_str = String::from_utf8_lossy(&bytes[offset..offset + notes_len]);
            Some(notes_str.to_string())
        } else {
            None
        };

        Ok(Self {
            version,
            accumulator,
            signature,
            exported_at,
            notes,
        })
    }

    /// Get the data that should be signed
    pub fn signable_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.version);
        data.extend_from_slice(&self.accumulator.modulus);
        data.extend_from_slice(&self.accumulator.accumulator);
        data.extend_from_slice(&self.accumulator.version.to_le_bytes());
        data.extend_from_slice(&self.exported_at.to_le_bytes());
        data
    }

    /// Convert to StoredAccumulator format for daemon
    pub fn to_stored_accumulator(&self) -> StoredAccumulator {
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(self.signature.as_bytes());

        StoredAccumulator::new(self.accumulator.clone(), sig_bytes, self.exported_at)
    }
}

/// Accumulator publisher
pub struct AccumulatorPublisher {
    /// Mother's signing key (for signing exports)
    #[allow(dead_code)]
    signing_key: [u8; 32],
}

impl AccumulatorPublisher {
    /// Create a new publisher
    pub fn new(signing_key: [u8; 32]) -> Self {
        Self { signing_key }
    }

    /// Export accumulator to a file
    pub fn export_to_file(
        &self,
        accumulator: &RsaAccumulator,
        path: &std::path::Path,
    ) -> Result<()> {
        // Sign the export
        let signature = self.sign_accumulator(accumulator);
        let export = AccumulatorExport::new(accumulator.clone(), signature);

        // Write to file
        let bytes = export.to_bytes();
        std::fs::write(path, bytes).map_err(MotherError::Io)?;

        Ok(())
    }

    /// Export accumulator to JSON string (for display or transfer)
    pub fn export_to_json(&self, accumulator: &RsaAccumulator) -> Result<String> {
        let signature = self.sign_accumulator(accumulator);
        let export = AccumulatorExport::new(accumulator.clone(), signature);
        export.to_json()
    }

    /// Export accumulator as a QR-encodable string
    ///
    /// For small accumulators, this can be encoded in a single QR code.
    /// Larger data should use chunked encoding.
    pub fn export_for_qr(&self, accumulator: &RsaAccumulator) -> Result<String> {
        let signature = self.sign_accumulator(accumulator);
        let export = AccumulatorExport::new(accumulator.clone(), signature);
        let bytes = export.to_bytes();

        // Base64 encode for QR
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

        Ok(format!("SIGIL:ACC:1:{}", encoded))
    }

    /// Sign the accumulator export
    fn sign_accumulator(&self, accumulator: &RsaAccumulator) -> Signature {
        use sha2::{Digest, Sha256};

        // Create signature data
        let mut data = Vec::new();
        data.extend_from_slice(&accumulator.modulus);
        data.extend_from_slice(&accumulator.accumulator);
        data.extend_from_slice(&accumulator.version.to_le_bytes());

        // Hash the data
        let hash = Sha256::digest(&data);

        // Create a deterministic "signature" from the hash and signing key
        // In production, use proper ECDSA signing
        let mut sig_bytes = [0u8; 64];
        let mut hasher = Sha256::new();
        hasher.update(self.signing_key);
        hasher.update(hash);
        sig_bytes[..32].copy_from_slice(&hasher.finalize());

        let mut hasher = Sha256::new();
        hasher.update(&sig_bytes[..32]);
        hasher.update(self.signing_key);
        sig_bytes[32..].copy_from_slice(&hasher.finalize());

        Signature::new(sig_bytes)
    }
}

/// Load an accumulator export from file
pub fn load_from_file(path: &std::path::Path) -> Result<AccumulatorExport> {
    let bytes = std::fs::read(path).map_err(MotherError::Io)?;
    AccumulatorExport::from_bytes(&bytes)
}

/// Decode accumulator from QR data
pub fn decode_from_qr(qr_data: &str) -> Result<AccumulatorExport> {
    let data = qr_data.strip_prefix("SIGIL:ACC:1:").ok_or_else(|| {
        MotherError::InvalidDiskFormat("Invalid accumulator QR prefix".to_string())
    })?;

    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
        .map_err(|e| MotherError::Serialization(format!("Base64 decode failed: {}", e)))?;

    AccumulatorExport::from_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_accumulator() -> RsaAccumulator {
        let mut modulus = [0u8; RSA_MODULUS_SIZE];
        let mut generator = [0u8; RSA_MODULUS_SIZE];
        modulus[0] = 0x80;
        modulus[RSA_MODULUS_SIZE - 1] = 0xFB;
        generator[RSA_MODULUS_SIZE - 1] = 3;
        RsaAccumulator::new(modulus, generator)
    }

    #[test]
    fn test_export_roundtrip() {
        let accumulator = test_accumulator();
        let signature = Signature::new([0x42; 64]);
        let export = AccumulatorExport::new(accumulator.clone(), signature);

        let bytes = export.to_bytes();
        let recovered = AccumulatorExport::from_bytes(&bytes).unwrap();

        assert_eq!(recovered.version, export.version);
        assert_eq!(recovered.accumulator.version, accumulator.version);
        assert_eq!(recovered.accumulator.modulus, accumulator.modulus);
    }

    #[test]
    fn test_export_to_json() {
        let accumulator = test_accumulator();
        let signature = Signature::new([0x42; 64]);
        let export = AccumulatorExport::new(accumulator, signature);

        let json = export.to_json().unwrap();
        assert!(json.contains("version"));
        assert!(json.contains("accumulator"));
    }

    #[test]
    fn test_publisher_qr_export() {
        let publisher = AccumulatorPublisher::new([0x01; 32]);
        let accumulator = test_accumulator();

        let qr_data = publisher.export_for_qr(&accumulator).unwrap();
        assert!(qr_data.starts_with("SIGIL:ACC:1:"));

        let recovered = decode_from_qr(&qr_data).unwrap();
        assert_eq!(recovered.accumulator.modulus, accumulator.modulus);
    }

    #[test]
    fn test_to_stored_accumulator() {
        let accumulator = test_accumulator();
        let signature = Signature::new([0x42; 64]);
        let export = AccumulatorExport::new(accumulator.clone(), signature);

        let stored = export.to_stored_accumulator();
        assert_eq!(stored.version(), accumulator.version);
    }
}
