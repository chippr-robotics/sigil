//! Batch presignature prover
//!
//! Proves: `R_i = (k_cold_i + k_agent_i)*G` for a batch of presignatures
//! Uses Merkle tree commitment for efficiency with large batches.

use k256::{
    elliptic_curve::{sec1::ToEncodedPoint, PrimeField},
    ProjectivePoint, Scalar,
};

use crate::error::{Result, ZkvmError};
use crate::merkle::MerkleTree;
use crate::provers::BatchPresigProverTrait;
use crate::types::{BatchPresigInput, BatchPresigOutput, SampledRPoint};

/// Batch presig prover that can use mock or SP1 backend
pub struct BatchPresigProver;

impl BatchPresigProver {
    /// Execute the batch presig computation (used by both mock and SP1)
    ///
    /// This is the core computation that proves:
    /// - Each `R_i = (k_cold_i + k_agent_i)*G`
    /// - Merkle root commits to all R points
    /// - Sampled points are verified in detail
    pub fn compute(input: &BatchPresigInput) -> Result<BatchPresigOutput> {
        let batch_size = input.batch_size as usize;

        // Validate input
        if input.k_colds.len() != batch_size {
            return Err(ZkvmError::InvalidInput(format!(
                "k_colds length {} doesn't match batch_size {}",
                input.k_colds.len(),
                batch_size
            )));
        }
        if input.k_agents.len() != batch_size {
            return Err(ZkvmError::InvalidInput(format!(
                "k_agents length {} doesn't match batch_size {}",
                input.k_agents.len(),
                batch_size
            )));
        }

        // Compute all R points
        let mut r_points: Vec<[u8; 33]> = Vec::with_capacity(batch_size);

        for i in 0..batch_size {
            let r_point = compute_r_point(&input.k_colds[i], &input.k_agents[i])?;
            r_points.push(r_point);
        }

        // Build Merkle tree
        let merkle_tree = MerkleTree::from_leaves(&r_points)?;
        let merkle_root = merkle_tree.root();

        // Get first and last R points
        let first_r_point = r_points[0];
        let last_r_point = r_points[batch_size - 1];

        // Generate sampled R points with proofs
        let mut sampled_r_points = Vec::new();
        for &sample_idx in &input.sample_indices {
            if sample_idx >= input.batch_size {
                return Err(ZkvmError::InvalidInput(format!(
                    "Sample index {} out of range for batch_size {}",
                    sample_idx, input.batch_size
                )));
            }

            let idx = sample_idx as usize;
            let proof = merkle_tree.proof(idx)?;

            sampled_r_points.push(SampledRPoint {
                r_point: r_points[idx],
                index: sample_idx,
                merkle_proof: proof,
            });
        }

        Ok(BatchPresigOutput {
            r_points_merkle_root: merkle_root,
            first_r_point,
            last_r_point,
            sampled_r_points,
            batch_size: input.batch_size,
            start_index: input.start_index,
            child_pubkey: input.child_pubkey,
        })
    }

    /// Verify that a batch output is consistent
    pub fn verify_output(output: &BatchPresigOutput) -> Result<bool> {
        // Verify all sampled R points have valid Merkle proofs
        for sample in &output.sampled_r_points {
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

    /// Compute all R points from nonce shares (for use in verification)
    pub fn compute_r_points(k_colds: &[[u8; 32]], k_agents: &[[u8; 32]]) -> Result<Vec<[u8; 33]>> {
        if k_colds.len() != k_agents.len() {
            return Err(ZkvmError::InvalidInput(
                "k_colds and k_agents must have same length".into(),
            ));
        }

        let mut r_points = Vec::with_capacity(k_colds.len());
        for i in 0..k_colds.len() {
            let r_point = compute_r_point(&k_colds[i], &k_agents[i])?;
            r_points.push(r_point);
        }

        Ok(r_points)
    }
}

/// Compute a single R point: R = (k_cold + k_agent) * G
fn compute_r_point(k_cold: &[u8; 32], k_agent: &[u8; 32]) -> Result<[u8; 33]> {
    let k_cold_scalar = Scalar::from_repr((*k_cold).into())
        .into_option()
        .ok_or_else(|| ZkvmError::Crypto("Invalid k_cold scalar".into()))?;

    let k_agent_scalar = Scalar::from_repr((*k_agent).into())
        .into_option()
        .ok_or_else(|| ZkvmError::Crypto("Invalid k_agent scalar".into()))?;

    let k_combined = k_cold_scalar + k_agent_scalar;
    let r_point = ProjectivePoint::GENERATOR * k_combined;
    let r_affine = r_point.to_affine();

    let r_bytes: [u8; 33] = r_affine
        .to_encoded_point(true)
        .as_bytes()
        .try_into()
        .map_err(|_| ZkvmError::Crypto("Failed to encode R point".into()))?;

    Ok(r_bytes)
}

/// SP1 batch presig prover (requires sp1-prover feature)
#[cfg(feature = "sp1-prover")]
pub struct Sp1BatchPresigProver {
    prover: sp1_sdk::EnvProver,
    pk: sp1_sdk::SP1ProvingKey,
    vk: sp1_sdk::SP1VerifyingKey,
}

#[cfg(feature = "sp1-prover")]
impl Sp1BatchPresigProver {
    /// Create a new SP1 batch presig prover
    pub fn new() -> Result<Self> {
        use sp1_sdk::ProverClient;

        let prover = ProverClient::from_env();

        // Load the ELF from the built program
        let elf = include_bytes!("../../programs/batch/elf/riscv32im-succinct-zkvm-elf");

        let (pk, vk) = prover.setup(elf);

        Ok(Self { prover, pk, vk })
    }

    /// Get the verification key
    pub fn vkey(&self) -> &sp1_sdk::SP1VerifyingKey {
        &self.vk
    }
}

#[cfg(feature = "sp1-prover")]
impl BatchPresigProverTrait for Sp1BatchPresigProver {
    fn prove(&self, input: BatchPresigInput) -> Result<(BatchPresigOutput, Vec<u8>)> {
        use sp1_sdk::SP1Stdin;

        // Write input to SP1 stdin
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);

        // Generate proof
        let mut proof = self
            .prover
            .prove(&self.pk, &stdin)
            .run()
            .map_err(|e| ZkvmError::Sp1Error(e.to_string()))?;

        // Decode output
        let output: BatchPresigOutput = proof.public_values.read();

        // Serialize proof
        let proof_bytes =
            bincode::serialize(&proof).map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        Ok((output, proof_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_batch_presig_small() {
        let input = create_test_input(10);
        let output = BatchPresigProver::compute(&input).unwrap();

        assert_eq!(output.batch_size, 10);
        assert_eq!(output.start_index, 0);
        assert_eq!(output.sampled_r_points.len(), 3);

        // Verify Merkle proofs
        assert!(BatchPresigProver::verify_output(&output).unwrap());
    }

    #[test]
    fn test_batch_presig_large() {
        let input = create_test_input(1000);
        let output = BatchPresigProver::compute(&input).unwrap();

        assert_eq!(output.batch_size, 1000);

        // Verify Merkle proofs
        assert!(BatchPresigProver::verify_output(&output).unwrap());
    }

    #[test]
    fn test_batch_presig_deterministic() {
        let input = create_test_input(100);

        let output1 = BatchPresigProver::compute(&input).unwrap();
        let output2 = BatchPresigProver::compute(&input).unwrap();

        assert_eq!(output1.r_points_merkle_root, output2.r_points_merkle_root);
        assert_eq!(output1.first_r_point, output2.first_r_point);
        assert_eq!(output1.last_r_point, output2.last_r_point);
    }

    #[test]
    fn test_r_point_computation() {
        let k_cold = [1u8; 32];
        let k_agent = [2u8; 32];

        let r_point = compute_r_point(&k_cold, &k_agent).unwrap();

        // Verify it's a valid compressed point
        assert!(r_point[0] == 0x02 || r_point[0] == 0x03);
    }
}
