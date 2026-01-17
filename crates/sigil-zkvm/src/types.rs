//! Types for zkVM signing operations

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Helper module for serializing [u8; 32] as hex
mod hex_bytes_32 {
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
mod hex_bytes_33 {
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
mod hex_bytes_64 {
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

/// Input to the zkVM signing program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningInput {
    // === Public Inputs ===
    /// Child public key (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],

    /// Message hash to sign (32 bytes)
    #[serde(with = "hex_bytes_32")]
    pub message_hash: [u8; 32],

    /// Index of the presignature being used
    pub presig_index: u32,

    // === Private Inputs ===
    /// Cold party's presignature share
    pub presig_cold: PresigShareInput,

    /// Agent party's presignature share
    pub presig_agent: PresigShareInput,
}

/// Presignature share input for zkVM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresigShareInput {
    /// Nonce commitment point R (compressed, 33 bytes)
    #[serde(with = "hex_bytes_33")]
    pub r_point: [u8; 33],

    /// Party's nonce share k
    #[serde(with = "hex_bytes_32")]
    pub k_share: [u8; 32],

    /// Auxiliary value chi for signature completion
    #[serde(with = "hex_bytes_32")]
    pub chi: [u8; 32],
}

/// Output from the zkVM signing program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningOutput {
    /// The produced ECDSA signature (r || s, 64 bytes)
    #[serde(with = "hex_bytes_64")]
    pub signature: [u8; 64],

    /// The presignature index that was used
    pub presig_index: u32,

    /// The message hash that was signed (for verification)
    #[serde(with = "hex_bytes_32")]
    pub message_hash: [u8; 32],

    /// The public key used (for verification)
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],
}
