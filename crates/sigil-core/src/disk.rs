//! Floppy disk format for MPC presignature storage
//!
//! This module defines the on-disk format for child floppy disks that store
//! presignature shares for MPC signing.

use crate::error::{Result, SigilError};
use crate::presig::PresigColdShare;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

// Helper for serializing/deserializing [u8; 64]
fn serialize_bytes_64<S>(bytes: &[u8; 64], serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(bytes)
}

fn deserialize_bytes_64<'de, D>(deserializer: D) -> std::result::Result<[u8; 64], D::Error>
where
    D: Deserializer<'de>,
{
    let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
    bytes.as_slice()
        .try_into()
        .map_err(|_| serde::de::Error::custom("Expected 64 bytes"))
}

/// Magic bytes identifying a Sigil disk
pub const MAGIC: &[u8; 8] = b"SIGILDSK";

/// Current disk format version
pub const VERSION: u32 = 1;

/// Maximum size for a floppy disk (1.44 MB)
pub const FLOPPY_SIZE_BYTES: u64 = 1_474_560;

/// Default number of presignatures per disk
pub const DEFAULT_PRESIG_COUNT: u32 = 1000;

/// Disk header size in bytes
pub const HEADER_SIZE: usize = 256;

/// Presignature table offset
pub const PRESIG_TABLE_OFFSET: usize = 0x0100;

/// Usage log offset (after presig table)
pub const USAGE_LOG_OFFSET: usize = PRESIG_TABLE_OFFSET + (DEFAULT_PRESIG_COUNT as usize * PresigColdShare::DISK_SIZE);

/// Disk expiry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskExpiry {
    /// Presigs cannot be used after this time
    pub expires_at: u64,
    
    /// Must reconcile with mother by this time
    pub reconciliation_deadline: u64,
    
    /// Max transactions before forced reconciliation
    pub max_uses_before_reconcile: u32,
    
    /// Current count since last reconciliation
    pub uses_since_reconcile: u32,
}

impl DiskExpiry {
    /// Create default expiry configuration
    pub fn default_config() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        const PRESIG_VALIDITY_DAYS: u64 = 30;
        const RECONCILIATION_DEADLINE_DAYS: u64 = 45;
        const MAX_USES_BEFORE_RECONCILE: u32 = 500;
        
        Self {
            expires_at: now + (PRESIG_VALIDITY_DAYS * 86400),
            reconciliation_deadline: now + (RECONCILIATION_DEADLINE_DAYS * 86400),
            max_uses_before_reconcile: MAX_USES_BEFORE_RECONCILE,
            uses_since_reconcile: 0,
        }
    }

    /// Check if disk has expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now > self.expires_at
    }

    /// Check if reconciliation is required
    pub fn needs_reconciliation(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now > self.reconciliation_deadline || 
        self.uses_since_reconcile >= self.max_uses_before_reconcile
    }

    /// Days until expiration
    pub fn days_until_expiry(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        ((self.expires_at as i64) - (now as i64)) / 86400
    }
}

/// Child disk status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChildStatus {
    Active,
    Suspended,
    Nullified,
}

/// Reason for nullification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NullificationReason {
    ManualRevocation,
    ReconciliationAnomaly,
    PresigMisuse,
    LostOrStolen,
    CompromisedAgent,
}

/// Disk header (256 bytes)
#[derive(Debug, Clone)]
pub struct DiskHeader {
    /// Magic bytes "SIGILDSK" (8 bytes)
    pub magic: [u8; 8],
    
    /// Format version (4 bytes)
    pub version: u32,
    
    /// Child ID (32 bytes - hash of pubkey)
    pub child_id: [u8; 32],
    
    /// Child public key (compressed, 33 bytes)
    pub child_pubkey: [u8; 33],
    
    /// HD derivation path (32 bytes)
    pub derivation_path: [u8; 32],
    
    /// Total presignatures on disk (4 bytes)
    pub presig_total: u32,
    
    /// Number of presigs used (4 bytes)
    pub presig_used: u32,
    
    /// Creation timestamp (8 bytes)
    pub created_at: u64,
    
    /// Expiry configuration
    pub expiry: DiskExpiry,
    
    /// Mother's signature over header (64 bytes)
    pub mother_signature: [u8; 64],
}

impl DiskHeader {
    /// Create a new disk header
    pub fn new(
        child_id: [u8; 32],
        child_pubkey: [u8; 33],
        derivation_path: [u8; 32],
        presig_total: u32,
        expiry: DiskExpiry,
    ) -> Result<Self> {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SigilError::DiskFormat(e.to_string()))?
            .as_secs();

        Ok(Self {
            magic: *MAGIC,
            version: VERSION,
            child_id,
            child_pubkey,
            derivation_path,
            presig_total,
            presig_used: 0,
            created_at,
            expiry,
            mother_signature: [0u8; 64],
        })
    }

    /// Serialize header to bytes (256 bytes)
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        let mut offset = 0;

        // Magic (8 bytes)
        bytes[offset..offset + 8].copy_from_slice(&self.magic);
        offset += 8;

        // Version (4 bytes)
        bytes[offset..offset + 4].copy_from_slice(&self.version.to_le_bytes());
        offset += 4;

        // Child ID (32 bytes)
        bytes[offset..offset + 32].copy_from_slice(&self.child_id);
        offset += 32;

        // Child pubkey (33 bytes)
        bytes[offset..offset + 33].copy_from_slice(&self.child_pubkey);
        offset += 33;

        // Derivation path (32 bytes)
        bytes[offset..offset + 32].copy_from_slice(&self.derivation_path);
        offset += 32;

        // Presig total (4 bytes)
        bytes[offset..offset + 4].copy_from_slice(&self.presig_total.to_le_bytes());
        offset += 4;

        // Presig used (4 bytes)
        bytes[offset..offset + 4].copy_from_slice(&self.presig_used.to_le_bytes());
        offset += 4;

        // Created at (8 bytes)
        bytes[offset..offset + 8].copy_from_slice(&self.created_at.to_le_bytes());
        offset += 8;

        // Expiry config
        bytes[offset..offset + 8].copy_from_slice(&self.expiry.expires_at.to_le_bytes());
        offset += 8;
        bytes[offset..offset + 8].copy_from_slice(&self.expiry.reconciliation_deadline.to_le_bytes());
        offset += 8;
        bytes[offset..offset + 4].copy_from_slice(&self.expiry.max_uses_before_reconcile.to_le_bytes());
        offset += 4;
        bytes[offset..offset + 4].copy_from_slice(&self.expiry.uses_since_reconcile.to_le_bytes());
        offset += 4;

        // Mother signature (64 bytes)
        bytes[offset..offset + 64].copy_from_slice(&self.mother_signature);
        // offset += 64; - not needed since this is the last field

        // Remaining bytes are reserved (should be at offset 181, 75 bytes remaining)

        bytes
    }

    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(SigilError::DiskFormat(
                "Insufficient bytes for header".to_string()
            ));
        }

        let mut offset = 0;

        // Magic
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[offset..offset + 8]);
        if &magic != MAGIC {
            return Err(SigilError::DiskFormat(
                "Invalid magic bytes".to_string()
            ));
        }
        offset += 8;

        // Version
        let version = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        // Child ID
        let mut child_id = [0u8; 32];
        child_id.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        // Child pubkey
        let mut child_pubkey = [0u8; 33];
        child_pubkey.copy_from_slice(&bytes[offset..offset + 33]);
        offset += 33;

        // Derivation path
        let mut derivation_path = [0u8; 32];
        derivation_path.copy_from_slice(&bytes[offset..offset + 32]);
        offset += 32;

        // Presig total
        let presig_total = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        // Presig used
        let presig_used = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        // Created at
        let created_at = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        // Expiry
        let expires_at = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let reconciliation_deadline = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let max_uses_before_reconcile = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let uses_since_reconcile = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let expiry = DiskExpiry {
            expires_at,
            reconciliation_deadline,
            max_uses_before_reconcile,
            uses_since_reconcile,
        };

        // Mother signature
        let mut mother_signature = [0u8; 64];
        mother_signature.copy_from_slice(&bytes[offset..offset + 64]);

        Ok(Self {
            magic,
            version,
            child_id,
            child_pubkey,
            derivation_path,
            presig_total,
            presig_used,
            created_at,
            expiry,
            mother_signature,
        })
    }

    /// Calculate hash of header for signing (excludes signature field)
    pub fn hash_for_signing(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        
        hasher.update(&self.magic);
        hasher.update(&self.version.to_le_bytes());
        hasher.update(&self.child_id);
        hasher.update(&self.child_pubkey);
        hasher.update(&self.derivation_path);
        hasher.update(&self.presig_total.to_le_bytes());
        hasher.update(&self.presig_used.to_le_bytes());
        hasher.update(&self.created_at.to_le_bytes());
        hasher.update(&self.expiry.expires_at.to_le_bytes());
        hasher.update(&self.expiry.reconciliation_deadline.to_le_bytes());
        hasher.update(&self.expiry.max_uses_before_reconcile.to_le_bytes());
        hasher.update(&self.expiry.uses_since_reconcile.to_le_bytes());
        
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }
}

/// Usage log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLogEntry {
    /// Presignature index used
    pub presig_index: u32,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Message hash that was signed
    pub message_hash: [u8; 32],
    
    /// Resulting signature
    #[serde(serialize_with = "serialize_bytes_64", deserialize_with = "deserialize_bytes_64")]
    pub signature: [u8; 64],
    
    /// Chain ID
    pub chain_id: u32,
    
    /// Transaction hash
    pub tx_hash: [u8; 32],
    
    /// zkProof hash (optional)
    pub zkproof_hash: Option<[u8; 32]>,
    
    /// Description
    pub description: String,
}

impl UsageLogEntry {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        bytes.extend_from_slice(&self.presig_index.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.message_hash);
        bytes.extend_from_slice(&self.signature);
        bytes.extend_from_slice(&self.chain_id.to_le_bytes());
        bytes.extend_from_slice(&self.tx_hash);
        
        // zkproof_hash (32 bytes or zeros if None)
        if let Some(hash) = self.zkproof_hash {
            bytes.extend_from_slice(&hash);
        } else {
            bytes.extend_from_slice(&[0u8; 32]);
        }
        
        // Description length and data
        let desc_bytes = self.description.as_bytes();
        let desc_len = desc_bytes.len().min(u16::MAX as usize) as u16;
        bytes.extend_from_slice(&desc_len.to_le_bytes());
        bytes.extend_from_slice(&desc_bytes[..desc_len as usize]);
        
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_expiry_default() {
        let expiry = DiskExpiry::default_config();
        assert!(!expiry.is_expired());
        assert!(expiry.days_until_expiry() > 0);
    }

    #[test]
    fn test_disk_header_serialization() {
        let child_id = [1u8; 32];
        let child_pubkey = [2u8; 33];
        let derivation_path = [3u8; 32];
        let expiry = DiskExpiry::default_config();
        
        let header = DiskHeader::new(
            child_id,
            child_pubkey,
            derivation_path,
            1000,
            expiry,
        ).unwrap();
        
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);
        
        let deserialized = DiskHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header.magic, deserialized.magic);
        assert_eq!(header.version, deserialized.version);
        assert_eq!(header.child_id, deserialized.child_id);
        assert_eq!(header.child_pubkey, deserialized.child_pubkey);
        assert_eq!(header.presig_total, deserialized.presig_total);
    }

    #[test]
    fn test_invalid_magic_bytes() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..8].copy_from_slice(b"WRONGMAG");
        
        let result = DiskHeader::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_header_hash_for_signing() {
        let child_id = [1u8; 32];
        let child_pubkey = [2u8; 33];
        let derivation_path = [3u8; 32];
        let expiry = DiskExpiry::default_config();
        
        let header = DiskHeader::new(
            child_id,
            child_pubkey,
            derivation_path,
            1000,
            expiry,
        ).unwrap();
        
        let hash = header.hash_for_signing();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_usage_log_entry_serialization() {
        let entry = UsageLogEntry {
            presig_index: 42,
            timestamp: 1234567890,
            message_hash: [1u8; 32],
            signature: [2u8; 64],
            chain_id: 1,
            tx_hash: [3u8; 32],
            zkproof_hash: Some([4u8; 32]),
            description: "Test transaction".to_string(),
        };
        
        let bytes = entry.to_bytes();
        assert!(!bytes.is_empty());
    }
}
