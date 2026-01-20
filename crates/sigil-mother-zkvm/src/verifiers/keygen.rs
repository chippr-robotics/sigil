//! Keygen proof verifier

use crate::error::{Result, ZkvmError};
use crate::types::KeygenOutput;

use k256::{
    elliptic_curve::sec1::{FromEncodedPoint, ToEncodedPoint},
    AffinePoint, EncodedPoint, ProjectivePoint,
};

/// Verifier for master key generation proofs
pub struct KeygenVerifier;

impl KeygenVerifier {
    /// Verify that the keygen output is mathematically consistent
    ///
    /// Checks that `master_pubkey = cold_pubkey + agent_pubkey`
    pub fn verify_output(output: &KeygenOutput) -> Result<bool> {
        // Decode public keys
        let cold_point = decode_point(&output.cold_pubkey)?;
        let agent_point = decode_point(&output.agent_pubkey)?;

        // Compute expected combined point
        let combined = cold_point + agent_point;
        let combined_affine = combined.to_affine();
        let combined_bytes: [u8; 33] = combined_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode combined point".into()))?;

        // Check if it matches the claimed master public key
        Ok(combined_bytes == output.master_pubkey)
    }

    /// Verify an SP1 proof
    #[cfg(feature = "sp1-prover")]
    pub fn verify_sp1(
        proof_bytes: &[u8],
        vkey: &sp1_sdk::SP1VerifyingKey,
        expected_output: &KeygenOutput,
    ) -> Result<bool> {
        use sp1_sdk::ProverClient;

        // Deserialize proof
        let mut proof: sp1_sdk::SP1ProofWithPublicValues = bincode::deserialize(proof_bytes)
            .map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        // Extract output from proof
        let output: KeygenOutput = proof.public_values.read();

        // Verify output matches expected
        if output != *expected_output {
            return Ok(false);
        }

        // Verify the SP1 proof
        let client = ProverClient::from_env();
        client
            .verify(&proof, vkey)
            .map_err(|e| ZkvmError::ProofVerificationFailed(e.to_string()))?;

        Ok(true)
    }
}

/// Decode a compressed point from bytes
fn decode_point(bytes: &[u8; 33]) -> Result<ProjectivePoint> {
    let encoded = EncodedPoint::from_bytes(bytes)
        .map_err(|_| ZkvmError::Crypto("Invalid point encoding".into()))?;

    let affine = AffinePoint::from_encoded_point(&encoded);
    if affine.is_none().into() {
        return Err(ZkvmError::Crypto("Invalid curve point".into()));
    }

    Ok(ProjectivePoint::from(affine.unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::KeygenInput;

    #[test]
    fn test_verify_output_valid() {
        let input = KeygenInput {
            cold_shard: [1u8; 32],
            agent_shard: [2u8; 32],
            ceremony_nonce: [3u8; 32],
        };

        let output = KeygenProver::compute(&input).unwrap();

        // Output should be valid
        assert!(KeygenVerifier::verify_output(&output).unwrap());
    }

    #[test]
    fn test_verify_output_invalid() {
        let input = KeygenInput {
            cold_shard: [1u8; 32],
            agent_shard: [2u8; 32],
            ceremony_nonce: [3u8; 32],
        };

        let mut output = KeygenProver::compute(&input).unwrap();

        // Tamper with the master pubkey
        output.master_pubkey[1] ^= 0xFF;

        // Output should be invalid
        assert!(!KeygenVerifier::verify_output(&output).unwrap());
    }
}
