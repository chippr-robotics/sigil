//! Derive proof verifier

use crate::error::Result;
#[cfg(feature = "sp1-prover")]
use crate::error::ZkvmError;
use crate::provers::DeriveProver;
use crate::types::DeriveOutput;

/// Verifier for child key derivation proofs
pub struct DeriveVerifier;

impl DeriveVerifier {
    /// Verify that the derive output is mathematically consistent
    ///
    /// Checks that `child_pubkey = cold_child_pubkey + agent_child_pubkey`
    pub fn verify_output(output: &DeriveOutput) -> Result<bool> {
        DeriveProver::verify_child_pubkey_combination(
            &output.cold_child_pubkey,
            &output.agent_child_pubkey,
            &output.child_pubkey,
        )
    }

    /// Verify an SP1 proof
    #[cfg(feature = "sp1-prover")]
    pub fn verify_sp1(
        proof_bytes: &[u8],
        vkey: &sp1_sdk::SP1VerifyingKey,
        expected_output: &DeriveOutput,
    ) -> Result<bool> {
        use sp1_sdk::ProverClient;

        // Deserialize proof
        let mut proof: sp1_sdk::SP1ProofWithPublicValues = bincode::deserialize(proof_bytes)
            .map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        // Extract output from proof
        let output: DeriveOutput = proof.public_values.read();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provers::DeriveProver;
    use crate::types::DeriveInput;
    use k256::{
        elliptic_curve::{sec1::ToEncodedPoint, PrimeField},
        ProjectivePoint, Scalar,
    };

    fn create_test_input() -> DeriveInput {
        let cold_master = [1u8; 32];
        let agent_master = [2u8; 32];

        let cold_scalar = Scalar::from_repr(cold_master.into()).unwrap();
        let agent_scalar = Scalar::from_repr(agent_master.into()).unwrap();

        let cold_point = ProjectivePoint::GENERATOR * cold_scalar;
        let agent_point = ProjectivePoint::GENERATOR * agent_scalar;
        let master_point = cold_point + agent_point;

        let master_pubkey: [u8; 33] = master_point
            .to_affine()
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .unwrap();

        DeriveInput {
            cold_master_shard: cold_master,
            agent_master_shard: agent_master,
            derivation_path: vec![0x80, 0x00, 0x00, 0x2c],
            master_pubkey,
        }
    }

    #[test]
    fn test_verify_output_valid() {
        let input = create_test_input();
        let output = DeriveProver::compute(&input).unwrap();

        assert!(DeriveVerifier::verify_output(&output).unwrap());
    }

    #[test]
    fn test_verify_output_invalid() {
        let input = create_test_input();
        let mut output = DeriveProver::compute(&input).unwrap();

        // Tamper with the child pubkey
        output.child_pubkey[1] ^= 0xFF;

        assert!(!DeriveVerifier::verify_output(&output).unwrap());
    }
}
