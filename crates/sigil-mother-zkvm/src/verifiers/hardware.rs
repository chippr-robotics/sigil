//! Hardware derivation proof verifier

use crate::error::{Result, ZkvmError};
use crate::types::HardwareOutput;

use k256::SecretKey;
use sha2::{Digest, Sha256};

/// Verifier for hardware wallet derivation proofs
pub struct HardwareVerifier;

impl HardwareVerifier {
    /// Verify that the hardware output is consistent
    ///
    /// Note: We cannot fully verify without the signature, but we can check
    /// that the shard_pubkey is a valid public key.
    pub fn verify_output(output: &HardwareOutput) -> Result<bool> {
        // Check that shard_pubkey is a valid compressed point
        if output.shard_pubkey.len() != 33 {
            return Ok(false);
        }

        // First byte must be 0x02 or 0x03 for compressed points
        if output.shard_pubkey[0] != 0x02 && output.shard_pubkey[0] != 0x03 {
            return Ok(false);
        }

        // Check that device_pubkey is a valid uncompressed point
        if output.device_pubkey.len() != 65 {
            return Ok(false);
        }

        // First byte must be 0x04 for uncompressed points
        if output.device_pubkey[0] != 0x04 {
            return Ok(false);
        }

        Ok(true)
    }

    /// Verify the derivation computation given the signature
    ///
    /// Checks that `shard = SHA256(domain || signature)` produces the correct public key.
    pub fn verify_derivation(
        output: &HardwareOutput,
        signature: &[u8; 65],
        domain: &[u8],
    ) -> Result<bool> {
        // Compute expected shard
        let expected_shard: [u8; 32] = {
            let mut hasher = Sha256::new();
            hasher.update(domain);
            hasher.update(signature);
            hasher.finalize().into()
        };

        // Compute public key from shard
        let secret_key = SecretKey::from_bytes((&expected_shard).into())
            .map_err(|e| ZkvmError::Crypto(format!("Invalid secret key: {}", e)))?;

        use k256::elliptic_curve::sec1::ToEncodedPoint;
        let public_key = secret_key.public_key();
        let encoded = public_key.to_encoded_point(true);

        let mut computed_pubkey = [0u8; 33];
        computed_pubkey.copy_from_slice(encoded.as_bytes());

        Ok(computed_pubkey == output.shard_pubkey)
    }

    /// Verify an SP1 proof
    #[cfg(feature = "sp1-prover")]
    pub fn verify_sp1(
        proof_bytes: &[u8],
        vkey: &sp1_sdk::SP1VerifyingKey,
        expected_output: &HardwareOutput,
    ) -> Result<bool> {
        use sp1_sdk::ProverClient;

        // Deserialize proof
        let mut proof: sp1_sdk::SP1ProofWithPublicValues = bincode::deserialize(proof_bytes)
            .map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        // Extract output from proof
        let output: HardwareOutput = proof.public_values.read();

        // Verify output matches expected
        if output != *expected_output {
            return Ok(false);
        }

        // Verify the SP1 proof
        let prover = ProverClient::from_env();
        prover
            .verify(&proof, vkey)
            .map_err(|e| ZkvmError::ProofVerificationFailed(e.to_string()))?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_output_valid() {
        // Create a valid-looking output
        let output = HardwareOutput {
            shard_pubkey: {
                let mut pk = [0x02; 33];
                pk[1] = 0x42;
                pk
            },
            device_pubkey: {
                let mut pk = [0x04; 65];
                pk[1] = 0x42;
                pk
            },
            message_hash: [0u8; 32],
            is_cold_shard: true,
        };

        assert!(HardwareVerifier::verify_output(&output).unwrap());
    }

    #[test]
    fn test_verify_output_invalid_shard_prefix() {
        let output = HardwareOutput {
            shard_pubkey: [0x05; 33], // Invalid prefix
            device_pubkey: [0x04; 65],
            message_hash: [0u8; 32],
            is_cold_shard: true,
        };

        assert!(!HardwareVerifier::verify_output(&output).unwrap());
    }

    #[test]
    fn test_verify_output_invalid_device_prefix() {
        let output = HardwareOutput {
            shard_pubkey: [0x02; 33],
            device_pubkey: [0x02; 65], // Wrong prefix for uncompressed
            message_hash: [0u8; 32],
            is_cold_shard: true,
        };

        assert!(!HardwareVerifier::verify_output(&output).unwrap());
    }

    #[test]
    fn test_verify_derivation() {
        use k256::ecdsa::SigningKey;
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        use rand::rngs::OsRng;

        // Create test signature
        let signature = [42u8; 65];
        let domain = b"cold_master_shard";

        // Compute expected shard and pubkey
        let expected_shard: [u8; 32] = {
            let mut hasher = Sha256::new();
            hasher.update(domain);
            hasher.update(&signature);
            hasher.finalize().into()
        };

        let secret_key = SecretKey::from_bytes((&expected_shard).into()).unwrap();
        let public_key = secret_key.public_key();
        let encoded = public_key.to_encoded_point(true);
        let mut shard_pubkey = [0u8; 33];
        shard_pubkey.copy_from_slice(encoded.as_bytes());

        let output = HardwareOutput {
            shard_pubkey,
            device_pubkey: [0x04; 65],
            message_hash: [0u8; 32],
            is_cold_shard: true,
        };

        assert!(HardwareVerifier::verify_derivation(&output, &signature, domain).unwrap());
    }
}
