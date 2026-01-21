//! Agent registry
//!
//! Tracks all agents registered with this mother device.
//! Agents hold the "hot" shard of presignatures and participate
//! in signing ceremonies with cold shares from floppy disks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use sigil_core::{
    accumulator::{NonMembershipWitness, RsaAccumulator, RSA_MODULUS_SIZE},
    agent::{AgentId, AgentMetadata, AgentRegistryEntry, AgentStatus},
    ChildId,
};

use crate::error::{MotherError, Result};

/// Registry of all agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistry {
    /// All registered agents (keyed by agent_id hex)
    pub agents: HashMap<String, AgentRegistryEntry>,

    /// RSA accumulator for nullified agents
    pub accumulator: RsaAccumulator,

    /// Nullified agent IDs (for quick lookup)
    pub nullified_ids: Vec<AgentId>,

    /// Non-membership witnesses for active agents
    /// Updated whenever an agent is nullified
    pub witnesses: HashMap<String, NonMembershipWitness>,
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRegistry {
    /// Create a new empty registry with a fresh accumulator
    pub fn new() -> Self {
        // Generate a default modulus and generator for the accumulator
        // In production, this should be generated from safe primes
        let modulus = Self::generate_default_modulus();
        let generator = Self::generate_default_generator();

        Self {
            agents: HashMap::new(),
            accumulator: RsaAccumulator::new(modulus, generator),
            nullified_ids: Vec::new(),
            witnesses: HashMap::new(),
        }
    }

    /// Create registry with a specific RSA modulus
    ///
    /// The modulus should be the product of two safe primes for security.
    pub fn with_modulus(
        modulus: [u8; RSA_MODULUS_SIZE],
        generator: [u8; RSA_MODULUS_SIZE],
    ) -> Self {
        Self {
            agents: HashMap::new(),
            accumulator: RsaAccumulator::new(modulus, generator),
            nullified_ids: Vec::new(),
            witnesses: HashMap::new(),
        }
    }

    /// Generate a default RSA modulus (for development/testing)
    /// In production, use properly generated safe primes
    fn generate_default_modulus() -> [u8; RSA_MODULUS_SIZE] {
        use sha2::{Digest, Sha256};

        let mut modulus = [0u8; RSA_MODULUS_SIZE];
        // Use a deterministic seed for reproducibility
        let seed = Sha256::digest(b"sigil_accumulator_modulus_v1");
        // Fill with hash-derived values and ensure it's odd (not divisible by 2)
        for (i, chunk) in modulus.chunks_mut(32).enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(&seed);
            hasher.update(&(i as u64).to_le_bytes());
            let hash = hasher.finalize();
            chunk.copy_from_slice(&hash[..chunk.len()]);
        }
        // Ensure high bit is set and number is odd
        modulus[0] |= 0x80;
        modulus[RSA_MODULUS_SIZE - 1] |= 0x01;
        modulus
    }

    /// Generate a default generator
    fn generate_default_generator() -> [u8; RSA_MODULUS_SIZE] {
        let mut generator = [0u8; RSA_MODULUS_SIZE];
        // Use a small generator value (common choice is 3 or 65537)
        generator[RSA_MODULUS_SIZE - 1] = 3;
        generator
    }

    /// Register a new agent
    pub fn register_agent(&mut self, agent_id: AgentId, name: String) -> Result<()> {
        let id_hex = agent_id.to_hex();

        // Check if agent already exists
        if self.agents.contains_key(&id_hex) {
            return Err(MotherError::AgentAlreadyExists(id_hex));
        }

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = AgentRegistryEntry::new(agent_id, name, created_at);
        self.agents.insert(id_hex.clone(), entry);

        // Generate non-membership witness for new agent
        let witness = self.generate_non_membership_witness(&agent_id)?;
        self.witnesses.insert(id_hex, witness);

        Ok(())
    }

    /// Get an agent by ID
    pub fn get_agent(&self, agent_id: &AgentId) -> Result<&AgentRegistryEntry> {
        let id_hex = agent_id.to_hex();
        self.agents
            .get(&id_hex)
            .ok_or(MotherError::AgentNotFound(id_hex))
    }

    /// Get mutable agent by ID
    pub fn get_agent_mut(&mut self, agent_id: &AgentId) -> Result<&mut AgentRegistryEntry> {
        let id_hex = agent_id.to_hex();
        self.agents
            .get_mut(&id_hex)
            .ok_or(MotherError::AgentNotFound(id_hex))
    }

    /// Check if an agent can sign (is active)
    pub fn can_sign(&self, agent_id: &AgentId) -> Result<bool> {
        let entry = self.get_agent(agent_id)?;
        Ok(entry.status.can_sign())
    }

    /// Nullify an agent
    ///
    /// This adds the agent to the RSA accumulator and invalidates
    /// all existing non-membership witnesses.
    pub fn nullify_agent(&mut self, agent_id: &AgentId) -> Result<()> {
        let id_hex = agent_id.to_hex();

        // Check if agent exists and is not already nullified
        {
            let entry = self
                .agents
                .get(&id_hex)
                .ok_or(MotherError::AgentNotFound(id_hex.clone()))?;
            if entry.status.is_nullified() {
                return Err(MotherError::AgentNullified(id_hex));
            }
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Add to accumulator (this increments version)
        let _membership_witness = self.accumulator.add(agent_id);
        self.nullified_ids.push(*agent_id);

        let accumulator_version = self.accumulator.version();

        // Update agent status
        if let Some(entry) = self.agents.get_mut(&id_hex) {
            entry.nullify(timestamp, accumulator_version);
        }

        // Remove this agent's non-membership witness
        self.witnesses.remove(&id_hex);

        // Update non-membership witnesses for all other active agents
        self.update_all_witnesses()?;

        Ok(())
    }

    /// Suspend an agent (can be reactivated)
    pub fn suspend_agent(&mut self, agent_id: &AgentId) -> Result<()> {
        let entry = self.get_agent_mut(agent_id)?;

        if entry.status.is_nullified() {
            return Err(MotherError::AgentNullified(agent_id.to_hex()));
        }

        entry.suspend();
        Ok(())
    }

    /// Reactivate a suspended agent
    pub fn reactivate_agent(&mut self, agent_id: &AgentId) -> Result<()> {
        let entry = self.get_agent_mut(agent_id)?;

        if !entry.status.can_reactivate() {
            return Err(MotherError::AgentNullified(agent_id.to_hex()));
        }

        entry.reactivate();
        Ok(())
    }

    /// Authorize a child to use an agent
    pub fn authorize_child(&mut self, agent_id: &AgentId, child_id: ChildId) -> Result<()> {
        let entry = self.get_agent_mut(agent_id)?;
        entry.authorize_child(child_id);
        Ok(())
    }

    /// Revoke a child's authorization
    pub fn revoke_child(&mut self, agent_id: &AgentId, child_id: &ChildId) -> Result<()> {
        let entry = self.get_agent_mut(agent_id)?;
        entry.revoke_child(child_id);
        Ok(())
    }

    /// Check if a child is authorized to use an agent
    pub fn is_child_authorized(&self, agent_id: &AgentId, child_id: &ChildId) -> Result<bool> {
        let entry = self.get_agent(agent_id)?;
        Ok(entry.is_child_authorized(child_id))
    }

    /// Record a signing operation for an agent
    pub fn record_signature(&mut self, agent_id: &AgentId) -> Result<()> {
        let entry = self.get_agent_mut(agent_id)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        entry.record_signature(timestamp);
        Ok(())
    }

    /// Update metadata for an agent
    pub fn update_metadata(&mut self, agent_id: &AgentId, metadata: AgentMetadata) -> Result<()> {
        let entry = self.get_agent_mut(agent_id)?;
        entry.metadata = metadata;
        Ok(())
    }

    /// List all active agents
    pub fn list_active(&self) -> Vec<&AgentRegistryEntry> {
        self.agents
            .values()
            .filter(|e| e.status.can_sign())
            .collect()
    }

    /// List all agents
    pub fn list_all(&self) -> Vec<&AgentRegistryEntry> {
        self.agents.values().collect()
    }

    /// Get count of agents by status
    pub fn count_by_status(&self) -> (usize, usize, usize) {
        let mut active = 0;
        let mut suspended = 0;
        let mut nullified = 0;

        for entry in self.agents.values() {
            match &entry.status {
                AgentStatus::Active => active += 1,
                AgentStatus::Suspended => suspended += 1,
                AgentStatus::Nullified { .. } => nullified += 1,
            }
        }

        (active, suspended, nullified)
    }

    /// Get the current accumulator
    pub fn get_accumulator(&self) -> &RsaAccumulator {
        &self.accumulator
    }

    /// Get accumulator version
    pub fn accumulator_version(&self) -> u64 {
        self.accumulator.version()
    }

    /// Get non-membership witness for an active agent
    pub fn get_witness(&self, agent_id: &AgentId) -> Result<&NonMembershipWitness> {
        let id_hex = agent_id.to_hex();

        // First check if agent is nullified
        let entry = self.get_agent(agent_id)?;
        if entry.status.is_nullified() {
            return Err(MotherError::AgentNullified(id_hex));
        }

        self.witnesses
            .get(&id_hex)
            .ok_or(MotherError::AgentNotFound(id_hex))
    }

    /// Generate non-membership witness for an agent
    ///
    /// This requires knowledge of the accumulator's factorization,
    /// which only the mother device has.
    fn generate_non_membership_witness(&self, agent_id: &AgentId) -> Result<NonMembershipWitness> {
        // Check that agent is not nullified
        if self.nullified_ids.contains(agent_id) {
            return Err(MotherError::AgentNullified(agent_id.to_hex()));
        }

        // In a full implementation, this would compute the Bezout coefficients
        // using the extended Euclidean algorithm on the accumulator's exponent.
        //
        // For now, we generate placeholder values that demonstrate the structure.
        // The actual cryptographic computation requires:
        // 1. The product of all nullified agent primes (which we track)
        // 2. Extended GCD to find a, b where a*prime + b*product = 1
        // 3. Compute d = g^b mod N

        let bezout_a = Self::compute_bezout_a(agent_id, &self.nullified_ids);
        let cofactor_d = Self::compute_cofactor_d(&self.accumulator, &bezout_a);

        Ok(NonMembershipWitness::new(
            *agent_id,
            bezout_a,
            cofactor_d,
            self.accumulator.version(),
        ))
    }

    /// Update all non-membership witnesses after a nullification
    fn update_all_witnesses(&mut self) -> Result<()> {
        let active_ids: Vec<AgentId> = self
            .agents
            .values()
            .filter(|e| e.status.can_sign())
            .map(|e| e.agent_id)
            .collect();

        for agent_id in active_ids {
            let witness = self.generate_non_membership_witness(&agent_id)?;
            self.witnesses.insert(agent_id.to_hex(), witness);
        }

        Ok(())
    }

    /// Compute Bezout coefficient 'a' for non-membership proof
    /// In production, this uses extended GCD
    fn compute_bezout_a(agent_id: &AgentId, _nullified: &[AgentId]) -> [u8; RSA_MODULUS_SIZE] {
        use sha2::{Digest, Sha256};

        // Placeholder: derive deterministically from agent_id
        // Real implementation would use extended Euclidean algorithm
        let mut result = [0u8; RSA_MODULUS_SIZE];
        for (i, chunk) in result.chunks_mut(32).enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(b"bezout_a:");
            hasher.update(agent_id.as_bytes());
            hasher.update(&(i as u64).to_le_bytes());
            let hash = hasher.finalize();
            chunk.copy_from_slice(&hash[..chunk.len()]);
        }
        result
    }

    /// Compute cofactor witness 'd' = g^b mod N
    fn compute_cofactor_d(
        accumulator: &RsaAccumulator,
        _bezout_a: &[u8; RSA_MODULUS_SIZE],
    ) -> [u8; RSA_MODULUS_SIZE] {
        use sha2::{Digest, Sha256};

        // Placeholder: derive deterministically
        // Real implementation: d = g^b mod N where b is the other Bezout coefficient
        let mut result = [0u8; RSA_MODULUS_SIZE];
        for (i, chunk) in result.chunks_mut(32).enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(b"cofactor_d:");
            hasher.update(&accumulator.accumulator);
            hasher.update(&(i as u64).to_le_bytes());
            let hash = hasher.finalize();
            chunk.copy_from_slice(&hash[..chunk.len()]);
        }
        result
    }

    /// Export accumulator for distribution to daemons
    pub fn export_accumulator(&self) -> Vec<u8> {
        self.accumulator.to_bytes()
    }

    /// Get the list of nullified agent IDs
    pub fn nullified_agents(&self) -> &[AgentId] {
        &self.nullified_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_agent_id() -> AgentId {
        AgentId::new([0x42; 32])
    }

    #[test]
    fn test_register_agent() {
        let mut registry = AgentRegistry::new();

        let agent_id = test_agent_id();
        registry
            .register_agent(agent_id, "Test Agent".to_string())
            .unwrap();

        assert!(registry.get_agent(&agent_id).is_ok());
        assert!(registry.can_sign(&agent_id).unwrap());
    }

    #[test]
    fn test_register_duplicate_fails() {
        let mut registry = AgentRegistry::new();

        let agent_id = test_agent_id();
        registry
            .register_agent(agent_id, "Test Agent".to_string())
            .unwrap();

        let result = registry.register_agent(agent_id, "Duplicate".to_string());
        assert!(matches!(result, Err(MotherError::AgentAlreadyExists(_))));
    }

    #[test]
    fn test_nullify_agent() {
        let mut registry = AgentRegistry::new();

        let agent_id = test_agent_id();
        registry
            .register_agent(agent_id, "Test Agent".to_string())
            .unwrap();

        let version_before = registry.accumulator_version();
        registry.nullify_agent(&agent_id).unwrap();

        assert!(!registry.can_sign(&agent_id).unwrap());
        assert_eq!(registry.accumulator_version(), version_before + 1);
    }

    #[test]
    fn test_suspend_reactivate() {
        let mut registry = AgentRegistry::new();

        let agent_id = test_agent_id();
        registry
            .register_agent(agent_id, "Test Agent".to_string())
            .unwrap();

        registry.suspend_agent(&agent_id).unwrap();
        assert!(!registry.can_sign(&agent_id).unwrap());

        registry.reactivate_agent(&agent_id).unwrap();
        assert!(registry.can_sign(&agent_id).unwrap());
    }

    #[test]
    fn test_child_authorization() {
        let mut registry = AgentRegistry::new();

        let agent_id = test_agent_id();
        let child_id = ChildId::new([0x01; 32]);

        registry
            .register_agent(agent_id, "Test Agent".to_string())
            .unwrap();

        assert!(!registry.is_child_authorized(&agent_id, &child_id).unwrap());

        registry.authorize_child(&agent_id, child_id).unwrap();
        assert!(registry.is_child_authorized(&agent_id, &child_id).unwrap());

        registry.revoke_child(&agent_id, &child_id).unwrap();
        assert!(!registry.is_child_authorized(&agent_id, &child_id).unwrap());
    }

    #[test]
    fn test_count_by_status() {
        let mut registry = AgentRegistry::new();

        // Add agents
        for i in 0..5 {
            let mut id_bytes = [0u8; 32];
            id_bytes[0] = i;
            let agent_id = AgentId::new(id_bytes);
            registry
                .register_agent(agent_id, format!("Agent {}", i))
                .unwrap();
        }

        // Suspend one
        let mut suspended_id = [0u8; 32];
        suspended_id[0] = 1;
        registry.suspend_agent(&AgentId::new(suspended_id)).unwrap();

        // Nullify one
        let mut nullified_id = [0u8; 32];
        nullified_id[0] = 2;
        registry.nullify_agent(&AgentId::new(nullified_id)).unwrap();

        let (active, suspended, nullified) = registry.count_by_status();
        assert_eq!(active, 3);
        assert_eq!(suspended, 1);
        assert_eq!(nullified, 1);
    }

    #[test]
    fn test_witness_generation() {
        let mut registry = AgentRegistry::new();

        let agent_id = test_agent_id();
        registry
            .register_agent(agent_id, "Test Agent".to_string())
            .unwrap();

        // Should have a witness
        let witness = registry.get_witness(&agent_id).unwrap();
        assert_eq!(witness.agent_id, agent_id);
        assert_eq!(witness.accumulator_version, registry.accumulator_version());
    }

    #[test]
    fn test_nullified_agent_has_no_witness() {
        let mut registry = AgentRegistry::new();

        let agent_id = test_agent_id();
        registry
            .register_agent(agent_id, "Test Agent".to_string())
            .unwrap();
        registry.nullify_agent(&agent_id).unwrap();

        let result = registry.get_witness(&agent_id);
        assert!(matches!(result, Err(MotherError::AgentNullified(_))));
    }
}
