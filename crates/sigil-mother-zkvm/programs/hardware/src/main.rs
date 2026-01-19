//! SP1 zkVM program for hardware wallet derivation
//!
//! Proves:
//! 1. Signature is valid for the device's public key and message
//! 2. Shard is correctly derived as `SHA256(domain || signature)`
//! 3. Shard's public key is correctly computed

#![no_main]
sp1_zkvm::entrypoint!(main);

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use k256::{
    ecdsa::{signature::Verifier, Signature, VerifyingKey},
    elliptic_curve::sec1::ToEncodedPoint,
    SecretKey,
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

mod hex_bytes_65 {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInput {
    #[serde(with = "hex_bytes_65")]
    pub signature: [u8; 65],
    #[serde(with = "hex_bytes_32")]
    pub derived_shard: [u8; 32],
    #[serde(with = "hex_bytes_65")]
    pub device_pubkey: [u8; 65],
    pub message: Vec<u8>,
    pub domain: Vec<u8>,
    pub is_cold_shard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareOutput {
    #[serde(with = "hex_bytes_33")]
    pub shard_pubkey: [u8; 33],
    #[serde(with = "hex_bytes_65")]
    pub device_pubkey: [u8; 65],
    #[serde(with = "hex_bytes_32")]
    pub message_hash: [u8; 32],
    pub is_cold_shard: bool,
}

// ============================================================================
// Main Program
// ============================================================================

pub fn main() {
    // Read the input from the prover
    let input: HardwareInput = sp1_zkvm::io::read();

    // Compute the output
    let output = compute_hardware(&input).expect("Hardware computation failed");

    // Commit the output (public)
    sp1_zkvm::io::commit(&output);
}

fn compute_hardware(input: &HardwareInput) -> Result<HardwareOutput, &'static str> {
    // 1. Verify signature
    verify_signature(&input.device_pubkey, &input.message, &input.signature)?;

    // 2. Verify shard derivation
    let expected_shard: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(&input.domain);
        hasher.update(&input.signature);
        hasher.finalize().into()
    };

    if expected_shard != input.derived_shard {
        return Err("Shard derivation mismatch");
    }

    // 3. Compute shard's public key
    let secret_key =
        SecretKey::from_bytes((&input.derived_shard).into()).map_err(|_| "Invalid secret key")?;

    let public_key = secret_key.public_key();
    let encoded = public_key.to_encoded_point(true);
    let shard_pubkey: [u8; 33] = encoded
        .as_bytes()
        .try_into()
        .map_err(|_| "Failed to encode shard pubkey")?;

    // 4. Compute message hash
    let message_hash: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(&input.message);
        hasher.finalize().into()
    };

    Ok(HardwareOutput {
        shard_pubkey,
        device_pubkey: input.device_pubkey,
        message_hash,
        is_cold_shard: input.is_cold_shard,
    })
}

fn verify_signature(
    pubkey: &[u8; 65],
    message: &[u8],
    signature: &[u8; 65],
) -> Result<(), &'static str> {
    // Parse the uncompressed public key
    let verifying_key =
        VerifyingKey::from_sec1_bytes(pubkey).map_err(|_| "Invalid public key")?;

    // Parse signature (r || s)
    let sig = Signature::from_slice(&signature[..64]).map_err(|_| "Invalid signature")?;

    // Hash the message
    let message_hash: [u8; 32] = {
        let mut hasher = Sha256::new();
        hasher.update(message);
        hasher.finalize().into()
    };

    // Verify
    verifying_key
        .verify(&message_hash, &sig)
        .map_err(|_| "Signature verification failed")?;

    Ok(())
}
