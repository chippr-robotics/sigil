//! SP1 zkVM program for batch presignature generation
//!
//! Proves: `R_i = (k_cold_i + k_agent_i)*G` for a batch of presignatures
//!
//! This program uses a Merkle tree to efficiently commit to large batches.
//! It proves:
//! 1. All R points are correctly computed from nonce shares
//! 2. The Merkle root commits to all R points
//! 3. Sampled R points have valid Merkle proofs

#![no_main]
sp1_zkvm::entrypoint!(main);

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use k256::{
    elliptic_curve::{sec1::ToEncodedPoint, PrimeField},
    ProjectivePoint, Scalar,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

// ============================================================================
// Types
// ============================================================================

mod hex_bytes_32 {
    use super::*;

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let mut bytes = [0u8; 32];
            hex::decode_to_slice(&s, &mut bytes).map_err(D::Error::custom)?;
            Ok(bytes)
        } else {
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            bytes
                .try_into()
                .map_err(|_| D::Error::custom("expected 32 bytes"))
        }
    }
}

mod hex_bytes_33 {
    use super::*;

    pub fn serialize<S>(bytes: &[u8; 33], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 33], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let mut bytes = [0u8; 33];
            hex::decode_to_slice(&s, &mut bytes).map_err(D::Error::custom)?;
            Ok(bytes)
        } else {
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            bytes
                .try_into()
                .map_err(|_| D::Error::custom("expected 33 bytes"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPresigInput {
    #[serde(with = "hex_bytes_32")]
    pub cold_child_shard: [u8; 32],
    #[serde(with = "hex_bytes_32")]
    pub agent_child_shard: [u8; 32],
    pub k_colds: Vec<[u8; 32]>,
    pub k_agents: Vec<[u8; 32]>,
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],
    pub start_index: u32,
    pub batch_size: u32,
    pub sample_indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPresigOutput {
    #[serde(with = "hex_bytes_32")]
    pub r_points_merkle_root: [u8; 32],
    #[serde(with = "hex_bytes_33")]
    pub first_r_point: [u8; 33],
    #[serde(with = "hex_bytes_33")]
    pub last_r_point: [u8; 33],
    pub sampled_r_points: Vec<SampledRPoint>,
    pub batch_size: u32,
    pub start_index: u32,
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampledRPoint {
    #[serde(with = "hex_bytes_33")]
    pub r_point: [u8; 33],
    pub index: u32,
    pub merkle_proof: Vec<[u8; 32]>,
}

// ============================================================================
// Main Program
// ============================================================================

pub fn main() {
    // Read the input from the prover
    let input: BatchPresigInput = sp1_zkvm::io::read();

    // Compute the output
    let output = compute_batch(&input).expect("Batch computation failed");

    // Commit the output (public)
    sp1_zkvm::io::commit(&output);
}

fn compute_batch(input: &BatchPresigInput) -> Result<BatchPresigOutput, &'static str> {
    let batch_size = input.batch_size as usize;

    // Validate input
    if input.k_colds.len() != batch_size {
        return Err("k_colds length mismatch");
    }
    if input.k_agents.len() != batch_size {
        return Err("k_agents length mismatch");
    }

    // Compute all R points
    let mut r_points: Vec<[u8; 33]> = Vec::with_capacity(batch_size);

    for i in 0..batch_size {
        let r_point = compute_r_point(&input.k_colds[i], &input.k_agents[i])?;
        r_points.push(r_point);
    }

    // Build Merkle tree
    let (merkle_root, tree_levels) = build_merkle_tree(&r_points)?;

    // Get first and last R points
    let first_r_point = r_points[0];
    let last_r_point = r_points[batch_size - 1];

    // Generate sampled R points with proofs
    let mut sampled_r_points = Vec::new();
    for &sample_idx in &input.sample_indices {
        if sample_idx >= input.batch_size {
            return Err("Sample index out of range");
        }

        let idx = sample_idx as usize;
        let proof = generate_merkle_proof(&tree_levels, idx);

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

fn compute_r_point(k_cold: &[u8; 32], k_agent: &[u8; 32]) -> Result<[u8; 33], &'static str> {
    let k_cold_scalar = Scalar::from_repr((*k_cold).into());
    if k_cold_scalar.is_none().into() {
        return Err("Invalid k_cold scalar");
    }
    let k_cold_scalar = k_cold_scalar.unwrap();

    let k_agent_scalar = Scalar::from_repr((*k_agent).into());
    if k_agent_scalar.is_none().into() {
        return Err("Invalid k_agent scalar");
    }
    let k_agent_scalar = k_agent_scalar.unwrap();

    let k_combined = k_cold_scalar + k_agent_scalar;
    let r_point = ProjectivePoint::GENERATOR * k_combined;
    let r_affine = r_point.to_affine();

    let r_bytes: [u8; 33] = r_affine
        .to_encoded_point(true)
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode R point")?;

    Ok(r_bytes)
}

fn hash_leaf(r_point: &[u8; 33]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(r_point);
    hasher.finalize().into()
}

fn build_merkle_tree(r_points: &[[u8; 33]]) -> Result<([u8; 32], Vec<Vec<[u8; 32]>>), &'static str> {
    if r_points.is_empty() {
        return Err("Cannot create tree with no leaves");
    }

    let mut levels: Vec<Vec<[u8; 32]>> = Vec::new();

    // Level 0: hash leaves
    let level0: Vec<[u8; 32]> = r_points.iter().map(|r| hash_leaf(r)).collect();
    levels.push(level0);

    // Build up the tree
    while levels.last().unwrap().len() > 1 {
        let current = levels.last().unwrap();
        let mut next = Vec::new();

        for i in (0..current.len()).step_by(2) {
            let left = &current[i];
            let right = if i + 1 < current.len() {
                &current[i + 1]
            } else {
                left
            };

            let mut hasher = Sha256::new();
            hasher.update(left);
            hasher.update(right);
            next.push(hasher.finalize().into());
        }

        levels.push(next);
    }

    let root = levels.last().unwrap()[0];
    Ok((root, levels))
}

fn generate_merkle_proof(levels: &[Vec<[u8; 32]>], index: usize) -> Vec<[u8; 32]> {
    let mut proof = Vec::new();
    let mut current_index = index;

    for level in &levels[..levels.len() - 1] {
        let sibling_index = if current_index % 2 == 0 {
            if current_index + 1 < level.len() {
                current_index + 1
            } else {
                current_index
            }
        } else {
            current_index - 1
        };

        proof.push(level[sibling_index]);
        current_index /= 2;
    }

    proof
}
