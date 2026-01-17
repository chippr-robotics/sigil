//! Usage log types for tracking signing operations

use serde::{Deserialize, Serialize};

use crate::types::{ChainId, MessageHash, Signature, TxHash, ZkProofHash};

/// Entry in the usage log on a floppy disk
///
/// Records each signing operation for audit purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLogEntry {
    /// Index of the presignature used
    pub presig_index: u32,

    /// Unix timestamp of the signing operation
    pub timestamp: u64,

    /// Hash of the message that was signed
    pub message_hash: MessageHash,

    /// The produced signature
    pub signature: Signature,

    /// Chain ID (for multi-chain support)
    pub chain_id: ChainId,

    /// Transaction hash (after broadcast)
    pub tx_hash: TxHash,

    /// Hash of the zkVM proof
    pub zkproof_hash: ZkProofHash,

    /// Human-readable description of the transaction
    pub description: String,
}

impl UsageLogEntry {
    /// Maximum description length
    pub const MAX_DESCRIPTION_LEN: usize = 256;

    /// Create a new usage log entry
    pub fn new(
        presig_index: u32,
        timestamp: u64,
        message_hash: MessageHash,
        signature: Signature,
        chain_id: ChainId,
        tx_hash: TxHash,
        zkproof_hash: ZkProofHash,
        description: String,
    ) -> Self {
        // Truncate description if too long
        let description = if description.len() > Self::MAX_DESCRIPTION_LEN {
            description[..Self::MAX_DESCRIPTION_LEN].to_string()
        } else {
            description
        };

        Self {
            presig_index,
            timestamp,
            message_hash,
            signature,
            chain_id,
            tx_hash,
            zkproof_hash,
            description,
        }
    }

    /// Serialize to bytes for disk storage
    /// Variable length due to description
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(256);

        // Fixed fields
        bytes.extend_from_slice(&self.presig_index.to_le_bytes()); // 4
        bytes.extend_from_slice(&self.timestamp.to_le_bytes()); // 8
        bytes.extend_from_slice(self.message_hash.as_bytes()); // 32
        bytes.extend_from_slice(self.signature.as_bytes()); // 64
        bytes.extend_from_slice(&self.chain_id.0.to_le_bytes()); // 4
        bytes.extend_from_slice(self.tx_hash.as_bytes()); // 32
        bytes.extend_from_slice(self.zkproof_hash.as_bytes()); // 32

        // Variable length description
        let desc_bytes = self.description.as_bytes();
        let desc_len = desc_bytes.len().min(Self::MAX_DESCRIPTION_LEN) as u16;
        bytes.extend_from_slice(&desc_len.to_le_bytes()); // 2
        bytes.extend_from_slice(&desc_bytes[..desc_len as usize]);

        bytes
    }

    /// Calculate the serialized size
    pub fn serialized_size(&self) -> usize {
        4 + 8 + 32 + 64 + 4 + 32 + 32 + 2 + self.description.len().min(Self::MAX_DESCRIPTION_LEN)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 178 {
            // Minimum size without description
            return None;
        }

        let presig_index = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
        let timestamp = u64::from_le_bytes(bytes[4..12].try_into().ok()?);

        let mut message_hash = [0u8; 32];
        message_hash.copy_from_slice(&bytes[12..44]);

        let mut signature = [0u8; 64];
        signature.copy_from_slice(&bytes[44..108]);

        let chain_id = u32::from_le_bytes(bytes[108..112].try_into().ok()?);

        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(&bytes[112..144]);

        let mut zkproof_hash = [0u8; 32];
        zkproof_hash.copy_from_slice(&bytes[144..176]);

        let desc_len = u16::from_le_bytes(bytes[176..178].try_into().ok()?) as usize;
        if bytes.len() < 178 + desc_len {
            return None;
        }

        let description = String::from_utf8_lossy(&bytes[178..178 + desc_len]).to_string();

        Some(Self {
            presig_index,
            timestamp,
            message_hash: MessageHash::new(message_hash),
            signature: Signature::new(signature),
            chain_id: ChainId::new(chain_id),
            tx_hash: TxHash::new(tx_hash),
            zkproof_hash: ZkProofHash::new(zkproof_hash),
            description,
        })
    }
}

/// Collection of usage log entries for a disk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageLog {
    /// All log entries
    pub entries: Vec<UsageLogEntry>,
}

impl UsageLog {
    /// Maximum number of entries (limited by disk space)
    /// ~1.1MB available, entries are ~200 bytes average
    pub const MAX_ENTRIES: usize = 5000;

    /// Create a new empty usage log
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a new entry
    pub fn push(&mut self, entry: UsageLogEntry) -> Result<(), crate::error::Error> {
        if self.entries.len() >= Self::MAX_ENTRIES {
            return Err(crate::error::Error::UsageLogFull);
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the last entry
    pub fn last(&self) -> Option<&UsageLogEntry> {
        self.entries.last()
    }

    /// Get entry by presig index
    pub fn find_by_presig_index(&self, index: u32) -> Option<&UsageLogEntry> {
        self.entries.iter().find(|e| e.presig_index == index)
    }

    /// Validate log integrity
    pub fn validate(&self) -> Result<(), crate::error::Error> {
        let mut last_index: Option<u32> = None;
        let mut last_timestamp: Option<u64> = None;

        for entry in &self.entries {
            // Check for index gaps or out-of-order indices
            if let Some(prev_idx) = last_index {
                if entry.presig_index <= prev_idx {
                    return Err(crate::error::Error::UsageLogAnomaly(format!(
                        "Non-monotonic presig index: {} after {}",
                        entry.presig_index, prev_idx
                    )));
                }
            }
            last_index = Some(entry.presig_index);

            // Check for timestamp ordering (allow some tolerance for clock skew)
            if let Some(prev_ts) = last_timestamp {
                if entry.timestamp + 3600 < prev_ts {
                    // Allow 1 hour backward
                    return Err(crate::error::Error::UsageLogAnomaly(format!(
                        "Timestamp out of order: {} after {}",
                        entry.timestamp, prev_ts
                    )));
                }
            }
            last_timestamp = Some(entry.timestamp);
        }

        Ok(())
    }

    /// Serialize all entries
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Entry count
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        // Each entry
        for entry in &self.entries {
            let entry_bytes = entry.to_bytes();
            bytes.extend_from_slice(&(entry_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&entry_bytes);
        }

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }

        let count = u32::from_le_bytes(bytes[0..4].try_into().ok()?) as usize;
        let mut entries = Vec::with_capacity(count);
        let mut offset = 4;

        for _ in 0..count {
            if offset + 4 > bytes.len() {
                return None;
            }
            let entry_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
            offset += 4;

            if offset + entry_len > bytes.len() {
                return None;
            }
            let entry = UsageLogEntry::from_bytes(&bytes[offset..offset + entry_len])?;
            entries.push(entry);
            offset += entry_len;
        }

        Some(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_log_entry_roundtrip() {
        let entry = UsageLogEntry::new(
            42,
            1700000000,
            MessageHash::new([1u8; 32]),
            Signature::new([2u8; 64]),
            ChainId::ETHEREUM,
            TxHash::new([3u8; 32]),
            ZkProofHash::new([4u8; 32]),
            "Test transaction".to_string(),
        );

        let bytes = entry.to_bytes();
        let recovered = UsageLogEntry::from_bytes(&bytes).unwrap();

        assert_eq!(entry.presig_index, recovered.presig_index);
        assert_eq!(entry.timestamp, recovered.timestamp);
        assert_eq!(entry.description, recovered.description);
    }

    #[test]
    fn test_usage_log_validation() {
        let mut log = UsageLog::new();

        // Add entries in order
        log.push(UsageLogEntry::new(
            0,
            1000,
            MessageHash::new([0u8; 32]),
            Signature::new([0u8; 64]),
            ChainId::ETHEREUM,
            TxHash::new([0u8; 32]),
            ZkProofHash::new([0u8; 32]),
            "First".to_string(),
        ))
        .unwrap();

        log.push(UsageLogEntry::new(
            1,
            2000,
            MessageHash::new([0u8; 32]),
            Signature::new([0u8; 64]),
            ChainId::ETHEREUM,
            TxHash::new([0u8; 32]),
            ZkProofHash::new([0u8; 32]),
            "Second".to_string(),
        ))
        .unwrap();

        assert!(log.validate().is_ok());
    }
}
