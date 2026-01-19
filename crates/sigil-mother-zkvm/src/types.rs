//! Input/Output types for zkVM programs
//!
//! These types are shared between the prover (host) and the SP1 programs (guest).
//! They must be serializable and compatible with `#![no_std]` environments.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ============================================================================
// Serialization Helpers
// ============================================================================

/// Helper module for serializing [u8; 32] as hex
pub mod hex_bytes_32 {
    use super::*;

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let mut bytes = [0u8; 32];
            hex::decode_to_slice(&s, &mut bytes).map_err(D::Error::custom)?;
            Ok(bytes)
        } else {
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            bytes
                .try_into()
                .map_err(|_| D::Error::custom("expected 32 bytes"))
        }
    }
}

/// Helper module for serializing [u8; 33] as hex
pub mod hex_bytes_33 {
    use super::*;

    pub fn serialize<S>(bytes: &[u8; 33], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 33], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let mut bytes = [0u8; 33];
            hex::decode_to_slice(&s, &mut bytes).map_err(D::Error::custom)?;
            Ok(bytes)
        } else {
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            bytes
                .try_into()
                .map_err(|_| D::Error::custom("expected 33 bytes"))
        }
    }
}

/// Helper module for serializing [u8; 64] as hex
pub mod hex_bytes_64 {
    use super::*;

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let mut bytes = [0u8; 64];
            hex::decode_to_slice(&s, &mut bytes).map_err(D::Error::custom)?;
            Ok(bytes)
        } else {
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            bytes
                .try_into()
                .map_err(|_| D::Error::custom("expected 64 bytes"))
        }
    }
}

/// Helper module for serializing [u8; 65] as hex
pub mod hex_bytes_65 {
    use super::*;

    pub fn serialize<S>(bytes: &[u8; 65], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 65], D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            let mut bytes = [0u8; 65];
            hex::decode_to_slice(&s, &mut bytes).map_err(D::Error::custom)?;
            Ok(bytes)
        } else {
            let bytes = Vec::<u8>::deserialize(deserializer)?;
            bytes
                .try_into()
                .map_err(|_| D::Error::custom("expected 65 bytes"))
        }
    }
}

// ============================================================================
// Master Key Generation Types
// ============================================================================

/// Input for master key generation proof
///
/// Proves: `master_pubkey = [cold_shard]*G + [agent_shard]*G`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeygenInput {
    // === Private Inputs ===
    /// Cold shard (32 bytes, private)
    #[serde(with = "hex_bytes_32")]
    pub cold_shard: [u8; 32],

    /// Agent shard (32 bytes, private)
    #[serde(with = "hex_bytes_32")]
    pub agent_shard: [u8; 32],

    // === Public Inputs ===
    /// Ceremony nonce for binding (32 bytes, public)
    #[serde(with = "hex_bytes_32")]
    pub ceremony_nonce: [u8; 32],
}

/// Output from master key generation proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeygenOutput {
    /// Combined master public key (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub master_pubkey: [u8; 33],

    /// Cold shard's public key contribution (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub cold_pubkey: [u8; 33],

    /// Agent shard's public key contribution (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub agent_pubkey: [u8; 33],

    /// Ceremony nonce (echoed for verification)
    #[serde(with = "hex_bytes_32")]
    pub ceremony_nonce: [u8; 32],
}

// ============================================================================
// Child Key Derivation Types
// ============================================================================

/// Input for child key derivation proof
///
/// Proves: `child = HKDF(master, path)`, `child_pubkey = [child]*G`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeriveInput {
    // === Private Inputs ===
    /// Cold master shard (32 bytes, private)
    #[serde(with = "hex_bytes_32")]
    pub cold_master_shard: [u8; 32],

    /// Agent master shard (32 bytes, private)
    #[serde(with = "hex_bytes_32")]
    pub agent_master_shard: [u8; 32],

    // === Public Inputs ===
    /// Derivation path bytes
    pub derivation_path: Vec<u8>,

    /// Expected master public key (for validation)
    #[serde(with = "hex_bytes_33")]
    pub master_pubkey: [u8; 33],
}

/// Output from child key derivation proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeriveOutput {
    /// Child public key (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],

    /// Cold child public key contribution (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub cold_child_pubkey: [u8; 33],

    /// Agent child public key contribution (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub agent_child_pubkey: [u8; 33],

    /// Derivation path used (echoed for verification)
    pub derivation_path: Vec<u8>,

    /// Master public key (echoed for verification)
    #[serde(with = "hex_bytes_33")]
    pub master_pubkey: [u8; 33],
}

// ============================================================================
// Batch Presignature Types
// ============================================================================

/// Input for batch presignature proof
///
/// Proves: `R_i = (k_cold_i + k_agent_i)*G` for a batch of presignatures
/// Uses Merkle tree for efficient commitment to all R points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPresigInput {
    // === Private Inputs ===
    /// Cold child shard (32 bytes, private)
    #[serde(with = "hex_bytes_32")]
    pub cold_child_shard: [u8; 32],

    /// Agent child shard (32 bytes, private)
    #[serde(with = "hex_bytes_32")]
    pub agent_child_shard: [u8; 32],

    /// Cold nonce shares for each presig (private)
    pub k_colds: Vec<[u8; 32]>,

    /// Agent nonce shares for each presig (private)
    pub k_agents: Vec<[u8; 32]>,

    // === Public Inputs ===
    /// Child public key (for validation)
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],

    /// Starting index for this batch
    pub start_index: u32,

    /// Number of presigs in batch
    pub batch_size: u32,

    /// Indices to sample for full verification (random sampling)
    pub sample_indices: Vec<u32>,
}

/// Output from batch presignature proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchPresigOutput {
    /// Merkle root of all R points
    #[serde(with = "hex_bytes_32")]
    pub r_points_merkle_root: [u8; 32],

    /// First R point in batch (for quick validation)
    #[serde(with = "hex_bytes_33")]
    pub first_r_point: [u8; 33],

    /// Last R point in batch (for quick validation)
    #[serde(with = "hex_bytes_33")]
    pub last_r_point: [u8; 33],

    /// Sampled R points with their indices (for random verification)
    pub sampled_r_points: Vec<SampledRPoint>,

    /// Batch size (echoed for verification)
    pub batch_size: u32,

    /// Start index (echoed for verification)
    pub start_index: u32,

    /// Child public key (echoed for verification)
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],
}

/// A sampled R point with its index and Merkle proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SampledRPoint {
    /// The R point (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub r_point: [u8; 33],

    /// Index within the batch
    pub index: u32,

    /// Merkle proof for this R point
    pub merkle_proof: Vec<[u8; 32]>,
}

// ============================================================================
// Hardware Derivation Types
// ============================================================================

/// Input for hardware wallet derivation proof
///
/// Proves:
/// 1. Signature is valid for the device's public key
/// 2. Shard is correctly derived as `SHA256(domain || signature)`
/// 3. Public key is correctly computed from shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInput {
    // === Private Inputs ===
    /// The signature from the hardware device (65 bytes, private)
    #[serde(with = "hex_bytes_65")]
    pub signature: [u8; 65],

    /// Derived shard (32 bytes, private)
    #[serde(with = "hex_bytes_32")]
    pub derived_shard: [u8; 32],

    // === Public Inputs ===
    /// Device's public key (uncompressed, 65 bytes)
    #[serde(with = "hex_bytes_65")]
    pub device_pubkey: [u8; 65],

    /// The message that was signed
    pub message: Vec<u8>,

    /// Domain separation tag used for shard derivation
    pub domain: Vec<u8>,

    /// Whether this is cold shard (true) or agent shard (false)
    pub is_cold_shard: bool,
}

/// Output from hardware wallet derivation proof
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareOutput {
    /// Derived shard's public key contribution (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub shard_pubkey: [u8; 33],

    /// Device's public key (echoed for verification)
    #[serde(with = "hex_bytes_65")]
    pub device_pubkey: [u8; 65],

    /// Hash of the message that was signed
    #[serde(with = "hex_bytes_32")]
    pub message_hash: [u8; 32],

    /// Whether this is cold shard (true) or agent shard (false)
    pub is_cold_shard: bool,
}

// ============================================================================
// Proof Metadata
// ============================================================================

/// Metadata about a generated proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Type of proof
    pub proof_type: ProofType,

    /// Timestamp when proof was generated (Unix timestamp)
    pub generated_at: u64,

    /// SP1 program version hash
    pub program_vkey: String,

    /// Proof size in bytes
    pub proof_size: usize,

    /// Generation time in milliseconds
    pub generation_time_ms: u64,
}

/// Type of proof
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProofType {
    /// Master key generation proof
    Keygen,
    /// Child key derivation proof
    Derive,
    /// Batch presignature proof
    BatchPresig,
    /// Hardware wallet derivation proof
    Hardware,
}

impl ProofType {
    /// Get the name of this proof type
    pub fn name(&self) -> &'static str {
        match self {
            ProofType::Keygen => "keygen",
            ProofType::Derive => "derive",
            ProofType::BatchPresig => "batch_presig",
            ProofType::Hardware => "hardware",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keygen_input_serialization() {
        let input = KeygenInput {
            cold_shard: [1u8; 32],
            agent_shard: [2u8; 32],
            ceremony_nonce: [3u8; 32],
        };

        let json = serde_json::to_string(&input).unwrap();
        let deserialized: KeygenInput = serde_json::from_str(&json).unwrap();

        assert_eq!(input.cold_shard, deserialized.cold_shard);
        assert_eq!(input.agent_shard, deserialized.agent_shard);
        assert_eq!(input.ceremony_nonce, deserialized.ceremony_nonce);
    }

    #[test]
    fn test_batch_presig_output_serialization() {
        let output = BatchPresigOutput {
            r_points_merkle_root: [4u8; 32],
            first_r_point: [0x02; 33],
            last_r_point: [0x03; 33],
            sampled_r_points: vec![],
            batch_size: 1000,
            start_index: 0,
            child_pubkey: [0x02; 33],
        };

        let json = serde_json::to_string_pretty(&output).unwrap();
        let deserialized: BatchPresigOutput = serde_json::from_str(&json).unwrap();

        assert_eq!(output, deserialized);
    }
}
