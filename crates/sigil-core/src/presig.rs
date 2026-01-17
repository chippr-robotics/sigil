//! Presignature types for MPC signing

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::types::{hex_bytes_32, hex_bytes_33};

/// Status of a presignature slot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PresigStatus {
    /// Fresh, unused presignature
    Fresh = 0,
    /// Used in a signing operation
    Used = 1,
    /// Voided (e.g., due to anomaly detection)
    Voided = 2,
}

impl From<u8> for PresigStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => PresigStatus::Fresh,
            1 => PresigStatus::Used,
            2 => PresigStatus::Voided,
            _ => PresigStatus::Voided, // Default to voided for unknown values
        }
    }
}

/// Cold shard of a presignature (stored on floppy disk)
///
/// Layout (256 bytes total):
/// - R: 33 bytes (nonce commitment point, compressed)
/// - k_cold: 32 bytes (cold party's nonce share)
/// - chi_cold: 32 bytes (auxiliary value for signature completion)
/// - status: 1 byte
/// - reserved: 158 bytes
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct PresigColdShare {
    /// Nonce commitment point R (compressed, 33 bytes)
    /// Both parties must agree on R for signature to be valid
    #[serde(with = "hex_bytes_33")]
    pub r_point: [u8; 33],

    /// Cold party's nonce share k_cold
    /// k = k_cold + k_agent (mod n)
    #[zeroize(skip)]
    #[serde(with = "hex_bytes_32")]
    pub k_cold: [u8; 32],

    /// Auxiliary value chi_cold for signature completion
    /// Used in the final signature computation
    #[zeroize(skip)]
    #[serde(with = "hex_bytes_32")]
    pub chi_cold: [u8; 32],

    /// Status of this presignature
    #[zeroize(skip)]
    pub status: PresigStatus,
}

impl PresigColdShare {
    /// Size of a serialized presig entry on disk
    pub const DISK_SIZE: usize = 256;

    /// Create a new fresh presignature share
    pub fn new(r_point: [u8; 33], k_cold: [u8; 32], chi_cold: [u8; 32]) -> Self {
        Self {
            r_point,
            k_cold,
            chi_cold,
            status: PresigStatus::Fresh,
        }
    }

    /// Serialize to disk format (256 bytes)
    pub fn to_bytes(&self) -> [u8; Self::DISK_SIZE] {
        let mut bytes = [0u8; Self::DISK_SIZE];
        bytes[0..33].copy_from_slice(&self.r_point);
        bytes[33..65].copy_from_slice(&self.k_cold);
        bytes[65..97].copy_from_slice(&self.chi_cold);
        bytes[97] = self.status as u8;
        // Remaining 158 bytes are reserved (zeros)
        bytes
    }

    /// Deserialize from disk format
    pub fn from_bytes(bytes: &[u8; Self::DISK_SIZE]) -> Self {
        let mut r_point = [0u8; 33];
        let mut k_cold = [0u8; 32];
        let mut chi_cold = [0u8; 32];

        r_point.copy_from_slice(&bytes[0..33]);
        k_cold.copy_from_slice(&bytes[33..65]);
        chi_cold.copy_from_slice(&bytes[65..97]);
        let status = PresigStatus::from(bytes[97]);

        Self {
            r_point,
            k_cold,
            chi_cold,
            status,
        }
    }

    /// Check if this presignature is available for use
    pub fn is_fresh(&self) -> bool {
        self.status == PresigStatus::Fresh
    }

    /// Mark as used
    pub fn mark_used(&mut self) {
        self.status = PresigStatus::Used;
    }

    /// Mark as voided
    pub fn mark_voided(&mut self) {
        self.status = PresigStatus::Voided;
    }
}

impl core::fmt::Debug for PresigColdShare {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PresigColdShare")
            .field("r_point", &hex::encode(&self.r_point[..8]))
            .field("k_cold", &"[REDACTED]")
            .field("chi_cold", &"[REDACTED]")
            .field("status", &self.status)
            .finish()
    }
}

/// Agent shard of a presignature (stored on agent server)
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct PresigAgentShare {
    /// Nonce commitment point R (must match cold share)
    #[serde(with = "hex_bytes_33")]
    pub r_point: [u8; 33],

    /// Agent party's nonce share k_agent
    #[zeroize(skip)]
    #[serde(with = "hex_bytes_32")]
    pub k_agent: [u8; 32],

    /// Auxiliary value chi_agent for signature completion
    #[zeroize(skip)]
    #[serde(with = "hex_bytes_32")]
    pub chi_agent: [u8; 32],
}

impl PresigAgentShare {
    /// Create a new agent presignature share
    pub fn new(r_point: [u8; 33], k_agent: [u8; 32], chi_agent: [u8; 32]) -> Self {
        Self {
            r_point,
            k_agent,
            chi_agent,
        }
    }
}

impl core::fmt::Debug for PresigAgentShare {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PresigAgentShare")
            .field("r_point", &hex::encode(&self.r_point[..8]))
            .field("k_agent", &"[REDACTED]")
            .field("chi_agent", &"[REDACTED]")
            .finish()
    }
}

/// Combined presignature data for signing
#[derive(Debug, Clone)]
pub struct CombinedPresig {
    /// The shared R point
    pub r_point: [u8; 33],
    /// Combined nonce k = k_cold + k_agent
    pub k: [u8; 32],
    /// Combined chi value
    pub chi: [u8; 32],
}

/// Presig table entry with index information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresigTableEntry {
    /// Index in the presig table (0-999)
    pub index: u32,
    /// The cold share data
    pub cold_share: PresigColdShare,
}

impl PresigTableEntry {
    /// Create a new table entry
    pub fn new(index: u32, cold_share: PresigColdShare) -> Self {
        Self { index, cold_share }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presig_cold_share_roundtrip() {
        let share = PresigColdShare::new([1u8; 33], [2u8; 32], [3u8; 32]);
        let bytes = share.to_bytes();
        let recovered = PresigColdShare::from_bytes(&bytes);

        assert_eq!(share.r_point, recovered.r_point);
        assert_eq!(share.k_cold, recovered.k_cold);
        assert_eq!(share.chi_cold, recovered.chi_cold);
        assert_eq!(share.status, recovered.status);
    }

    #[test]
    fn test_presig_status_values() {
        assert_eq!(PresigStatus::Fresh as u8, 0);
        assert_eq!(PresigStatus::Used as u8, 1);
        assert_eq!(PresigStatus::Voided as u8, 2);
    }
}
