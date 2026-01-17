//! Presignature generation
//!
//! Generates presignature shares for both cold (disk) and agent sides.

use k256::{
    elliptic_curve::{rand_core::OsRng, PrimeField},
    ProjectivePoint, Scalar,
};
use rand::RngCore;

use sigil_core::presig::{PresigAgentShare, PresigColdShare};

use crate::error::{MotherError, Result};

/// Presignature generator
pub struct PresigGenerator;

/// Output of presignature generation for a single presig
pub struct PresigPair {
    /// Cold share (goes to floppy disk)
    pub cold_share: PresigColdShare,

    /// Agent share (goes to agent)
    pub agent_share: PresigAgentShare,
}

impl PresigGenerator {
    /// Generate a batch of presignatures
    ///
    /// This generates matching cold and agent shares that can be used
    /// together to produce valid ECDSA signatures.
    pub fn generate_batch(
        cold_child_shard: &[u8; 32],
        agent_child_shard: &[u8; 32],
        count: usize,
    ) -> Result<Vec<PresigPair>> {
        let mut pairs = Vec::with_capacity(count);

        for _ in 0..count {
            let pair = Self::generate_single(cold_child_shard, agent_child_shard)?;
            pairs.push(pair);
        }

        Ok(pairs)
    }

    /// Generate a single presignature pair
    fn generate_single(
        cold_child_shard: &[u8; 32],
        agent_child_shard: &[u8; 32],
    ) -> Result<PresigPair> {
        // Generate random nonce shares
        let mut k_cold_bytes = [0u8; 32];
        let mut k_agent_bytes = [0u8; 32];

        OsRng.fill_bytes(&mut k_cold_bytes);
        OsRng.fill_bytes(&mut k_agent_bytes);

        // Convert to scalars
        let k_cold = Scalar::from_repr(k_cold_bytes.into());
        let k_agent = Scalar::from_repr(k_agent_bytes.into());

        if k_cold.is_none().into() || k_agent.is_none().into() {
            return Err(MotherError::PresigGenerationFailed(
                "Invalid nonce scalar".to_string(),
            ));
        }

        let k_cold = k_cold.unwrap();
        let k_agent = k_agent.unwrap();

        // Compute R = (k_cold + k_agent) * G
        let k_combined = k_cold + k_agent;
        let r_point = ProjectivePoint::GENERATOR * k_combined;
        let r_affine = r_point.to_affine();

        // Encode R point
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        let r_encoded = r_affine.to_encoded_point(true);
        let r_bytes: [u8; 33] = r_encoded.as_bytes().try_into().map_err(|_| {
            MotherError::PresigGenerationFailed("Failed to encode R point".to_string())
        })?;

        // Compute chi values (private key share contribution)
        // chi_cold = cold_child_shard (as scalar)
        // chi_agent = agent_child_shard (as scalar)
        // These are used in the final signature computation

        let chi_cold = Scalar::from_repr((*cold_child_shard).into());
        let chi_agent = Scalar::from_repr((*agent_child_shard).into());

        if chi_cold.is_none().into() || chi_agent.is_none().into() {
            return Err(MotherError::PresigGenerationFailed(
                "Invalid chi scalar".to_string(),
            ));
        }

        let chi_cold_bytes: [u8; 32] = chi_cold.unwrap().to_bytes().into();
        let chi_agent_bytes: [u8; 32] = chi_agent.unwrap().to_bytes().into();

        // Create shares
        let cold_share = PresigColdShare::new(r_bytes, k_cold_bytes, chi_cold_bytes);

        let agent_share = PresigAgentShare::new(r_bytes, k_agent_bytes, chi_agent_bytes);

        Ok(PresigPair {
            cold_share,
            agent_share,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presig_generation() {
        let cold_shard = [1u8; 32];
        let agent_shard = [2u8; 32];

        let pair = PresigGenerator::generate_single(&cold_shard, &agent_shard).unwrap();

        // R points should match
        assert_eq!(pair.cold_share.r_point, pair.agent_share.r_point);

        // Shares should be different
        assert_ne!(pair.cold_share.k_cold, pair.agent_share.k_agent);
    }

    #[test]
    fn test_batch_generation() {
        let cold_shard = [1u8; 32];
        let agent_shard = [2u8; 32];

        let pairs = PresigGenerator::generate_batch(&cold_shard, &agent_shard, 10).unwrap();

        assert_eq!(pairs.len(), 10);

        // All R points within each pair should match
        for pair in &pairs {
            assert_eq!(pair.cold_share.r_point, pair.agent_share.r_point);
        }

        // R points between pairs should be different (with overwhelming probability)
        assert_ne!(pairs[0].cold_share.r_point, pairs[1].cold_share.r_point);
    }
}
