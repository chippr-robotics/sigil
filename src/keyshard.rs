//! Keyshard management and cryptographic operations
//!
//! This module handles the creation, storage, and reconstruction of keyshards
//! used in multi-party computation (MPC) schemes.

use crate::error::{Result, SigilError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Represents a single keyshard in an MPC scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Keyshard {
    /// Unique identifier for this keyshard
    pub id: String,
    
    /// The shard index (e.g., 1 of 3)
    pub index: u32,
    
    /// Total number of shards required for reconstruction
    pub threshold: u32,
    
    /// The actual key material (encrypted)
    pub data: Vec<u8>,
    
    /// Metadata about the keyshard
    pub metadata: KeyshardMetadata,
}

/// Metadata associated with a keyshard
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyshardMetadata {
    /// Creation timestamp
    pub created_at: u64,
    
    /// Purpose or description
    pub purpose: String,
    
    /// Associated blockchain address (if any)
    pub blockchain_address: Option<String>,
    
    /// Checksum for integrity verification
    pub checksum: String,
}

impl Keyshard {
    /// Create a new keyshard
    pub fn new(
        id: String,
        index: u32,
        threshold: u32,
        data: Vec<u8>,
        purpose: String,
        blockchain_address: Option<String>,
    ) -> Result<Self> {
        if index == 0 || index > threshold {
            return Err(SigilError::InvalidKeyshard(
                format!("Invalid index: {} (threshold: {})", index, threshold)
            ));
        }

        let checksum = Self::calculate_checksum(&data);
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SigilError::Keyshard(e.to_string()))?
            .as_secs();

        Ok(Self {
            id,
            index,
            threshold,
            data,
            metadata: KeyshardMetadata {
                created_at,
                purpose,
                blockchain_address,
                checksum,
            },
        })
    }

    /// Calculate a SHA256 checksum of the key data
    fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Verify the integrity of this keyshard
    pub fn verify_integrity(&self) -> Result<bool> {
        let calculated = Self::calculate_checksum(&self.data);
        Ok(calculated == self.metadata.checksum)
    }

    /// Serialize this keyshard to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }

    /// Deserialize a keyshard from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }

    /// Serialize to binary format
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }

    /// Deserialize from binary format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }
}

/// Keyshard collection manager
#[derive(Debug, Default)]
pub struct KeyshardCollection {
    shards: Vec<Keyshard>,
}

impl KeyshardCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self { shards: Vec::new() }
    }

    /// Add a keyshard to the collection
    pub fn add(&mut self, shard: Keyshard) -> Result<()> {
        // Verify integrity before adding
        if !shard.verify_integrity()? {
            return Err(SigilError::InvalidKeyshard(
                "Checksum verification failed".to_string()
            ));
        }
        self.shards.push(shard);
        Ok(())
    }

    /// Get all keyshards
    pub fn shards(&self) -> &[Keyshard] {
        &self.shards
    }

    /// Check if we have enough shards to meet a threshold
    pub fn has_threshold(&self, threshold: u32) -> bool {
        self.shards.len() >= threshold as usize
    }

    /// Get the number of shards
    pub fn count(&self) -> usize {
        self.shards.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyshard_creation() {
        let data = b"test key data".to_vec();
        let shard = Keyshard::new(
            "test-id".to_string(),
            1,
            3,
            data,
            "Test keyshard".to_string(),
            Some("0x1234".to_string()),
        );
        assert!(shard.is_ok());
        let shard = shard.unwrap();
        assert_eq!(shard.index, 1);
        assert_eq!(shard.threshold, 3);
    }

    #[test]
    fn test_invalid_index() {
        let data = b"test key data".to_vec();
        let shard = Keyshard::new(
            "test-id".to_string(),
            0,
            3,
            data,
            "Test keyshard".to_string(),
            None,
        );
        assert!(shard.is_err());
    }

    #[test]
    fn test_keyshard_integrity() {
        let data = b"test key data".to_vec();
        let shard = Keyshard::new(
            "test-id".to_string(),
            1,
            3,
            data,
            "Test keyshard".to_string(),
            None,
        ).unwrap();
        
        assert!(shard.verify_integrity().unwrap());
    }

    #[test]
    fn test_keyshard_serialization() {
        let data = b"test key data".to_vec();
        let shard = Keyshard::new(
            "test-id".to_string(),
            1,
            3,
            data,
            "Test keyshard".to_string(),
            None,
        ).unwrap();

        // Test JSON serialization
        let json = shard.to_json().unwrap();
        let deserialized = Keyshard::from_json(&json).unwrap();
        assert_eq!(shard, deserialized);

        // Test binary serialization
        let bytes = shard.to_bytes().unwrap();
        let deserialized = Keyshard::from_bytes(&bytes).unwrap();
        assert_eq!(shard, deserialized);
    }

    #[test]
    fn test_keyshard_collection() {
        let mut collection = KeyshardCollection::new();
        
        let shard1 = Keyshard::new(
            "id1".to_string(),
            1,
            3,
            b"data1".to_vec(),
            "Purpose".to_string(),
            None,
        ).unwrap();

        let shard2 = Keyshard::new(
            "id2".to_string(),
            2,
            3,
            b"data2".to_vec(),
            "Purpose".to_string(),
            None,
        ).unwrap();

        collection.add(shard1).unwrap();
        collection.add(shard2).unwrap();

        assert_eq!(collection.count(), 2);
        assert!(!collection.has_threshold(3));
        assert!(collection.has_threshold(2));
    }
}
