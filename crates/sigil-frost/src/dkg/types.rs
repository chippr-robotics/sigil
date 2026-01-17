//! DKG types for round packages and configuration

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::SignatureScheme;

/// Participant identifier (1-indexed)
pub type ParticipantId = u16;

/// Role of a participant in the DKG ceremony
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum ParticipantRole {
    /// Mother device (air-gapped, holds cold share)
    Mother,
    /// Agent device (network-connected, holds agent share)
    Agent,
}

impl ParticipantRole {
    /// Get the default participant ID for this role
    pub fn default_id(&self) -> ParticipantId {
        match self {
            ParticipantRole::Mother => 1,
            ParticipantRole::Agent => 2,
        }
    }
}

/// Configuration for a DKG ceremony
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgConfig {
    /// This participant's ID (1-indexed)
    pub participant_id: ParticipantId,

    /// This participant's role
    pub role: ParticipantRole,

    /// Minimum signers required (threshold)
    pub min_signers: u16,

    /// Maximum signers (total participants)
    pub max_signers: u16,

    /// Signature scheme to use
    pub scheme: SignatureScheme,
}

impl DkgConfig {
    /// Create a new 2-of-2 configuration for mother device
    pub fn mother_2of2(scheme: SignatureScheme) -> Self {
        Self {
            participant_id: 1,
            role: ParticipantRole::Mother,
            min_signers: 2,
            max_signers: 2,
            scheme,
        }
    }

    /// Create a new 2-of-2 configuration for agent device
    pub fn agent_2of2(scheme: SignatureScheme) -> Self {
        Self {
            participant_id: 2,
            role: ParticipantRole::Agent,
            min_signers: 2,
            max_signers: 2,
            scheme,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.participant_id == 0 {
            return Err("Participant ID must be >= 1");
        }
        if self.participant_id > self.max_signers {
            return Err("Participant ID must be <= max_signers");
        }
        if self.min_signers < 2 {
            return Err("Minimum signers must be >= 2");
        }
        if self.min_signers > self.max_signers {
            return Err("min_signers must be <= max_signers");
        }
        Ok(())
    }
}

/// State of the DKG ceremony
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DkgState {
    /// Initial state, ready to generate Round 1
    Initialized,
    /// Round 1 generated, waiting for other Round 1 packages
    Round1Generated,
    /// Round 2 generated, waiting for other Round 2 packages
    Round2Generated,
    /// Ceremony completed successfully
    Completed,
    /// Ceremony failed
    Failed,
}

/// Round 1 package for DKG
///
/// Contains commitments to the participant's secret polynomial
/// and a proof of knowledge of the secret.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DkgRound1Package {
    /// Protocol version
    pub version: u8,

    /// Sender's participant ID
    pub sender_id: ParticipantId,

    /// Signature scheme
    pub scheme: SignatureScheme,

    /// Threshold parameters
    pub min_signers: u16,
    pub max_signers: u16,

    /// Commitment to secret polynomial coefficients (Feldman VSS)
    /// For t-of-n: contains t commitment points
    /// Each point is a compressed curve point (33 bytes for secp256k1, 32 for ed25519)
    pub commitments: Vec<Vec<u8>>,

    /// Proof of knowledge of the secret (signature)
    /// Prevents rogue-key attacks
    pub proof_of_knowledge: Vec<u8>,

    /// Serialized FROST Round1Package for the specific scheme
    /// This contains the actual cryptographic data
    pub frost_package: Vec<u8>,
}

impl DkgRound1Package {
    /// Current protocol version
    pub const VERSION: u8 = 1;

    /// Create a new Round 1 package
    pub fn new(
        sender_id: ParticipantId,
        scheme: SignatureScheme,
        min_signers: u16,
        max_signers: u16,
        commitments: Vec<Vec<u8>>,
        proof_of_knowledge: Vec<u8>,
        frost_package: Vec<u8>,
    ) -> Self {
        Self {
            version: Self::VERSION,
            sender_id,
            scheme,
            min_signers,
            max_signers,
            commitments,
            proof_of_knowledge,
            frost_package,
        }
    }

    /// Compute a hash of this package for binding in Round 2
    pub fn binding_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update([self.version]);
        hasher.update(self.sender_id.to_le_bytes());
        hasher.update([self.scheme as u8]);
        hasher.update(self.min_signers.to_le_bytes());
        hasher.update(self.max_signers.to_le_bytes());
        for commitment in &self.commitments {
            hasher.update(commitment);
        }
        hasher.update(&self.proof_of_knowledge);
        hasher.finalize().into()
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        bitcode::encode(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::error::FrostError> {
        bitcode::decode(bytes).map_err(|e| crate::error::FrostError::Deserialization(e.to_string()))
    }
}

/// Round 2 package for DKG
///
/// Contains the secret share for a specific recipient,
/// bound to the Round 1 transcript.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DkgRound2Package {
    /// Protocol version
    pub version: u8,

    /// Sender's participant ID
    pub sender_id: ParticipantId,

    /// Recipient's participant ID
    pub recipient_id: ParticipantId,

    /// Signature scheme
    pub scheme: SignatureScheme,

    /// Hash of the Round 1 transcript (all Round 1 packages)
    /// Ensures both parties agree on the ceremony state
    pub round1_hash: [u8; 32],

    /// Serialized FROST Round2Package for the specific scheme
    /// Contains the secret share for the recipient
    pub frost_package: Vec<u8>,
}

impl DkgRound2Package {
    /// Current protocol version
    pub const VERSION: u8 = 1;

    /// Create a new Round 2 package
    pub fn new(
        sender_id: ParticipantId,
        recipient_id: ParticipantId,
        scheme: SignatureScheme,
        round1_hash: [u8; 32],
        frost_package: Vec<u8>,
    ) -> Self {
        Self {
            version: Self::VERSION,
            sender_id,
            recipient_id,
            scheme,
            round1_hash,
            frost_package,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        bitcode::encode(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::error::FrostError> {
        bitcode::decode(bytes).map_err(|e| crate::error::FrostError::Deserialization(e.to_string()))
    }
}

/// Compute the Round 1 transcript hash from all Round 1 packages
pub fn compute_round1_hash(packages: &[DkgRound1Package]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();

    // Sort by sender_id for deterministic ordering
    let mut sorted: Vec<_> = packages.iter().collect();
    sorted.sort_by_key(|p| p.sender_id);

    for pkg in sorted {
        hasher.update(pkg.binding_hash());
    }

    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let valid = DkgConfig::mother_2of2(SignatureScheme::Taproot);
        assert!(valid.validate().is_ok());

        let invalid = DkgConfig {
            participant_id: 0,
            role: ParticipantRole::Mother,
            min_signers: 2,
            max_signers: 2,
            scheme: SignatureScheme::Taproot,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_round1_serialization() {
        let pkg = DkgRound1Package::new(
            1,
            SignatureScheme::Taproot,
            2,
            2,
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![7, 8, 9],
            vec![10, 11, 12],
        );

        let bytes = pkg.to_bytes();
        let decoded = DkgRound1Package::from_bytes(&bytes).unwrap();

        assert_eq!(pkg.sender_id, decoded.sender_id);
        assert_eq!(pkg.commitments, decoded.commitments);
    }

    #[test]
    fn test_round1_hash_deterministic() {
        let pkg1 = DkgRound1Package::new(
            1,
            SignatureScheme::Taproot,
            2,
            2,
            vec![vec![1, 2, 3]],
            vec![4, 5, 6],
            vec![7, 8, 9],
        );

        let pkg2 = DkgRound1Package::new(
            2,
            SignatureScheme::Taproot,
            2,
            2,
            vec![vec![10, 11, 12]],
            vec![13, 14, 15],
            vec![16, 17, 18],
        );

        // Order shouldn't matter
        let hash1 = compute_round1_hash(&[pkg1.clone(), pkg2.clone()]);
        let hash2 = compute_round1_hash(&[pkg2, pkg1]);

        assert_eq!(hash1, hash2);
    }
}
