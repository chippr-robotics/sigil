//! Mock provers for testing without SP1 toolchain
//!
//! These provers perform the actual computation but generate "mock" proofs
//! that are not cryptographically verifiable. Use for development and testing.

use crate::error::Result;
use crate::provers::{
    BatchPresigProver, BatchPresigProverTrait, DeriveProver, DeriveProverTrait, HardwareProver,
    HardwareProverTrait, KeygenProver, KeygenProverTrait,
};
use crate::types::*;

/// Mock proof header for identification
const MOCK_PROOF_HEADER: &[u8] = b"SIGIL_MOCK_PROOF_V1";

/// Mock keygen prover
pub struct MockKeygenProver;

impl KeygenProverTrait for MockKeygenProver {
    fn prove(&self, input: KeygenInput) -> Result<(KeygenOutput, Vec<u8>)> {
        // Compute the actual output
        let output = KeygenProver::compute(&input)?;

        // Generate mock proof
        let proof = create_mock_proof(ProofType::Keygen, &output)?;

        Ok((output, proof))
    }
}

/// Mock derive prover
pub struct MockDeriveProver;

impl DeriveProverTrait for MockDeriveProver {
    fn prove(&self, input: DeriveInput) -> Result<(DeriveOutput, Vec<u8>)> {
        // Compute the actual output
        let output = DeriveProver::compute(&input)?;

        // Generate mock proof
        let proof = create_mock_proof(ProofType::Derive, &output)?;

        Ok((output, proof))
    }
}

/// Mock batch presig prover
pub struct MockBatchPresigProver;

impl BatchPresigProverTrait for MockBatchPresigProver {
    fn prove(&self, input: BatchPresigInput) -> Result<(BatchPresigOutput, Vec<u8>)> {
        // Compute the actual output
        let output = BatchPresigProver::compute(&input)?;

        // Generate mock proof
        let proof = create_mock_proof(ProofType::BatchPresig, &output)?;

        Ok((output, proof))
    }
}

/// Mock hardware prover
pub struct MockHardwareProver;

impl HardwareProverTrait for MockHardwareProver {
    fn prove(&self, input: HardwareInput) -> Result<(HardwareOutput, Vec<u8>)> {
        // Compute the actual output
        let output = HardwareProver::compute(&input)?;

        // Generate mock proof
        let proof = create_mock_proof(ProofType::Hardware, &output)?;

        Ok((output, proof))
    }
}

/// Create a mock proof containing the proof type and serialized output
fn create_mock_proof<T: serde::Serialize>(proof_type: ProofType, output: &T) -> Result<Vec<u8>> {
    use sha2::{Digest, Sha256};

    let output_json = serde_json::to_vec(output)?;

    // Mock proof structure:
    // - Header (19 bytes)
    // - Proof type (1 byte)
    // - Output hash (32 bytes)
    // - Output JSON length (4 bytes)
    // - Output JSON
    let mut proof = Vec::new();

    // Header
    proof.extend_from_slice(MOCK_PROOF_HEADER);

    // Proof type
    proof.push(match proof_type {
        ProofType::Keygen => 0,
        ProofType::Derive => 1,
        ProofType::BatchPresig => 2,
        ProofType::Hardware => 3,
    });

    // Output hash
    let mut hasher = Sha256::new();
    hasher.update(&output_json);
    proof.extend_from_slice(&hasher.finalize());

    // Output JSON length
    proof.extend_from_slice(&(output_json.len() as u32).to_le_bytes());

    // Output JSON
    proof.extend_from_slice(&output_json);

    Ok(proof)
}

/// Check if a proof is a mock proof
pub fn is_mock_proof(proof: &[u8]) -> bool {
    proof.len() >= MOCK_PROOF_HEADER.len()
        && &proof[..MOCK_PROOF_HEADER.len()] == MOCK_PROOF_HEADER
}

/// Extract the proof type from a mock proof
pub fn mock_proof_type(proof: &[u8]) -> Option<ProofType> {
    if !is_mock_proof(proof) || proof.len() < MOCK_PROOF_HEADER.len() + 1 {
        return None;
    }

    match proof[MOCK_PROOF_HEADER.len()] {
        0 => Some(ProofType::Keygen),
        1 => Some(ProofType::Derive),
        2 => Some(ProofType::BatchPresig),
        3 => Some(ProofType::Hardware),
        _ => None,
    }
}

/// Extract the output from a mock proof
pub fn extract_mock_output<T: serde::de::DeserializeOwned>(proof: &[u8]) -> Result<T> {
    use crate::error::ZkvmError;

    if !is_mock_proof(proof) {
        return Err(ZkvmError::ProofVerificationFailed(
            "Not a mock proof".into(),
        ));
    }

    let header_len = MOCK_PROOF_HEADER.len();
    if proof.len() < header_len + 1 + 32 + 4 {
        return Err(ZkvmError::ProofVerificationFailed(
            "Mock proof too short".into(),
        ));
    }

    // Skip header + type + hash
    let len_offset = header_len + 1 + 32;
    let len_bytes: [u8; 4] = proof[len_offset..len_offset + 4]
        .try_into()
        .map_err(|_| ZkvmError::ProofVerificationFailed("Invalid length".into()))?;
    let output_len = u32::from_le_bytes(len_bytes) as usize;

    let output_offset = len_offset + 4;
    if proof.len() < output_offset + output_len {
        return Err(ZkvmError::ProofVerificationFailed(
            "Mock proof truncated".into(),
        ));
    }

    let output: T = serde_json::from_slice(&proof[output_offset..output_offset + output_len])?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_keygen_prover() {
        let prover = MockKeygenProver;

        let input = KeygenInput {
            cold_shard: [1u8; 32],
            agent_shard: [2u8; 32],
            ceremony_nonce: [3u8; 32],
        };

        let (output, proof) = prover.prove(input).unwrap();

        // Verify proof is a mock proof
        assert!(is_mock_proof(&proof));
        assert_eq!(mock_proof_type(&proof), Some(ProofType::Keygen));

        // Extract output from proof
        let extracted: KeygenOutput = extract_mock_output(&proof).unwrap();
        assert_eq!(extracted, output);
    }

    #[test]
    fn test_mock_batch_presig_prover() {
        let prover = MockBatchPresigProver;

        let mut k_colds = Vec::new();
        let mut k_agents = Vec::new();
        for i in 0..10u8 {
            let mut k = [0u8; 32];
            k[0] = i;
            k_colds.push(k);
            k[1] = 1;
            k_agents.push(k);
        }

        let input = BatchPresigInput {
            cold_child_shard: [1u8; 32],
            agent_child_shard: [2u8; 32],
            k_colds,
            k_agents,
            child_pubkey: [0x02; 33],
            start_index: 0,
            batch_size: 10,
            sample_indices: vec![0, 5, 9],
        };

        let (output, proof) = prover.prove(input).unwrap();

        assert!(is_mock_proof(&proof));
        assert_eq!(mock_proof_type(&proof), Some(ProofType::BatchPresig));

        let extracted: BatchPresigOutput = extract_mock_output(&proof).unwrap();
        assert_eq!(extracted, output);
    }

    #[test]
    fn test_not_mock_proof() {
        let fake_proof = b"not a mock proof";
        assert!(!is_mock_proof(fake_proof));
        assert_eq!(mock_proof_type(fake_proof), None);
    }
}
