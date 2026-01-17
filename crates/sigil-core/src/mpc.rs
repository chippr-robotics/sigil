//! MPC (Multi-Party Computation) operations
//!
//! This module provides the core MPC signing operations for the Sigil system.

use crate::error::Result;
use crate::hd::{ChildShard, MasterShard};
use crate::presig::{generate_presig_batch, PresigAgentShare, PresigColdShare};
use rand::thread_rng;
use serde::{Deserialize, Serialize};

/// MPC shard pair (cold + agent)
#[derive(Debug, Clone)]
pub struct ShardPair {
    pub cold: ChildShard,
    pub agent: ChildShard,
}

impl ShardPair {
    /// Create a new shard pair from master shards
    pub fn from_masters(
        cold_master: &MasterShard,
        agent_master: &MasterShard,
        path: &crate::hd::DerivationPath,
    ) -> Result<Self> {
        let cold = cold_master.derive_child(path)?;
        let agent = agent_master.derive_child(path)?;
        
        Ok(Self { cold, agent })
    }

    /// Get the combined public key
    pub fn pubkey(&self) -> Result<[u8; 33]> {
        ChildShard::compute_combined_pubkey(&self.cold, &self.agent)
    }

    /// Generate presignatures for this shard pair
    pub fn generate_presignatures(
        &self,
        count: usize,
    ) -> Result<(Vec<PresigColdShare>, Vec<PresigAgentShare>)> {
        let mut rng = thread_rng();
        generate_presig_batch(&mut rng, count)
    }
}

/// Agent presignature store
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AgentPresigStore {
    /// Map from child_id to presignature shares
    shares: std::collections::HashMap<String, Vec<PresigAgentShare>>,
}

impl AgentPresigStore {
    /// Create a new empty store
    pub fn new() -> Self {
        Self::default()
    }

    /// Store presignatures for a child
    pub fn store(&mut self, child_id: String, shares: Vec<PresigAgentShare>) {
        self.shares.insert(child_id, shares);
    }

    /// Get presignatures for a child
    pub fn get(&self, child_id: &str) -> Option<&Vec<PresigAgentShare>> {
        self.shares.get(child_id)
    }

    /// Remove presignatures for a child (nullification)
    pub fn remove(&mut self, child_id: &str) -> Option<Vec<PresigAgentShare>> {
        self.shares.remove(child_id)
    }

    /// Get count of available presignatures for a child
    pub fn count(&self, child_id: &str) -> usize {
        self.shares.get(child_id).map(|s| s.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hd::DerivationPath;

    #[test]
    fn test_shard_pair_creation() {
        let cold_master = MasterShard::from_seed(b"cold seed").unwrap();
        let agent_master = MasterShard::from_seed(b"agent seed").unwrap();
        
        let path = DerivationPath::bip44_ethereum(0);
        let pair = ShardPair::from_masters(&cold_master, &agent_master, &path).unwrap();
        
        let pubkey = pair.pubkey().unwrap();
        assert_eq!(pubkey.len(), 33);
    }

    #[test]
    fn test_presignature_generation() {
        let cold_master = MasterShard::from_seed(b"cold seed").unwrap();
        let agent_master = MasterShard::from_seed(b"agent seed").unwrap();
        
        let path = DerivationPath::bip44_ethereum(0);
        let pair = ShardPair::from_masters(&cold_master, &agent_master, &path).unwrap();
        
        let (cold_shares, agent_shares) = pair.generate_presignatures(10).unwrap();
        assert_eq!(cold_shares.len(), 10);
        assert_eq!(agent_shares.len(), 10);
    }

    #[test]
    fn test_agent_presig_store() {
        let mut store = AgentPresigStore::new();
        
        let child_id = "test_child".to_string();
        let shares = vec![];
        
        store.store(child_id.clone(), shares);
        assert_eq!(store.count(&child_id), 0);
        
        let retrieved = store.get(&child_id);
        assert!(retrieved.is_some());
    }
}
