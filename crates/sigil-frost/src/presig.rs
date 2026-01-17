//! FROST presignature (nonce) storage types
//!
//! In FROST, "presignatures" are pre-generated nonces and commitments that
//! can be used in the signing round. Like ECDSA presignatures, each nonce
//! can only be used once - reusing a nonce would compromise the private key.

use crate::{FrostError, Result, SignatureScheme};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// A single FROST presignature (nonce + commitment)
///
/// The nonce is kept secret and used during signing.
/// The commitment is shared with other participants.
#[derive(Clone, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct FrostPresig {
    /// Index of this presignature
    pub index: u32,
    /// The secret nonce (must be kept private)
    pub nonce: Vec<u8>,
    /// The public commitment (can be shared)
    pub commitment: Vec<u8>,
}

impl FrostPresig {
    /// Create a new presignature
    pub fn new(index: u32, nonce: Vec<u8>, commitment: Vec<u8>) -> Self {
        Self {
            index,
            nonce,
            commitment,
        }
    }

    /// Mark this presignature as used (zeros out the nonce)
    pub fn consume(&mut self) {
        self.nonce.zeroize();
    }

    /// Check if this presignature has been consumed
    pub fn is_consumed(&self) -> bool {
        self.nonce.iter().all(|&b| b == 0)
    }
}

impl Drop for FrostPresig {
    fn drop(&mut self) {
        self.nonce.zeroize();
    }
}

/// A single FROST nonce (just the secret part)
///
/// Used for compact storage when commitments are stored separately.
#[derive(Clone, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct FrostNonce {
    /// Index of this nonce
    pub index: u32,
    /// The secret nonce bytes
    pub data: Vec<u8>,
}

impl FrostNonce {
    /// Create a new nonce
    pub fn new(index: u32, data: Vec<u8>) -> Self {
        Self { index, data }
    }
}

impl Drop for FrostNonce {
    fn drop(&mut self) {
        self.data.zeroize();
    }
}

/// A batch of FROST presignatures for storage on a child disk
#[derive(Clone, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct FrostPresigBatch {
    /// The signature scheme these presigs are for
    pub scheme: SignatureScheme,
    /// Participant identifier (1 for cold/mother, 2 for agent)
    pub participant_id: u16,
    /// Starting index of this batch
    pub start_index: u32,
    /// The presignatures
    pub presigs: Vec<FrostPresig>,
}

impl FrostPresigBatch {
    /// Create a new batch
    pub fn new(
        scheme: SignatureScheme,
        participant_id: u16,
        start_index: u32,
        presigs: Vec<FrostPresig>,
    ) -> Self {
        Self {
            scheme,
            participant_id,
            start_index,
            presigs,
        }
    }

    /// Get the number of presignatures in this batch
    pub fn len(&self) -> usize {
        self.presigs.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.presigs.is_empty()
    }

    /// Get a presignature by index
    pub fn get(&self, index: u32) -> Option<&FrostPresig> {
        if index < self.start_index {
            return None;
        }
        let offset = (index - self.start_index) as usize;
        self.presigs.get(offset)
    }

    /// Get a mutable presignature by index
    pub fn get_mut(&mut self, index: u32) -> Option<&mut FrostPresig> {
        if index < self.start_index {
            return None;
        }
        let offset = (index - self.start_index) as usize;
        self.presigs.get_mut(offset)
    }

    /// Get the next available (unconsumed) presignature
    pub fn next_available(&self) -> Option<&FrostPresig> {
        self.presigs.iter().find(|p| !p.is_consumed())
    }

    /// Count remaining (unconsumed) presignatures
    pub fn remaining(&self) -> usize {
        self.presigs.iter().filter(|p| !p.is_consumed()).count()
    }

    /// Consume and return the next available presignature
    pub fn consume_next(&mut self) -> Result<FrostPresig> {
        let idx = self
            .presigs
            .iter()
            .position(|p| !p.is_consumed())
            .ok_or(FrostError::NoPresigsRemaining)?;

        let presig = self.presigs[idx].clone();
        self.presigs[idx].consume();
        Ok(presig)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(bitcode::encode(self))
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bitcode::decode(data).map_err(|e| FrostError::Deserialization(e.to_string()))
    }
}

/// Commitment batch for the other participant
///
/// When creating a child disk, the mother generates presigs for both parties.
/// The cold nonces stay on the disk, while commitments are shared.
#[derive(Clone, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
pub struct FrostCommitmentBatch {
    /// The signature scheme
    pub scheme: SignatureScheme,
    /// Which participant these commitments belong to
    pub participant_id: u16,
    /// Starting index
    pub start_index: u32,
    /// The commitments (parallel to presig indices)
    pub commitments: Vec<Vec<u8>>,
}

impl FrostCommitmentBatch {
    /// Create a new commitment batch
    pub fn new(
        scheme: SignatureScheme,
        participant_id: u16,
        start_index: u32,
        commitments: Vec<Vec<u8>>,
    ) -> Self {
        Self {
            scheme,
            participant_id,
            start_index,
            commitments,
        }
    }

    /// Get a commitment by index
    pub fn get(&self, index: u32) -> Option<&[u8]> {
        if index < self.start_index {
            return None;
        }
        let offset = (index - self.start_index) as usize;
        self.commitments.get(offset).map(|v| v.as_slice())
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(bitcode::encode(self))
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        bitcode::decode(data).map_err(|e| FrostError::Deserialization(e.to_string()))
    }
}

/// Extract commitments from a presig batch
impl From<&FrostPresigBatch> for FrostCommitmentBatch {
    fn from(batch: &FrostPresigBatch) -> Self {
        Self {
            scheme: batch.scheme,
            participant_id: batch.participant_id,
            start_index: batch.start_index,
            commitments: batch.presigs.iter().map(|p| p.commitment.clone()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presig_consume() {
        let mut presig = FrostPresig::new(0, vec![1, 2, 3, 4], vec![5, 6, 7, 8]);
        assert!(!presig.is_consumed());

        presig.consume();
        assert!(presig.is_consumed());
    }

    #[test]
    fn test_batch_operations() {
        // Use i+1 to avoid all-zero nonces (which appear consumed)
        let presigs = (0..10)
            .map(|i| FrostPresig::new(i, vec![(i + 1) as u8; 32], vec![(i + 100) as u8; 32]))
            .collect();

        let mut batch = FrostPresigBatch::new(SignatureScheme::Taproot, 1, 0, presigs);

        assert_eq!(batch.len(), 10);
        assert_eq!(batch.remaining(), 10);

        // Consume first
        let p1 = batch.consume_next().unwrap();
        assert_eq!(p1.index, 0);
        assert_eq!(batch.remaining(), 9);

        // Consume second
        let p2 = batch.consume_next().unwrap();
        assert_eq!(p2.index, 1);
        assert_eq!(batch.remaining(), 8);

        // Get by index
        assert!(batch.get(0).unwrap().is_consumed());
        assert!(!batch.get(5).unwrap().is_consumed());
    }

    #[test]
    fn test_batch_serialization() {
        let presigs = vec![
            FrostPresig::new(0, vec![1; 32], vec![2; 33]),
            FrostPresig::new(1, vec![3; 32], vec![4; 33]),
        ];

        let batch = FrostPresigBatch::new(SignatureScheme::Ed25519, 1, 0, presigs);

        let bytes = batch.to_bytes().unwrap();
        let decoded = FrostPresigBatch::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.scheme, SignatureScheme::Ed25519);
        assert_eq!(decoded.len(), 2);
    }
}
