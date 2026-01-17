//! Mother device storage
//!
//! Manages persistent storage for the mother device's master shard
//! and child registry.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{MotherError, Result};
use crate::registry::ChildRegistry;
use sigil_core::types::{hex_bytes_32, hex_bytes_33};

/// Mother device storage
pub struct MotherStorage {
    /// Base path for storage
    base_path: PathBuf,
}

/// Master shard data (encrypted at rest in production)
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct MasterShardData {
    /// The cold master shard (32 bytes)
    #[serde(with = "hex_bytes_32")]
    #[zeroize(skip)]
    pub cold_master_shard: [u8; 32],

    /// Master public key (for verification)
    #[serde(with = "hex_bytes_33")]
    pub master_pubkey: [u8; 33],

    /// Creation timestamp
    pub created_at: u64,

    /// Next child index to use
    pub next_child_index: u32,
}

impl MotherStorage {
    /// Create a new storage instance
    pub fn new(base_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    /// Check if master shard exists
    pub fn has_master_shard(&self) -> bool {
        self.master_shard_path().exists()
    }

    /// Load master shard
    pub fn load_master_shard(&self) -> Result<MasterShardData> {
        let path = self.master_shard_path();
        if !path.exists() {
            return Err(MotherError::MasterKeyNotInitialized);
        }

        let content = std::fs::read_to_string(&path)?;
        let data: MasterShardData = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Save master shard
    pub fn save_master_shard(&self, data: &MasterShardData) -> Result<()> {
        let path = self.master_shard_path();
        let content = serde_json::to_string_pretty(data)?;

        // Write to temp file first, then rename for atomicity
        let temp_path = path.with_extension("json.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &path)?;

        Ok(())
    }

    /// Load child registry
    pub fn load_registry(&self) -> Result<ChildRegistry> {
        let path = self.registry_path();
        if !path.exists() {
            return Ok(ChildRegistry::new());
        }

        let content = std::fs::read_to_string(&path)?;
        let registry: ChildRegistry = serde_json::from_str(&content)?;
        Ok(registry)
    }

    /// Save child registry
    pub fn save_registry(&self, registry: &ChildRegistry) -> Result<()> {
        let path = self.registry_path();
        let content = serde_json::to_string_pretty(registry)?;

        let temp_path = path.with_extension("json.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &path)?;

        Ok(())
    }

    /// Save reconciliation log entry
    pub fn save_reconciliation_log(&self, child_id: &str, log_entry: &str) -> Result<()> {
        let log_dir = self.base_path.join("reconciliation_logs");
        std::fs::create_dir_all(&log_dir)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_path = log_dir.join(format!("{}_{}.log", child_id, timestamp));
        std::fs::write(&log_path, log_entry)?;

        Ok(())
    }

    /// Get path to master shard file
    fn master_shard_path(&self) -> PathBuf {
        self.base_path.join("master_shard.json")
    }

    /// Get path to registry file
    fn registry_path(&self) -> PathBuf {
        self.base_path.join("child_registry.json")
    }
}

impl MasterShardData {
    /// Create new master shard data
    pub fn new(cold_master_shard: [u8; 32], master_pubkey: [u8; 33]) -> Self {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            cold_master_shard,
            master_pubkey,
            created_at,
            next_child_index: 0,
        }
    }

    /// Get and increment the next child index
    pub fn allocate_child_index(&mut self) -> u32 {
        let index = self.next_child_index;
        self.next_child_index += 1;
        index
    }
}
