//! zkVM integration for proof generation during ceremonies
//!
//! This module provides integration with sigil-mother-zkvm for generating
//! zero-knowledge proofs of mother device operations.
//!
//! # Feature Requirements
//!
//! - `zkvm-mock` - Use mock provers for testing
//! - `zkvm-sp1` - Use real SP1 provers (requires SP1 toolchain)

#[cfg(feature = "zkvm")]
use sigil_mother_zkvm::{
    provers::CombinedProver,
    storage::ProofStorage,
    types::{BatchPresigInput, DeriveInput, KeygenInput, ProofType},
    BatchPresigOutput, DeriveOutput, KeygenOutput,
};

#[cfg(feature = "zkvm")]
use crate::error::{MotherError, Result};

#[cfg(feature = "zkvm")]
use std::path::Path;

/// Proof generator for mother device operations
#[cfg(feature = "zkvm")]
pub struct ProofGenerator {
    prover: CombinedProver,
    storage: ProofStorage,
}

#[cfg(feature = "zkvm")]
impl ProofGenerator {
    /// Create a new proof generator with mock provers
    #[cfg(feature = "zkvm-mock")]
    pub fn mock(storage_path: impl AsRef<Path>) -> Self {
        Self {
            prover: CombinedProver::mock(),
            storage: ProofStorage::new(storage_path),
        }
    }

    /// Create a new proof generator with SP1 provers
    #[cfg(feature = "zkvm-sp1")]
    pub fn sp1(storage_path: impl AsRef<Path>) -> Result<Self> {
        let prover = CombinedProver::sp1()
            .map_err(|e| MotherError::Crypto(format!("Failed to create SP1 prover: {}", e)))?;

        Ok(Self {
            prover,
            storage: ProofStorage::new(storage_path),
        })
    }

    /// Check if this is using mock provers
    pub fn is_mock(&self) -> bool {
        use sigil_mother_zkvm::provers::MotherProver;
        self.prover.is_mock()
    }

    /// Generate and store a keygen proof
    pub fn prove_keygen(
        &self,
        cold_shard: &[u8; 32],
        agent_shard: &[u8; 32],
        ceremony_nonce: &[u8; 32],
    ) -> Result<KeygenOutput> {
        use sigil_mother_zkvm::provers::MotherProver;

        let input = KeygenInput {
            cold_shard: *cold_shard,
            agent_shard: *agent_shard,
            ceremony_nonce: *ceremony_nonce,
        };

        let (output, proof) = self
            .prover
            .prove_keygen(input)
            .map_err(|e| MotherError::Crypto(format!("Keygen proof failed: {}", e)))?;

        // Store the proof
        self.storage
            .save_keygen_proof(&output, &proof, self.is_mock())
            .map_err(|e| MotherError::Storage(format!("Failed to save keygen proof: {}", e)))?;

        Ok(output)
    }

    /// Generate and store a derivation proof
    pub fn prove_derive(
        &self,
        child_id: &str,
        cold_master_shard: &[u8; 32],
        agent_master_shard: &[u8; 32],
        derivation_path: &[u8],
        master_pubkey: &[u8; 33],
    ) -> Result<DeriveOutput> {
        use sigil_mother_zkvm::provers::MotherProver;

        let input = DeriveInput {
            cold_master_shard: *cold_master_shard,
            agent_master_shard: *agent_master_shard,
            derivation_path: derivation_path.to_vec(),
            master_pubkey: *master_pubkey,
        };

        let (output, proof) = self
            .prover
            .prove_derive(input)
            .map_err(|e| MotherError::Crypto(format!("Derive proof failed: {}", e)))?;

        // Store the proof
        self.storage
            .save_derive_proof(child_id, &output, &proof, self.is_mock())
            .map_err(|e| MotherError::Storage(format!("Failed to save derive proof: {}", e)))?;

        Ok(output)
    }

    /// Generate and store a batch presig proof
    pub fn prove_batch_presig(
        &self,
        child_id: &str,
        cold_child_shard: &[u8; 32],
        agent_child_shard: &[u8; 32],
        k_colds: Vec<[u8; 32]>,
        k_agents: Vec<[u8; 32]>,
        child_pubkey: &[u8; 33],
        start_index: u32,
        sample_count: usize,
    ) -> Result<BatchPresigOutput> {
        use sigil_mother_zkvm::provers::MotherProver;

        let batch_size = k_colds.len() as u32;

        // Generate sample indices (deterministic based on batch parameters)
        let sample_indices = generate_sample_indices(batch_size, sample_count, start_index);

        let input = BatchPresigInput {
            cold_child_shard: *cold_child_shard,
            agent_child_shard: *agent_child_shard,
            k_colds,
            k_agents,
            child_pubkey: *child_pubkey,
            start_index,
            batch_size,
            sample_indices,
        };

        let (output, proof) = self
            .prover
            .prove_batch_presig(input)
            .map_err(|e| MotherError::Crypto(format!("Batch presig proof failed: {}", e)))?;

        // Store the proof
        self.storage
            .save_batch_proof(child_id, &output, &proof, self.is_mock())
            .map_err(|e| MotherError::Storage(format!("Failed to save batch proof: {}", e)))?;

        Ok(output)
    }

    /// Get the proof storage for direct access
    pub fn storage(&self) -> &ProofStorage {
        &self.storage
    }
}

/// Generate deterministic sample indices for batch verification
#[cfg(feature = "zkvm")]
fn generate_sample_indices(batch_size: u32, sample_count: usize, seed: u32) -> Vec<u32> {
    use sha2::{Digest, Sha256};

    if batch_size == 0 {
        return vec![];
    }

    // Always include first and last
    let mut indices = vec![0, batch_size - 1];

    // Generate additional samples if needed
    if sample_count > 2 && batch_size > 2 {
        let mut hasher = Sha256::new();
        hasher.update(seed.to_le_bytes());
        hasher.update(batch_size.to_le_bytes());
        let hash = hasher.finalize();

        for i in 0..(sample_count - 2).min((batch_size - 2) as usize) {
            let offset = i * 4;
            let rand_bytes: [u8; 4] = if offset + 4 <= hash.len() {
                hash[offset..offset + 4].try_into().unwrap()
            } else {
                // Extend hash if needed
                let mut extended_hasher = Sha256::new();
                extended_hasher.update(&hash);
                extended_hasher.update(&[i as u8]);
                let extended = extended_hasher.finalize();
                extended[0..4].try_into().unwrap()
            };

            let rand_val = u32::from_le_bytes(rand_bytes);
            let index = 1 + (rand_val % (batch_size - 2));

            if !indices.contains(&index) {
                indices.push(index);
            }
        }
    }

    indices.sort();
    indices.dedup();
    indices
}

/// Proof verification utilities
#[cfg(feature = "zkvm")]
pub mod verify {
    use super::*;
    use sigil_mother_zkvm::verifiers::{
        BatchPresigVerifier, CombinedVerifier, DeriveVerifier, KeygenVerifier, MotherVerifier,
    };

    /// Verify a keygen proof output
    pub fn verify_keygen_output(output: &KeygenOutput) -> Result<bool> {
        KeygenVerifier::verify_output(output)
            .map_err(|e| MotherError::Crypto(format!("Keygen verification failed: {}", e)))
    }

    /// Verify a derive proof output
    pub fn verify_derive_output(output: &DeriveOutput) -> Result<bool> {
        DeriveVerifier::verify_output(output)
            .map_err(|e| MotherError::Crypto(format!("Derive verification failed: {}", e)))
    }

    /// Verify a batch presig proof output
    pub fn verify_batch_output(output: &BatchPresigOutput) -> Result<bool> {
        BatchPresigVerifier::verify_output(output)
            .map_err(|e| MotherError::Crypto(format!("Batch verification failed: {}", e)))
    }

    /// Create a combined verifier
    pub fn create_verifier() -> CombinedVerifier {
        CombinedVerifier::new()
    }
}

#[cfg(test)]
#[cfg(feature = "zkvm-mock")]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_proof_generator_keygen() {
        let temp_dir = TempDir::new().unwrap();
        let generator = ProofGenerator::mock(temp_dir.path());

        let cold_shard = [1u8; 32];
        let agent_shard = [2u8; 32];
        let ceremony_nonce = [3u8; 32];

        let output = generator
            .prove_keygen(&cold_shard, &agent_shard, &ceremony_nonce)
            .unwrap();

        assert_eq!(output.ceremony_nonce, ceremony_nonce);
        assert!(output.master_pubkey[0] == 0x02 || output.master_pubkey[0] == 0x03);
    }

    #[test]
    fn test_sample_indices_generation() {
        let indices = generate_sample_indices(100, 10, 42);

        // Should include first and last
        assert!(indices.contains(&0));
        assert!(indices.contains(&99));

        // Should be sorted and deduped
        for i in 1..indices.len() {
            assert!(indices[i] > indices[i - 1]);
        }
    }
}
