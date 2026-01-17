//! Agent shard storage
//!
//! Manages encrypted storage of agent presignature shares.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

use sigil_core::{presig::PresigAgentShare, ChildId};

use crate::error::{DaemonError, Result};

/// Storage for agent-side presignature shares
pub struct AgentStore {
    /// Base path for storage
    store_path: PathBuf,

    /// In-memory cache of agent shares (child_id -> shares)
    cache: HashMap<ChildId, AgentChildData>,
}

/// Data stored for each child
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChildData {
    /// Child ID
    pub child_id: ChildId,

    /// Agent's presignature shares (indexed by presig index)
    pub presig_shares: Vec<PresigAgentShare>,

    /// Index of the next unused presig
    pub next_presig_index: u32,

    /// Total presigs allocated
    pub total_presigs: u32,
}

impl AgentStore {
    /// Create a new agent store
    pub fn new(store_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&store_path)?;

        Ok(Self {
            store_path,
            cache: HashMap::new(),
        })
    }

    /// Load data for a specific child
    pub fn load_child(&mut self, child_id: &ChildId) -> Result<&AgentChildData> {
        if !self.cache.contains_key(child_id) {
            let data = self.load_from_disk(child_id)?;
            self.cache.insert(*child_id, data);
        }
        self.cache
            .get(child_id)
            .ok_or_else(|| DaemonError::AgentShardNotFound(child_id.to_hex()))
    }

    /// Get mutable data for a child
    pub fn get_child_mut(&mut self, child_id: &ChildId) -> Result<&mut AgentChildData> {
        if !self.cache.contains_key(child_id) {
            let data = self.load_from_disk(child_id)?;
            self.cache.insert(*child_id, data);
        }
        self.cache
            .get_mut(child_id)
            .ok_or_else(|| DaemonError::AgentShardNotFound(child_id.to_hex()))
    }

    /// Store data for a child (new or update)
    pub fn store_child(&mut self, data: AgentChildData) -> Result<()> {
        let child_id = data.child_id;
        self.save_to_disk(&data)?;
        self.cache.insert(child_id, data);
        Ok(())
    }

    /// Get agent presig share for a specific index
    pub fn get_presig_share(
        &mut self,
        child_id: &ChildId,
        index: u32,
    ) -> Result<&PresigAgentShare> {
        let data = self.load_child(child_id)?;
        data.presig_shares.get(index as usize).ok_or_else(|| {
            DaemonError::AgentShardNotFound(format!(
                "Presig {} for child {}",
                index,
                child_id.short()
            ))
        })
    }

    /// Mark a presig as used
    pub fn mark_presig_used(&mut self, child_id: &ChildId, index: u32) -> Result<()> {
        // Update in cache
        {
            let data = self.get_child_mut(child_id)?;
            if index >= data.next_presig_index {
                data.next_presig_index = index + 1;
            }
        }
        // Save to disk (borrow released)
        if let Some(data) = self.cache.get(child_id) {
            self.save_to_disk(data)?;
        }
        Ok(())
    }

    /// Delete data for a child (on nullification)
    pub fn delete_child(&mut self, child_id: &ChildId) -> Result<()> {
        // Remove from cache
        if let Some(mut data) = self.cache.remove(child_id) {
            // Zeroize sensitive data
            for share in &mut data.presig_shares {
                share.k_agent.zeroize();
                share.chi_agent.zeroize();
            }
        }

        // Remove from disk
        let path = self.child_path(child_id);
        if path.exists() {
            // Overwrite with zeros before deletion for security
            let zeros = vec![0u8; std::fs::metadata(&path)?.len() as usize];
            std::fs::write(&path, &zeros)?;
            std::fs::remove_file(&path)?;
        }

        Ok(())
    }

    /// List all stored child IDs
    pub fn list_children(&self) -> Result<Vec<ChildId>> {
        let mut children = Vec::new();

        for entry in std::fs::read_dir(&self.store_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    if let Ok(child_id) = ChildId::from_hex(&stem.to_string_lossy()) {
                        children.push(child_id);
                    }
                }
            }
        }

        Ok(children)
    }

    /// Get path for a child's data file
    fn child_path(&self, child_id: &ChildId) -> PathBuf {
        self.store_path.join(format!("{}.json", child_id.to_hex()))
    }

    /// Load child data from disk
    fn load_from_disk(&self, child_id: &ChildId) -> Result<AgentChildData> {
        let path = self.child_path(child_id);
        if !path.exists() {
            return Err(DaemonError::AgentShardNotFound(child_id.to_hex()));
        }

        let content = std::fs::read_to_string(&path)?;
        let data: AgentChildData = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Save child data to disk
    fn save_to_disk(&self, data: &AgentChildData) -> Result<()> {
        let path = self.child_path(&data.child_id);
        let content = serde_json::to_string_pretty(data)?;

        // Write to temp file first, then rename for atomicity
        let temp_path = path.with_extension("json.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &path)?;

        Ok(())
    }
}

impl AgentChildData {
    /// Create new child data with presig shares
    pub fn new(child_id: ChildId, presig_shares: Vec<PresigAgentShare>) -> Self {
        let total = presig_shares.len() as u32;
        Self {
            child_id,
            presig_shares,
            next_presig_index: 0,
            total_presigs: total,
        }
    }

    /// Get remaining presigs
    pub fn remaining_presigs(&self) -> u32 {
        self.total_presigs.saturating_sub(self.next_presig_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = AgentStore::new(temp_dir.path().to_path_buf()).unwrap();

        let child_id = ChildId::new([1u8; 32]);
        let shares = vec![PresigAgentShare::new([2u8; 33], [3u8; 32], [4u8; 32])];

        let data = AgentChildData::new(child_id, shares);
        store.store_child(data).unwrap();

        let loaded = store.load_child(&child_id).unwrap();
        assert_eq!(loaded.child_id, child_id);
        assert_eq!(loaded.presig_shares.len(), 1);
    }
}
