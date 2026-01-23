//! Batch presig proof verifier

use crate::error::Result;
#[cfg(feature = "sp1-prover")]
use crate::error::ZkvmError;
use crate::merkle::MerkleTree;
use crate::types::BatchPresigOutput;

/// Verifier for batch presignature proofs
pub struct BatchPresigVerifier;

impl BatchPresigVerifier {
    /// Verify that the batch presig output is consistent
    ///
    /// Checks:
    /// - All sampled R points have valid Merkle proofs
    /// - Batch size and indices are consistent
    pub fn verify_output(output: &BatchPresigOutput) -> Result<bool> {
        // Verify Merkle proofs for all sampled R points
        for sample in &output.sampled_r_points {
            if sample.index >= output.batch_size {
                return Ok(false);
            }

            if !MerkleTree::verify_proof(
                &output.r_points_merkle_root,
                &sample.r_point,
                sample.index as usize,
                &sample.merkle_proof,
            ) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Verify output with expected R points (for full verification)
    ///
    /// This verifies that the Merkle root matches the expected R points.
    pub fn verify_with_r_points(
        output: &BatchPresigOutput,
        expected_r_points: &[[u8; 33]],
    ) -> Result<bool> {
        if expected_r_points.len() != output.batch_size as usize {
            return Ok(false);
        }

        // Build Merkle tree from expected R points
        let tree = MerkleTree::from_leaves(expected_r_points)?;

        // Check root matches
        if tree.root() != output.r_points_merkle_root {
            return Ok(false);
        }

        // Check first and last R points
        if expected_r_points[0] != output.first_r_point {
            return Ok(false);
        }

        if expected_r_points[expected_r_points.len() - 1] != output.last_r_point {
            return Ok(false);
        }

        Ok(true)
    }

    /// Verify an SP1 proof
    #[cfg(feature = "sp1-prover")]
    pub fn verify_sp1(
        proof_bytes: &[u8],
        vkey: &sp1_sdk::SP1VerifyingKey,
        expected_output: &BatchPresigOutput,
    ) -> Result<bool> {
        use sp1_sdk::ProverClient;

        // Deserialize proof
        let mut proof: sp1_sdk::SP1ProofWithPublicValues = bincode::deserialize(proof_bytes)
            .map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        // Extract output from proof
        let output: BatchPresigOutput = proof.public_values.read();

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
    use crate::provers::BatchPresigProver;
    use crate::types::BatchPresigInput;

    fn create_test_input(batch_size: u32) -> BatchPresigInput {
        let mut k_colds = Vec::new();
        let mut k_agents = Vec::new();

        for i in 0..batch_size {
            let mut k_cold = [0u8; 32];
            let mut k_agent = [0u8; 32];
            k_cold[0] = i as u8;
            k_cold[1] = 1;
            k_agent[0] = i as u8;
            k_agent[1] = 2;
            k_colds.push(k_cold);
            k_agents.push(k_agent);
        }

        BatchPresigInput {
            cold_child_shard: [1u8; 32],
            agent_child_shard: [2u8; 32],
            k_colds,
            k_agents,
            child_pubkey: [0x02; 33],
            start_index: 0,
            batch_size,
            sample_indices: vec![0, batch_size / 2, batch_size - 1],
        }
    }

    #[test]
    fn test_verify_output_valid() {
        let input = create_test_input(100);
        let output = BatchPresigProver::compute(&input).unwrap();

        assert!(BatchPresigVerifier::verify_output(&output).unwrap());
    }

    #[test]
    fn test_verify_output_invalid_merkle_proof() {
        let input = create_test_input(100);
        let mut output = BatchPresigProver::compute(&input).unwrap();

        // Tamper with a Merkle proof
        if !output.sampled_r_points.is_empty() {
            output.sampled_r_points[0].merkle_proof[0][0] ^= 0xFF;
        }

        assert!(!BatchPresigVerifier::verify_output(&output).unwrap());
    }

    #[test]
    fn test_verify_with_r_points() {
        let input = create_test_input(50);
        let output = BatchPresigProver::compute(&input).unwrap();

        // Compute expected R points
        let r_points =
            BatchPresigProver::compute_r_points(&input.k_colds, &input.k_agents).unwrap();

        assert!(BatchPresigVerifier::verify_with_r_points(&output, &r_points).unwrap());
    }
}
