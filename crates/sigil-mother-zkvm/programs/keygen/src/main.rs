//! SP1 zkVM program for master key generation
//!
//! Proves: `master_pubkey = [cold_shard]*G + [agent_shard]*G`
//!
//! This program runs inside the SP1 zkVM and produces a proof that:
//! 1. The cold and agent shards are valid secp256k1 scalars
//! 2. The individual public keys are correctly computed
//! 3. The master public key is the sum of the individual public keys

#![no_main]
sp1_zkvm::entrypoint!(main);

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use k256::{
    elliptic_curve::{sec1::ToEncodedPoint, PrimeField},
    ProjectivePoint, Scalar,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ============================================================================
// Types (mirrored from sigil-mother-zkvm/src/types.rs for no_std compatibility)
// ============================================================================

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeygenInput {
    #[serde(with = "hex_bytes_32")]
    pub cold_shard: [u8; 32],
    #[serde(with = "hex_bytes_32")]
    pub agent_shard: [u8; 32],
    #[serde(with = "hex_bytes_32")]
    pub ceremony_nonce: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeygenOutput {
    #[serde(with = "hex_bytes_33")]
    pub master_pubkey: [u8; 33],
    #[serde(with = "hex_bytes_33")]
    pub cold_pubkey: [u8; 33],
    #[serde(with = "hex_bytes_33")]
    pub agent_pubkey: [u8; 33],
    #[serde(with = "hex_bytes_32")]
    pub ceremony_nonce: [u8; 32],
}

// ============================================================================
// Main Program
// ============================================================================

pub fn main() {
    // Read the input from the prover
    let input: KeygenInput = sp1_zkvm::io::read();

    // Compute the output
    let output = compute_keygen(&input).expect("Keygen computation failed");

    // Commit the output (public)
    sp1_zkvm::io::commit(&output);
}

fn compute_keygen(input: &KeygenInput) -> Result<KeygenOutput, &'static str> {
    // Convert shards to scalars
    let cold_scalar = Scalar::from_repr(input.cold_shard.into());
    if cold_scalar.is_none().into() {
        return Err("Invalid cold shard scalar");
    }
    let cold_scalar = cold_scalar.unwrap();

    let agent_scalar = Scalar::from_repr(input.agent_shard.into());
    if agent_scalar.is_none().into() {
        return Err("Invalid agent shard scalar");
    }
    let agent_scalar = agent_scalar.unwrap();

    // Compute public key points
    let cold_point = ProjectivePoint::GENERATOR * cold_scalar;
    let agent_point = ProjectivePoint::GENERATOR * agent_scalar;

    // Combined public key = cold_point + agent_point
    let combined_point = cold_point + agent_point;

    // Encode as compressed public keys
    let cold_affine = cold_point.to_affine();
    let cold_encoded = cold_affine.to_encoded_point(true);
    let cold_pubkey: [u8; 33] = cold_encoded
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode cold pubkey")?;

    let agent_affine = agent_point.to_affine();
    let agent_encoded = agent_affine.to_encoded_point(true);
    let agent_pubkey: [u8; 33] = agent_encoded
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode agent pubkey")?;

    let combined_affine = combined_point.to_affine();
    let combined_encoded = combined_affine.to_encoded_point(true);
    let master_pubkey: [u8; 33] = combined_encoded
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode master pubkey")?;

    Ok(KeygenOutput {
        master_pubkey,
        cold_pubkey,
        agent_pubkey,
        ceremony_nonce: input.ceremony_nonce,
    })
}
