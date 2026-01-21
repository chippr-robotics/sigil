//! Agent types and registry entry for agent management
//!
//! Agents are software entities that hold the "hot" shard of presignatures
//! and participate in the signing ceremony with cold shares from floppy disks.

use serde::{Deserialize, Serialize};

use crate::crypto::sha256_multi;
use crate::types::{hex_bytes_32, ChildId};

/// Agent ID - 32-byte hash of the agent's public key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(#[serde(with = "hex_bytes_32")] pub [u8; 32]);

impl AgentId {
    /// Create a new AgentId from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create AgentId from an agent's public key (SHA256 hash)
    pub fn from_pubkey(pubkey: &[u8]) -> Self {
        let hash = sha256_multi(&[pubkey]);
        Self(hash)
    }

    /// Get the bytes of the AgentId
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from hex string
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Short display format (first 4 bytes as hex)
    pub fn short(&self) -> String {
        hex::encode(&self.0[..4])
    }

    /// Convert to a prime for RSA accumulator operations
    /// Uses hash-to-prime: repeatedly hash until we get a probable prime
    pub fn to_prime(&self) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        // Start with the agent ID bytes
        let mut candidate = self.0.to_vec();
        let mut counter: u64 = 0;

        loop {
            // Hash the candidate with a counter to get a new candidate
            let mut hasher = Sha256::new();
            hasher.update(&candidate);
            hasher.update(&counter.to_le_bytes());
            let hash: [u8; 32] = hasher.finalize().into();

            // Extend to 256 bits and set the high bit (ensure it's odd and large enough)
            let mut prime_candidate = hash.to_vec();
            prime_candidate[0] |= 0x80; // Set high bit
            prime_candidate[31] |= 0x01; // Ensure odd

            // Simple primality check (Miller-Rabin would be better but this is simpler)
            // For now, we use a deterministic mapping that's consistent
            // In production, this would use proper hash-to-prime
            if counter > 1000 || is_likely_prime(&prime_candidate) {
                return prime_candidate;
            }

            counter += 1;
            candidate = hash.to_vec();
        }
    }
}

/// Simple primality check for small candidates
/// In production, use proper Miller-Rabin or similar
fn is_likely_prime(bytes: &[u8]) -> bool {
    // For demonstration, we just check if it's odd and the hash meets criteria
    // Real implementation would use num-bigint with Miller-Rabin
    if bytes.is_empty() {
        return false;
    }

    // Must be odd
    if bytes[bytes.len() - 1] & 1 == 0 {
        return false;
    }

    // Check some small prime factors
    // This is a simplified check - production code would be more thorough
    let small_primes = [3u8, 5, 7, 11, 13, 17, 19, 23, 29, 31];
    for &p in &small_primes {
        let mut remainder = 0u16;
        for &byte in bytes {
            remainder = ((remainder << 8) | byte as u16) % p as u16;
        }
        if remainder == 0 && bytes.len() > 1 {
            return false;
        }
    }

    true
}

impl AsRef<[u8]> for AgentId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Status of an agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AgentStatus {
    /// Active and can sign
    #[default]
    Active,

    /// Temporarily suspended (can be reactivated)
    Suspended,

    /// Permanently nullified (added to accumulator)
    Nullified {
        /// Unix timestamp when nullified
        timestamp: u64,
        /// The accumulator version when this agent was nullified
        nullified_at_version: u64,
    },
}

impl AgentStatus {
    /// Check if the agent can sign
    pub fn can_sign(&self) -> bool {
        matches!(self, AgentStatus::Active)
    }

    /// Check if the agent can be reactivated
    pub fn can_reactivate(&self) -> bool {
        matches!(self, AgentStatus::Suspended)
    }

    /// Check if permanently nullified
    pub fn is_nullified(&self) -> bool {
        matches!(self, AgentStatus::Nullified { .. })
    }
}

impl core::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AgentStatus::Active => write!(f, "Active"),
            AgentStatus::Suspended => write!(f, "Suspended"),
            AgentStatus::Nullified { timestamp, .. } => write!(f, "Nullified at {}", timestamp),
        }
    }
}

/// Metadata associated with an agent
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentMetadata {
    /// Human-readable description
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// IP address or hostname where the agent runs (for audit purposes)
    pub host: Option<String>,
    /// Custom key-value pairs
    pub custom: std::collections::HashMap<String, String>,
}

/// Agent registry entry stored on mother device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistryEntry {
    /// Unique identifier for this agent
    pub agent_id: AgentId,

    /// Human-readable name
    pub name: String,

    /// Current status
    pub status: AgentStatus,

    /// Creation timestamp
    pub created_at: u64,

    /// Children authorized to use this agent
    pub authorized_children: Vec<ChildId>,

    /// Additional metadata
    pub metadata: AgentMetadata,

    /// Total signatures facilitated by this agent
    pub total_signatures: u64,

    /// Last activity timestamp
    pub last_activity: Option<u64>,
}

impl AgentRegistryEntry {
    /// Create a new agent registry entry
    pub fn new(agent_id: AgentId, name: String, created_at: u64) -> Self {
        Self {
            agent_id,
            name,
            status: AgentStatus::Active,
            created_at,
            authorized_children: Vec::new(),
            metadata: AgentMetadata::default(),
            total_signatures: 0,
            last_activity: None,
        }
    }

    /// Authorize a child to use this agent
    pub fn authorize_child(&mut self, child_id: ChildId) {
        if !self.authorized_children.contains(&child_id) {
            self.authorized_children.push(child_id);
        }
    }

    /// Revoke a child's authorization
    pub fn revoke_child(&mut self, child_id: &ChildId) {
        self.authorized_children.retain(|id| id != child_id);
    }

    /// Check if a child is authorized
    pub fn is_child_authorized(&self, child_id: &ChildId) -> bool {
        self.authorized_children.contains(child_id)
    }

    /// Record a signing operation
    pub fn record_signature(&mut self, timestamp: u64) {
        self.total_signatures += 1;
        self.last_activity = Some(timestamp);
    }

    /// Suspend this agent
    pub fn suspend(&mut self) {
        if matches!(self.status, AgentStatus::Active) {
            self.status = AgentStatus::Suspended;
        }
    }

    /// Reactivate this agent (if suspended)
    pub fn reactivate(&mut self) -> bool {
        if matches!(self.status, AgentStatus::Suspended) {
            self.status = AgentStatus::Active;
            true
        } else {
            false
        }
    }

    /// Nullify this agent (permanent)
    pub fn nullify(&mut self, timestamp: u64, accumulator_version: u64) {
        self.status = AgentStatus::Nullified {
            timestamp,
            nullified_at_version: accumulator_version,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_from_pubkey() {
        let pubkey = [0x04u8; 33]; // Example compressed pubkey
        let agent_id = AgentId::from_pubkey(&pubkey);
        assert_eq!(agent_id.as_bytes().len(), 32);
    }

    #[test]
    fn test_agent_id_hex_roundtrip() {
        let agent_id = AgentId::new([0xab; 32]);
        let hex = agent_id.to_hex();
        let recovered = AgentId::from_hex(&hex).unwrap();
        assert_eq!(agent_id, recovered);
    }

    #[test]
    fn test_agent_status_can_sign() {
        assert!(AgentStatus::Active.can_sign());
        assert!(!AgentStatus::Suspended.can_sign());
        assert!(!(AgentStatus::Nullified {
            timestamp: 0,
            nullified_at_version: 0
        })
        .can_sign());
    }

    #[test]
    fn test_agent_child_authorization() {
        let agent_id = AgentId::new([0x01; 32]);
        let mut entry = AgentRegistryEntry::new(agent_id, "Test Agent".to_string(), 1000);

        let child_id = ChildId::new([0x02; 32]);
        assert!(!entry.is_child_authorized(&child_id));

        entry.authorize_child(child_id);
        assert!(entry.is_child_authorized(&child_id));

        entry.revoke_child(&child_id);
        assert!(!entry.is_child_authorized(&child_id));
    }

    #[test]
    fn test_agent_suspend_reactivate() {
        let agent_id = AgentId::new([0x01; 32]);
        let mut entry = AgentRegistryEntry::new(agent_id, "Test Agent".to_string(), 1000);

        assert!(entry.status.can_sign());

        entry.suspend();
        assert!(!entry.status.can_sign());
        assert!(entry.status.can_reactivate());

        assert!(entry.reactivate());
        assert!(entry.status.can_sign());
    }

    #[test]
    fn test_agent_nullify() {
        let agent_id = AgentId::new([0x01; 32]);
        let mut entry = AgentRegistryEntry::new(agent_id, "Test Agent".to_string(), 1000);

        entry.nullify(2000, 5);

        assert!(entry.status.is_nullified());
        assert!(!entry.status.can_sign());
        assert!(!entry.status.can_reactivate());

        if let AgentStatus::Nullified {
            timestamp,
            nullified_at_version,
        } = entry.status
        {
            assert_eq!(timestamp, 2000);
            assert_eq!(nullified_at_version, 5);
        } else {
            panic!("Expected nullified status");
        }
    }
}
