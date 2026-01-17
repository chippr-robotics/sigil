//! Master key generation
//!
//! Generates the master shards for both cold (mother) and agent sides.

use k256::{
    elliptic_curve::{rand_core::OsRng, Field, PrimeField},
    ProjectivePoint, Scalar,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use sigil_core::crypto::{DerivationPath, PublicKey};

use crate::error::{MotherError, Result};
use crate::storage::MasterShardData;

/// Master key generator
pub struct MasterKeyGenerator;

/// Output of master key generation
pub struct MasterKeyGenOutput {
    /// Cold master shard (stays on mother device)
    pub cold_master_shard: MasterShardData,

    /// Agent master shard (goes to agent device)
    /// This should be securely transferred and then zeroized
    pub agent_master_shard: [u8; 32],

    /// Combined master public key
    pub master_pubkey: PublicKey,
}

impl MasterKeyGenerator {
    /// Generate new master shards
    ///
    /// This creates a 2-of-2 split of the master private key.
    /// The cold shard stays on the mother device, and the agent
    /// shard is transferred to the agent.
    pub fn generate() -> Result<MasterKeyGenOutput> {
        // Generate random shards
        let mut cold_shard = [0u8; 32];
        let mut agent_shard = [0u8; 32];

        OsRng.fill_bytes(&mut cold_shard);
        OsRng.fill_bytes(&mut agent_shard);

        // Convert to scalars
        let cold_scalar = Scalar::from_repr(cold_shard.into());
        let agent_scalar = Scalar::from_repr(agent_shard.into());

        if cold_scalar.is_none().into() || agent_scalar.is_none().into() {
            // Extremely unlikely, retry
            cold_shard.zeroize();
            agent_shard.zeroize();
            return Err(MotherError::Crypto("Failed to generate valid scalars".to_string()));
        }

        let cold_scalar = cold_scalar.unwrap();
        let agent_scalar = agent_scalar.unwrap();

        // Compute public key points
        let cold_point = ProjectivePoint::GENERATOR * cold_scalar;
        let agent_point = ProjectivePoint::GENERATOR * agent_scalar;

        // Combined public key = cold_point + agent_point
        let combined_point = cold_point + agent_point;
        let combined_affine = combined_point.to_affine();

        // Encode as compressed public key
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        let encoded = combined_affine.to_encoded_point(true);
        let pubkey_bytes: [u8; 33] = encoded.as_bytes().try_into().map_err(|_| {
            MotherError::Crypto("Failed to encode public key".to_string())
        })?;

        let master_pubkey = PublicKey::new(pubkey_bytes);

        // Create cold master shard data
        let cold_master_shard = MasterShardData::new(cold_shard, pubkey_bytes);

        Ok(MasterKeyGenOutput {
            cold_master_shard,
            agent_master_shard: agent_shard,
            master_pubkey,
        })
    }

    /// Derive a child key pair from the master shards
    ///
    /// Both mother and agent must derive using the same path to get
    /// matching child shards.
    pub fn derive_child(
        master_shard: &[u8; 32],
        path: &DerivationPath,
    ) -> Result<([u8; 32], PublicKey)> {
        // Simplified HD derivation (not full BIP32, but deterministic)
        // In production, use proper SLIP-10 for secp256k1

        let path_bytes = path.to_bytes();

        // Derive child shard: child = HKDF(master, path)
        let mut hasher = Sha256::new();
        hasher.update(master_shard);
        hasher.update(&path_bytes);
        let child_shard_bytes: [u8; 32] = hasher.finalize().into();

        // Convert to scalar
        let child_scalar = Scalar::from_repr(child_shard_bytes.into());
        if child_scalar.is_none().into() {
            return Err(MotherError::Crypto("Invalid child scalar".to_string()));
        }
        let child_scalar = child_scalar.unwrap();

        // Compute child public key point
        let child_point = ProjectivePoint::GENERATOR * child_scalar;
        let child_affine = child_point.to_affine();

        use k256::elliptic_curve::sec1::ToEncodedPoint;
        let encoded = child_affine.to_encoded_point(true);
        let pubkey_bytes: [u8; 33] = encoded.as_bytes().try_into().map_err(|_| {
            MotherError::Crypto("Failed to encode child public key".to_string())
        })?;

        Ok((child_shard_bytes, PublicKey::new(pubkey_bytes)))
    }

    /// Combine two child public keys to get the full child public key
    pub fn combine_child_pubkeys(cold_pubkey: &PublicKey, agent_pubkey: &PublicKey) -> Result<PublicKey> {
        sigil_core::crypto::point_add(cold_pubkey, agent_pubkey)
            .map_err(|e| MotherError::Crypto(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_generation() {
        let output = MasterKeyGenerator::generate().unwrap();

        // Verify public key is valid
        assert_eq!(output.master_pubkey.as_bytes().len(), 33);

        // Verify shards are different
        assert_ne!(
            output.cold_master_shard.cold_master_shard,
            output.agent_master_shard
        );
    }

    #[test]
    fn test_child_derivation() {
        let master_shard = [42u8; 32];
        let path = DerivationPath::ethereum_hardened(0);

        let (child_shard, child_pubkey) =
            MasterKeyGenerator::derive_child(&master_shard, &path).unwrap();

        // Verify deterministic
        let (child_shard2, child_pubkey2) =
            MasterKeyGenerator::derive_child(&master_shard, &path).unwrap();

        assert_eq!(child_shard, child_shard2);
        assert_eq!(child_pubkey.as_bytes(), child_pubkey2.as_bytes());
    }
}
