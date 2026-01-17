//! Presignature types and operations for MPC signing
//!
//! This module implements the presignature protocol for 2-of-2 ECDSA MPC.

use crate::error::{Result, SigilError};
use rand::RngCore;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

// Helper for serializing/deserializing [u8; 33]
fn serialize_bytes_33<S>(bytes: &[u8; 33], serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(bytes)
}

fn deserialize_bytes_33<'de, D>(deserializer: D) -> std::result::Result<[u8; 33], D::Error>
where
    D: Deserializer<'de>,
{
    let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
    bytes.as_slice()
        .try_into()
        .map_err(|_| serde::de::Error::custom("Expected 33 bytes"))
}

/// Status of a presignature share
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum PresigStatus {
    /// Fresh, unused presignature
    Fresh = 0,
    /// Used for signing
    Used = 1,
    /// Voided/invalidated
    Void = 2,
}

impl TryFrom<u8> for PresigStatus {
    type Error = SigilError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(PresigStatus::Fresh),
            1 => Ok(PresigStatus::Used),
            2 => Ok(PresigStatus::Void),
            _ => Err(SigilError::InvalidPresignature(
                format!("Invalid presig status: {}", value)
            )),
        }
    }
}

/// A presignature share for cold storage (floppy disk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresigColdShare {
    /// Nonce point R (compressed, 33 bytes)
    #[serde(serialize_with = "serialize_bytes_33", deserialize_with = "deserialize_bytes_33")]
    pub r_point: [u8; 33],
    
    /// Cold nonce share k_cold (32 bytes)
    pub k_cold: [u8; 32],
    
    /// Auxiliary value for signing (32 bytes)
    pub chi_cold: [u8; 32],
    
    /// Status of this presignature
    pub status: PresigStatus,
}

impl PresigColdShare {
    /// Size in bytes when serialized to disk (256 bytes fixed)
    pub const DISK_SIZE: usize = 256;

    /// Create a new cold share
    pub fn new(r_point: [u8; 33], k_cold: [u8; 32], chi_cold: [u8; 32]) -> Self {
        Self {
            r_point,
            k_cold,
            chi_cold,
            status: PresigStatus::Fresh,
        }
    }

    /// Mark this share as used
    pub fn mark_used(&mut self) {
        self.status = PresigStatus::Used;
    }

    /// Mark this share as void
    pub fn mark_void(&mut self) {
        self.status = PresigStatus::Void;
    }

    /// Check if this share is available for use
    pub fn is_available(&self) -> bool {
        self.status == PresigStatus::Fresh
    }

    /// Serialize to disk format (256 bytes fixed)
    pub fn to_disk_bytes(&self) -> [u8; Self::DISK_SIZE] {
        let mut bytes = [0u8; Self::DISK_SIZE];
        
        // R point (33 bytes)
        bytes[0..33].copy_from_slice(&self.r_point);
        
        // k_cold (32 bytes)
        bytes[33..65].copy_from_slice(&self.k_cold);
        
        // chi_cold (32 bytes)
        bytes[65..97].copy_from_slice(&self.chi_cold);
        
        // status (1 byte)
        bytes[97] = self.status as u8;
        
        // Reserved (158 bytes)
        // bytes[98..256] remain zeros
        
        bytes
    }

    /// Deserialize from disk format
    pub fn from_disk_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::DISK_SIZE {
            return Err(SigilError::InvalidPresignature(
                "Insufficient bytes for presignature".to_string()
            ));
        }

        let mut r_point = [0u8; 33];
        let mut k_cold = [0u8; 32];
        let mut chi_cold = [0u8; 32];

        r_point.copy_from_slice(&bytes[0..33]);
        k_cold.copy_from_slice(&bytes[33..65]);
        chi_cold.copy_from_slice(&bytes[65..97]);
        
        let status = PresigStatus::try_from(bytes[97])?;

        Ok(Self {
            r_point,
            k_cold,
            chi_cold,
            status,
        })
    }
}

/// A presignature share for agent storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresigAgentShare {
    /// Nonce point R (compressed, 33 bytes) - must match cold share
    #[serde(serialize_with = "serialize_bytes_33", deserialize_with = "deserialize_bytes_33")]
    pub r_point: [u8; 33],
    
    /// Agent nonce share k_agent (32 bytes)
    pub k_agent: [u8; 32],
    
    /// Auxiliary value for signing (32 bytes)
    pub chi_agent: [u8; 32],
}

impl PresigAgentShare {
    /// Create a new agent share
    pub fn new(r_point: [u8; 33], k_agent: [u8; 32], chi_agent: [u8; 32]) -> Self {
        Self {
            r_point,
            k_agent,
            chi_agent,
        }
    }
}

/// A complete presignature (cold + agent shares)
#[derive(Debug)]
pub struct Presignature {
    pub cold: PresigColdShare,
    pub agent: PresigAgentShare,
}

impl Presignature {
    /// Create a new presignature from both shares
    pub fn new(cold: PresigColdShare, agent: PresigAgentShare) -> Result<Self> {
        // Verify both shares reference the same R point
        if cold.r_point != agent.r_point {
            return Err(SigilError::InvalidPresignature(
                "R point mismatch between cold and agent shares".to_string()
            ));
        }

        Ok(Self { cold, agent })
    }

    /// Complete ECDSA signature using this presignature
    /// This is a simplified implementation - real MPC would use proper protocols
    pub fn complete_signature(
        &self,
        message_hash: &[u8; 32],
        _pubkey: &[u8; 33],
    ) -> Result<[u8; 64]> {
        // This is a simplified placeholder
        // In production, this would:
        // 1. Combine k_cold and k_agent to get k
        // 2. Compute r = x-coordinate of R
        // 3. Compute s = k^-1 * (hash + r * x) where x is the private key
        // 4. Return signature (r, s)

        // For now, we'll create a deterministic signature based on the inputs
        let mut hasher = Sha256::new();
        hasher.update(&self.cold.r_point);
        hasher.update(&self.cold.k_cold);
        hasher.update(&self.agent.k_agent);
        hasher.update(message_hash);
        
        let hash1 = hasher.finalize();
        
        let mut hasher2 = Sha256::new();
        hasher2.update(&hash1);
        hasher2.update(message_hash);
        let hash2 = hasher2.finalize();
        
        let mut signature = [0u8; 64];
        signature[..32].copy_from_slice(&hash1);
        signature[32..].copy_from_slice(&hash2);
        
        Ok(signature)
    }
}

/// Generate a pair of presignature shares (cold and agent)
pub fn generate_presig_pair<R: RngCore>(rng: &mut R) -> Result<(PresigColdShare, PresigAgentShare)> {
    // Generate random nonce shares
    let mut k_cold = [0u8; 32];
    let mut k_agent = [0u8; 32];
    let mut chi_cold = [0u8; 32];
    let mut chi_agent = [0u8; 32];
    
    rng.fill_bytes(&mut k_cold);
    rng.fill_bytes(&mut k_agent);
    rng.fill_bytes(&mut chi_cold);
    rng.fill_bytes(&mut chi_agent);
    
    // In a real MPC implementation, we would:
    // 1. Generate k_cold and k_agent as secret shares
    // 2. Compute R = k_cold * G + k_agent * G
    // 3. Both parties compute R without revealing their k values
    
    // For now, we'll create a deterministic R point from the k values
    let mut hasher = Sha256::new();
    hasher.update(&k_cold);
    hasher.update(&k_agent);
    let r_hash = hasher.finalize();
    
    // Create a compressed point representation (33 bytes)
    let mut r_point = [0u8; 33];
    r_point[0] = 0x02; // Compressed point prefix
    r_point[1..33].copy_from_slice(&r_hash);
    
    let cold = PresigColdShare::new(r_point, k_cold, chi_cold);
    let agent = PresigAgentShare::new(r_point, k_agent, chi_agent);
    
    Ok((cold, agent))
}

/// Generate multiple presignature pairs
pub fn generate_presig_batch<R: RngCore>(
    rng: &mut R,
    count: usize,
) -> Result<(Vec<PresigColdShare>, Vec<PresigAgentShare>)> {
    let mut cold_shares = Vec::with_capacity(count);
    let mut agent_shares = Vec::with_capacity(count);
    
    for _ in 0..count {
        let (cold, agent) = generate_presig_pair(rng)?;
        cold_shares.push(cold);
        agent_shares.push(agent);
    }
    
    Ok((cold_shares, agent_shares))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::thread_rng;

    #[test]
    fn test_presig_status() {
        assert_eq!(PresigStatus::try_from(0).unwrap(), PresigStatus::Fresh);
        assert_eq!(PresigStatus::try_from(1).unwrap(), PresigStatus::Used);
        assert_eq!(PresigStatus::try_from(2).unwrap(), PresigStatus::Void);
        assert!(PresigStatus::try_from(3).is_err());
    }

    #[test]
    fn test_presig_cold_share_serialization() {
        let r_point = [1u8; 33];
        let k_cold = [2u8; 32];
        let chi_cold = [3u8; 32];
        
        let share = PresigColdShare::new(r_point, k_cold, chi_cold);
        let bytes = share.to_disk_bytes();
        
        assert_eq!(bytes.len(), PresigColdShare::DISK_SIZE);
        
        let deserialized = PresigColdShare::from_disk_bytes(&bytes).unwrap();
        assert_eq!(share.r_point, deserialized.r_point);
        assert_eq!(share.k_cold, deserialized.k_cold);
        assert_eq!(share.chi_cold, deserialized.chi_cold);
        assert_eq!(share.status, deserialized.status);
    }

    #[test]
    fn test_presig_status_mutations() {
        let mut share = PresigColdShare::new([0u8; 33], [0u8; 32], [0u8; 32]);
        
        assert!(share.is_available());
        assert_eq!(share.status, PresigStatus::Fresh);
        
        share.mark_used();
        assert!(!share.is_available());
        assert_eq!(share.status, PresigStatus::Used);
        
        share.mark_void();
        assert!(!share.is_available());
        assert_eq!(share.status, PresigStatus::Void);
    }

    #[test]
    fn test_presig_pair_generation() {
        let mut rng = thread_rng();
        let (cold, agent) = generate_presig_pair(&mut rng).unwrap();
        
        assert_eq!(cold.r_point, agent.r_point);
        assert!(cold.is_available());
    }

    #[test]
    fn test_presig_batch_generation() {
        let mut rng = thread_rng();
        let count = 10;
        let (cold_shares, agent_shares) = generate_presig_batch(&mut rng, count).unwrap();
        
        assert_eq!(cold_shares.len(), count);
        assert_eq!(agent_shares.len(), count);
        
        for (cold, agent) in cold_shares.iter().zip(agent_shares.iter()) {
            assert_eq!(cold.r_point, agent.r_point);
        }
    }

    #[test]
    fn test_presignature_r_point_mismatch() {
        let cold = PresigColdShare::new([1u8; 33], [0u8; 32], [0u8; 32]);
        let agent = PresigAgentShare::new([2u8; 33], [0u8; 32], [0u8; 32]);
        
        let result = Presignature::new(cold, agent);
        assert!(result.is_err());
    }

    #[test]
    fn test_presignature_signature_completion() {
        let mut rng = thread_rng();
        let (cold, agent) = generate_presig_pair(&mut rng).unwrap();
        
        let presig = Presignature::new(cold, agent).unwrap();
        let message_hash = [42u8; 32];
        let pubkey = [3u8; 33];
        
        let signature = presig.complete_signature(&message_hash, &pubkey).unwrap();
        assert_eq!(signature.len(), 64);
    }
}
