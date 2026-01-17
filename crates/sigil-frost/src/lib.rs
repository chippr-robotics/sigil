//! # Sigil FROST
//!
//! FROST (Flexible Round-Optimized Schnorr Threshold) signature support for Sigil.
//!
//! This crate provides threshold Schnorr signatures for multiple curves:
//! - **secp256k1-tr**: Bitcoin Taproot (BIP-340)
//! - **ed25519**: Solana, Cosmos, and other Ed25519 chains
//! - **ristretto255**: Foundation for Zcash shielded transactions
//!
//! ## Architecture
//!
//! FROST uses a two-round signing protocol with pre-processing support:
//!
//! 1. **Round 1 (Pre-processing)**: Generate nonces and commitments
//!    - Can be done offline during child disk creation
//!    - Stored on disk as "FROST presignatures"
//!
//! 2. **Round 2 (Signing)**: Generate signature shares
//!    - Uses pre-generated nonces
//!    - Requires the message to sign
//!
//! ## Presignature Model
//!
//! Like ECDSA presignatures, FROST nonces can be pre-generated and stored:
//!
//! ```text
//! Mother Device (offline)     Child Disk              Agent Device
//! ─────────────────────────   ──────────────────      ─────────────
//! Generate key shares    ───► cold_key_share          agent_key_share
//! Pre-generate nonces    ───► frost_nonces[1000]      frost_commitments
//! ```
//!
//! Each nonce can only be used once (same security model as ECDSA presigs).

use zeroize::Zeroize;

pub mod error;
pub mod presig;
pub mod traits;

#[cfg(feature = "taproot")]
pub mod taproot;

#[cfg(feature = "ed25519")]
pub mod ed25519;

#[cfg(feature = "ristretto255")]
pub mod ristretto255;

pub mod dkg;

pub use dkg::{DkgCeremony, DkgConfig, DkgKeyPackage, DkgOutput, DkgRound1Package, DkgRound2Package};
pub use error::{FrostError, Result};
pub use presig::{FrostNonce, FrostPresig, FrostPresigBatch};
pub use traits::{FrostCipherSuite, FrostKeyGen, FrostSigner};

/// Signature scheme identifier for disk format
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    bitcode::Encode,
    bitcode::Decode,
)]
#[repr(u8)]
pub enum SignatureScheme {
    /// secp256k1 ECDSA (legacy, current implementation)
    Ecdsa = 0,
    /// secp256k1 Schnorr (Bitcoin Taproot, BIP-340)
    Taproot = 1,
    /// Ed25519 EdDSA (Solana, Cosmos)
    Ed25519 = 2,
    /// Ristretto255 (Zcash foundation)
    Ristretto255 = 3,
}

impl SignatureScheme {
    /// Get the scheme from a u8 value
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Ecdsa),
            1 => Some(Self::Taproot),
            2 => Some(Self::Ed25519),
            3 => Some(Self::Ristretto255),
            _ => None,
        }
    }

    /// Get the human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ecdsa => "ECDSA (secp256k1)",
            Self::Taproot => "Schnorr/Taproot (secp256k1)",
            Self::Ed25519 => "EdDSA (Ed25519)",
            Self::Ristretto255 => "Schnorr (Ristretto255)",
        }
    }

    /// Get supported blockchains for this scheme
    pub fn supported_chains(&self) -> &'static [&'static str] {
        match self {
            Self::Ecdsa => &[
                "Ethereum",
                "Bitcoin (legacy)",
                "BSC",
                "Polygon",
                "Avalanche",
            ],
            Self::Taproot => &["Bitcoin (Taproot)"],
            Self::Ed25519 => &["Solana", "Cosmos", "Near", "Polkadot", "Cardano"],
            Self::Ristretto255 => &["Zcash (shielded)"],
        }
    }
}

/// Key share for a participant in FROST
#[derive(Clone)]
pub struct KeyShare {
    /// The signature scheme this key is for
    pub scheme: SignatureScheme,
    /// Serialized key share (scheme-specific format)
    pub data: Vec<u8>,
    /// Participant identifier
    pub identifier: u16,
}

impl std::fmt::Debug for KeyShare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyShare")
            .field("scheme", &self.scheme)
            .field("data", &"[REDACTED]")
            .field("identifier", &self.identifier)
            .finish()
    }
}

impl zeroize::Zeroize for KeyShare {
    fn zeroize(&mut self) {
        self.data.zeroize();
    }
}

impl Drop for KeyShare {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl KeyShare {
    /// Create a new key share
    pub fn new(scheme: SignatureScheme, data: Vec<u8>, identifier: u16) -> Self {
        Self {
            scheme,
            data,
            identifier,
        }
    }
}

/// Public key for verification
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerifyingKey {
    /// The signature scheme
    pub scheme: SignatureScheme,
    /// Serialized public key
    pub data: Vec<u8>,
}

impl VerifyingKey {
    /// Create a new verifying key
    pub fn new(scheme: SignatureScheme, data: Vec<u8>) -> Self {
        Self { scheme, data }
    }

    /// Get the public key as hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.data)
    }
}

/// A complete FROST signature
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrostSignature {
    /// The signature scheme
    pub scheme: SignatureScheme,
    /// Serialized signature
    pub data: Vec<u8>,
}

impl FrostSignature {
    /// Create a new signature
    pub fn new(scheme: SignatureScheme, data: Vec<u8>) -> Self {
        Self { scheme, data }
    }

    /// Get the signature as hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.data)
    }
}
