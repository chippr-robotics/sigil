//! SP1 zkVM program for child key derivation
//!
//! Proves: `child = HKDF(master, path)`, `child_pubkey = [child]*G`
//!
//! This program runs inside the SP1 zkVM and produces a proof that:
//! 1. The master shards combine to the claimed master public key
//! 2. Child shards are correctly derived using SHA256(master || path)
//! 3. The child public key is correctly computed from the child shards

#![no_main]
sp1_zkvm::entrypoint!(main);

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use k256::{
    elliptic_curve::{
        sec1::{FromEncodedPoint, ToEncodedPoint},
        PrimeField,
    },
    AffinePoint, EncodedPoint, ProjectivePoint, Scalar,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

// ============================================================================
// Types
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
pub struct DeriveInput {
    #[serde(with = "hex_bytes_32")]
    pub cold_master_shard: [u8; 32],
    #[serde(with = "hex_bytes_32")]
    pub agent_master_shard: [u8; 32],
    pub derivation_path: Vec<u8>,
    #[serde(with = "hex_bytes_33")]
    pub master_pubkey: [u8; 33],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeriveOutput {
    #[serde(with = "hex_bytes_33")]
    pub child_pubkey: [u8; 33],
    #[serde(with = "hex_bytes_33")]
    pub cold_child_pubkey: [u8; 33],
    #[serde(with = "hex_bytes_33")]
    pub agent_child_pubkey: [u8; 33],
    pub derivation_path: Vec<u8>,
    #[serde(with = "hex_bytes_33")]
    pub master_pubkey: [u8; 33],
}

// ============================================================================
// Main Program
// ============================================================================

pub fn main() {
    // Read the input from the prover
    let input: DeriveInput = sp1_zkvm::io::read();

    // Compute the output
    let output = compute_derive(&input).expect("Derive computation failed");

    // Commit the output (public)
    sp1_zkvm::io::commit(&output);
}

fn compute_derive(input: &DeriveInput) -> Result<DeriveOutput, &'static str> {
    // 1. Verify master public key
    let cold_master_scalar = Scalar::from_repr(input.cold_master_shard.into());
    if cold_master_scalar.is_none().into() {
        return Err("Invalid cold master shard");
    }
    let cold_master_scalar = cold_master_scalar.unwrap();

    let agent_master_scalar = Scalar::from_repr(input.agent_master_shard.into());
    if agent_master_scalar.is_none().into() {
        return Err("Invalid agent master shard");
    }
    let agent_master_scalar = agent_master_scalar.unwrap();

    // Compute expected master public key
    let cold_master_point = ProjectivePoint::GENERATOR * cold_master_scalar;
    let agent_master_point = ProjectivePoint::GENERATOR * agent_master_scalar;
    let expected_master = cold_master_point + agent_master_point;
    let expected_master_affine = expected_master.to_affine();
    let expected_master_bytes: [u8; 33] = expected_master_affine
        .to_encoded_point(true)
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode master pubkey")?;

    // Verify master public key matches
    if expected_master_bytes != input.master_pubkey {
        return Err("Master public key does not match provided shards");
    }

    // 2. Derive child shards using SHA256
    let cold_child_shard: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(&input.cold_master_shard);
        hasher.update(&input.derivation_path);
        hasher.finalize().into()
    };

    let agent_child_shard: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(&input.agent_master_shard);
        hasher.update(&input.derivation_path);
        hasher.finalize().into()
    };

    // 3. Convert to scalars
    let cold_child_scalar = Scalar::from_repr(cold_child_shard.into());
    if cold_child_scalar.is_none().into() {
        return Err("Invalid cold child scalar");
    }
    let cold_child_scalar = cold_child_scalar.unwrap();

    let agent_child_scalar = Scalar::from_repr(agent_child_shard.into());
    if agent_child_scalar.is_none().into() {
        return Err("Invalid agent child scalar");
    }
    let agent_child_scalar = agent_child_scalar.unwrap();

    // 4. Compute child public keys
    let cold_child_point = ProjectivePoint::GENERATOR * cold_child_scalar;
    let agent_child_point = ProjectivePoint::GENERATOR * agent_child_scalar;
    let child_point = cold_child_point + agent_child_point;

    // 5. Encode as compressed public keys
    let cold_child_affine = cold_child_point.to_affine();
    let cold_child_pubkey: [u8; 33] = cold_child_affine
        .to_encoded_point(true)
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode cold child pubkey")?;

    let agent_child_affine = agent_child_point.to_affine();
    let agent_child_pubkey: [u8; 33] = agent_child_affine
        .to_encoded_point(true)
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode agent child pubkey")?;

    let child_affine = child_point.to_affine();
    let child_pubkey: [u8; 33] = child_affine
        .to_encoded_point(true)
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode child pubkey")?;

    Ok(DeriveOutput {
        child_pubkey,
        cold_child_pubkey,
        agent_child_pubkey,
        derivation_path: input.derivation_path.clone(),
        master_pubkey: input.master_pubkey,
    })
}
