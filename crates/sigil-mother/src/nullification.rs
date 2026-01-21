//! Agent nullification operations
//!
//! This module handles the nullification of agents, which involves:
//! 1. Adding the agent's ID to the RSA accumulator
//! 2. Updating witnesses for all remaining active agents
//! 3. Recording the nullification in the registry

use sigil_core::{
    accumulator::{NonMembershipWitness, RsaAccumulator, RSA_MODULUS_SIZE},
    agent::AgentId,
};

use crate::error::{MotherError, Result};

/// Nullification ceremony result
#[derive(Debug, Clone)]
pub struct NullificationResult {
    /// The nullified agent's ID
    pub agent_id: AgentId,

    /// Accumulator version after nullification
    pub new_accumulator_version: u64,

    /// Number of witnesses that need updating
    pub witnesses_invalidated: usize,

    /// Timestamp of nullification
    pub timestamp: u64,
}

/// Nullification manager
pub struct NullificationManager {
    /// The RSA accumulator
    accumulator: RsaAccumulator,

    /// List of nullified agent IDs
    nullified_agents: Vec<AgentId>,

    /// Exponent tracking (product of all nullified agent primes)
    /// This is needed to compute non-membership witnesses
    exponent_factors: Vec<Vec<u8>>,
}

impl NullificationManager {
    /// Create a new nullification manager with a fresh accumulator
    pub fn new(modulus: [u8; RSA_MODULUS_SIZE], generator: [u8; RSA_MODULUS_SIZE]) -> Self {
        Self {
            accumulator: RsaAccumulator::new(modulus, generator),
            nullified_agents: Vec::new(),
            exponent_factors: Vec::new(),
        }
    }

    /// Create from an existing accumulator state
    pub fn from_accumulator(accumulator: RsaAccumulator, nullified_agents: Vec<AgentId>) -> Self {
        // Reconstruct exponent factors from nullified agents
        let exponent_factors: Vec<Vec<u8>> =
            nullified_agents.iter().map(|id| id.to_prime()).collect();

        Self {
            accumulator,
            nullified_agents,
            exponent_factors,
        }
    }

    /// Nullify an agent
    ///
    /// This adds the agent's ID (as a prime) to the accumulator exponent
    /// and updates the accumulator value.
    pub fn nullify(&mut self, agent_id: &AgentId) -> Result<NullificationResult> {
        // Check if already nullified
        if self.nullified_agents.contains(agent_id) {
            return Err(MotherError::AgentNullified(agent_id.to_hex()));
        }

        // Get the prime representation
        let prime = agent_id.to_prime();

        // Add to accumulator (increments version)
        let _membership_witness = self.accumulator.add(agent_id);

        // Track nullification
        self.nullified_agents.push(*agent_id);
        self.exponent_factors.push(prime);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(NullificationResult {
            agent_id: *agent_id,
            new_accumulator_version: self.accumulator.version(),
            witnesses_invalidated: 0, // Would be computed from active agents
            timestamp,
        })
    }

    /// Generate a non-membership witness for an active agent
    ///
    /// This proves that the agent's ID is NOT in the accumulator.
    pub fn generate_witness(&self, agent_id: &AgentId) -> Result<NonMembershipWitness> {
        // Check that agent is not nullified
        if self.nullified_agents.contains(agent_id) {
            return Err(MotherError::AgentNullified(agent_id.to_hex()));
        }

        // Compute the Bezout coefficients using extended GCD
        // In a full implementation, this would use big integer arithmetic
        let bezout_a = self.compute_bezout_a(agent_id);
        let cofactor_d = self.compute_cofactor_d(&bezout_a);

        Ok(NonMembershipWitness::new(
            *agent_id,
            bezout_a,
            cofactor_d,
            self.accumulator.version(),
        ))
    }

    /// Update a witness after accumulator changes
    ///
    /// If a different agent was nullified, existing witnesses need updating.
    /// This is an O(1) operation.
    pub fn update_witness(
        &self,
        witness: &NonMembershipWitness,
        nullified_id: &AgentId,
    ) -> Result<NonMembershipWitness> {
        // Check that the witness agent is still active
        if self.nullified_agents.contains(&witness.agent_id) {
            return Err(MotherError::AgentNullified(witness.agent_id.to_hex()));
        }

        // The witness for agent A needs updating when agent B is nullified
        // New witness = old_witness * (old_accumulator^prime_b) mod N
        //
        // This requires computing the new Bezout coefficients

        // For now, regenerate the witness (O(n) where n = number of nullified)
        // A more efficient O(1) update algorithm exists but requires more complex
        // big integer arithmetic
        self.generate_witness(&witness.agent_id)
    }

    /// Get the current accumulator
    pub fn accumulator(&self) -> &RsaAccumulator {
        &self.accumulator
    }

    /// Get the current accumulator version
    pub fn version(&self) -> u64 {
        self.accumulator.version()
    }

    /// Get the list of nullified agents
    pub fn nullified_agents(&self) -> &[AgentId] {
        &self.nullified_agents
    }

    /// Check if an agent is nullified
    pub fn is_nullified(&self, agent_id: &AgentId) -> bool {
        self.nullified_agents.contains(agent_id)
    }

    /// Compute Bezout coefficient 'a' for extended GCD
    ///
    /// Given:
    /// - prime p (agent's ID as prime)
    /// - product P = p1 * p2 * ... * pn (all nullified primes)
    ///
    /// We need a, b such that: a*p + b*P = gcd(p, P)
    ///
    /// If agent is not nullified, gcd = 1 (coprime), so a*p + b*P = 1
    fn compute_bezout_a(&self, agent_id: &AgentId) -> [u8; RSA_MODULUS_SIZE] {
        use sha2::{Digest, Sha256};

        // Placeholder implementation using deterministic derivation
        // In production, use proper extended GCD with big integers
        let mut result = [0u8; RSA_MODULUS_SIZE];

        // Mix agent ID with current accumulator state
        for (i, chunk) in result.chunks_mut(32).enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(b"bezout_a_v1:");
            hasher.update(agent_id.as_bytes());
            hasher.update(&self.accumulator.accumulator[..32]);
            hasher.update(&(i as u64).to_le_bytes());
            hasher.update(&self.accumulator.version().to_le_bytes());
            let hash = hasher.finalize();
            chunk.copy_from_slice(&hash[..chunk.len()]);
        }

        result
    }

    /// Compute cofactor witness d = g^b mod N
    fn compute_cofactor_d(&self, _bezout_a: &[u8; RSA_MODULUS_SIZE]) -> [u8; RSA_MODULUS_SIZE] {
        use sha2::{Digest, Sha256};

        // Placeholder implementation
        // In production: d = g^b mod N where b is the other Bezout coefficient
        let mut result = [0u8; RSA_MODULUS_SIZE];

        for (i, chunk) in result.chunks_mut(32).enumerate() {
            let mut hasher = Sha256::new();
            hasher.update(b"cofactor_d_v1:");
            hasher.update(&self.accumulator.generator[..32]);
            hasher.update(&self.accumulator.accumulator[..32]);
            hasher.update(&(i as u64).to_le_bytes());
            let hash = hasher.finalize();
            chunk.copy_from_slice(&hash[..chunk.len()]);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_modulus() -> [u8; RSA_MODULUS_SIZE] {
        let mut modulus = [0u8; RSA_MODULUS_SIZE];
        modulus[0] = 0x80;
        modulus[RSA_MODULUS_SIZE - 1] = 0xFB; // 251, small prime for testing
        modulus
    }

    fn test_generator() -> [u8; RSA_MODULUS_SIZE] {
        let mut generator = [0u8; RSA_MODULUS_SIZE];
        generator[RSA_MODULUS_SIZE - 1] = 3;
        generator
    }

    #[test]
    fn test_nullification_manager_creation() {
        let manager = NullificationManager::new(test_modulus(), test_generator());
        assert_eq!(manager.version(), 0);
        assert!(manager.nullified_agents().is_empty());
    }

    #[test]
    fn test_nullify_agent() {
        let mut manager = NullificationManager::new(test_modulus(), test_generator());
        let agent_id = AgentId::new([0x42; 32]);

        let result = manager.nullify(&agent_id).unwrap();

        assert_eq!(result.agent_id, agent_id);
        assert_eq!(result.new_accumulator_version, 1);
        assert!(manager.is_nullified(&agent_id));
    }

    #[test]
    fn test_cannot_nullify_twice() {
        let mut manager = NullificationManager::new(test_modulus(), test_generator());
        let agent_id = AgentId::new([0x42; 32]);

        manager.nullify(&agent_id).unwrap();
        let result = manager.nullify(&agent_id);

        assert!(matches!(result, Err(MotherError::AgentNullified(_))));
    }

    #[test]
    fn test_generate_witness_for_active_agent() {
        let manager = NullificationManager::new(test_modulus(), test_generator());
        let agent_id = AgentId::new([0x42; 32]);

        let witness = manager.generate_witness(&agent_id).unwrap();

        assert_eq!(witness.agent_id, agent_id);
        assert_eq!(witness.accumulator_version, 0);
    }

    #[test]
    fn test_cannot_generate_witness_for_nullified_agent() {
        let mut manager = NullificationManager::new(test_modulus(), test_generator());
        let agent_id = AgentId::new([0x42; 32]);

        manager.nullify(&agent_id).unwrap();
        let result = manager.generate_witness(&agent_id);

        assert!(matches!(result, Err(MotherError::AgentNullified(_))));
    }

    #[test]
    fn test_witness_update_after_different_agent_nullified() {
        let mut manager = NullificationManager::new(test_modulus(), test_generator());

        let agent_a = AgentId::new([0x01; 32]);
        let agent_b = AgentId::new([0x02; 32]);

        // Generate witness for agent A
        let witness_a = manager.generate_witness(&agent_a).unwrap();
        assert_eq!(witness_a.accumulator_version, 0);

        // Nullify agent B
        manager.nullify(&agent_b).unwrap();

        // Update witness for agent A
        let updated_witness = manager.update_witness(&witness_a, &agent_b).unwrap();
        assert_eq!(updated_witness.accumulator_version, 1);
        assert_eq!(updated_witness.agent_id, agent_a);
    }
}
