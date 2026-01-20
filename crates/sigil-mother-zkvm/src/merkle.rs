//! Merkle tree utilities for batch presignature proofs
//!
//! Provides efficient commitment to large batches of R points.

use sha2::{Digest, Sha256};

use crate::error::{Result, ZkvmError};

/// A simple Merkle tree for committing to a list of values
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// All nodes in the tree, level by level (leaves first)
    nodes: Vec<Vec<[u8; 32]>>,
    /// Number of leaves
    leaf_count: usize,
}

impl MerkleTree {
    /// Build a Merkle tree from a list of leaf values
    ///
    /// Each leaf is hashed as `SHA256(leaf)`.
    /// Internal nodes are `SHA256(left || right)`.
    pub fn from_leaves(leaves: &[[u8; 33]]) -> Result<Self> {
        if leaves.is_empty() {
            return Err(ZkvmError::MerkleTree(
                "Cannot create tree with no leaves".into(),
            ));
        }

        let mut nodes: Vec<Vec<[u8; 32]>> = Vec::new();

        // Hash leaves to get level 0
        let level0: Vec<[u8; 32]> = leaves
            .iter()
            .map(|leaf| {
                let mut hasher = Sha256::new();
                hasher.update(leaf);
                hasher.finalize().into()
            })
            .collect();

        nodes.push(level0);

        // Build up the tree
        while nodes.last().unwrap().len() > 1 {
            let current_level = nodes.last().unwrap();
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = &current_level[i];
                let right = if i + 1 < current_level.len() {
                    &current_level[i + 1]
                } else {
                    // Odd number of nodes: duplicate the last one
                    left
                };

                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);
                next_level.push(hasher.finalize().into());
            }

            nodes.push(next_level);
        }

        Ok(Self {
            nodes,
            leaf_count: leaves.len(),
        })
    }

    /// Get the Merkle root
    pub fn root(&self) -> [u8; 32] {
        self.nodes.last().unwrap()[0]
    }

    /// Get the number of leaves
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Generate a Merkle proof for a leaf at the given index
    pub fn proof(&self, index: usize) -> Result<Vec<[u8; 32]>> {
        if index >= self.leaf_count {
            return Err(ZkvmError::MerkleTree(format!(
                "Index {} out of range (tree has {} leaves)",
                index, self.leaf_count
            )));
        }

        let mut proof = Vec::new();
        let mut current_index = index;

        for level in &self.nodes[..self.nodes.len() - 1] {
            let sibling_index = if current_index.is_multiple_of(2) {
                // We're a left child, sibling is to the right
                if current_index + 1 < level.len() {
                    current_index + 1
                } else {
                    // No sibling (odd tree), use self
                    current_index
                }
            } else {
                // We're a right child, sibling is to the left
                current_index - 1
            };

            proof.push(level[sibling_index]);
            current_index /= 2;
        }

        Ok(proof)
    }

    /// Verify a Merkle proof
    pub fn verify_proof(
        root: &[u8; 32],
        leaf: &[u8; 33],
        index: usize,
        proof: &[[u8; 32]],
    ) -> bool {
        // Hash the leaf
        let mut current_hash: [u8; 32] = {
            let mut hasher = Sha256::new();
            hasher.update(leaf);
            hasher.finalize().into()
        };

        let mut current_index = index;

        for sibling in proof {
            let mut hasher = Sha256::new();
            if current_index.is_multiple_of(2) {
                // We're a left child
                hasher.update(current_hash);
                hasher.update(sibling);
            } else {
                // We're a right child
                hasher.update(sibling);
                hasher.update(current_hash);
            }
            current_hash = hasher.finalize().into();
            current_index /= 2;
        }

        current_hash == *root
    }
}

/// Compute a leaf hash for an R point
pub fn hash_r_point(r_point: &[u8; 33]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(r_point);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_single_leaf() {
        let leaves = vec![[0x02; 33]];
        let tree = MerkleTree::from_leaves(&leaves).unwrap();

        assert_eq!(tree.leaf_count(), 1);

        // Root should be hash of the single leaf
        let expected_root = hash_r_point(&leaves[0]);
        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn test_merkle_tree_two_leaves() {
        let leaves = vec![[0x02; 33], [0x03; 33]];
        let tree = MerkleTree::from_leaves(&leaves).unwrap();

        assert_eq!(tree.leaf_count(), 2);

        // Verify both leaves
        for i in 0..2 {
            let proof = tree.proof(i).unwrap();
            assert!(MerkleTree::verify_proof(
                &tree.root(),
                &leaves[i],
                i,
                &proof
            ));
        }
    }

    #[test]
    fn test_merkle_tree_many_leaves() {
        let leaves: Vec<[u8; 33]> = (0..100)
            .map(|i| {
                let mut leaf = [0x02; 33];
                leaf[0] = (i % 2 + 2) as u8;
                leaf[1] = i as u8;
                leaf
            })
            .collect();

        let tree = MerkleTree::from_leaves(&leaves).unwrap();

        // Verify random samples
        for i in [0, 1, 50, 99] {
            let proof = tree.proof(i).unwrap();
            assert!(
                MerkleTree::verify_proof(&tree.root(), &leaves[i], i, &proof),
                "Failed to verify leaf at index {}",
                i
            );
        }
    }

    #[test]
    fn test_merkle_proof_invalid() {
        let leaves = vec![[0x02; 33], [0x03; 33]];
        let tree = MerkleTree::from_leaves(&leaves).unwrap();

        let proof = tree.proof(0).unwrap();

        // Wrong leaf should fail
        let wrong_leaf = [0x04; 33];
        assert!(!MerkleTree::verify_proof(
            &tree.root(),
            &wrong_leaf,
            0,
            &proof
        ));

        // Wrong index should fail
        assert!(!MerkleTree::verify_proof(
            &tree.root(),
            &leaves[0],
            1,
            &proof
        ));
    }
}
