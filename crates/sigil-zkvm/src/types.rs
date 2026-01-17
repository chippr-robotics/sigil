//! Types for zkVM signing operations

use serde::{Deserialize, Serialize};

/// Input to the zkVM signing program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningInput {
    // === Public Inputs ===
    /// Child public key (compressed, 33 bytes)
    pub child_pubkey: [u8; 33],

    /// Message hash to sign (32 bytes)
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
    pub r_point: [u8; 33],

    /// Party's nonce share k
    pub k_share: [u8; 32],

    /// Auxiliary value chi for signature completion
    pub chi: [u8; 32],
}

/// Output from the zkVM signing program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningOutput {
    /// The produced ECDSA signature (r || s, 64 bytes)
    pub signature: [u8; 64],

    /// The presignature index that was used
    pub presig_index: u32,

    /// The message hash that was signed (for verification)
    pub message_hash: [u8; 32],

    /// The public key used (for verification)
    pub child_pubkey: [u8; 33],
}
