//! Distributed Key Generation (DKG) for FROST threshold signatures
//!
//! This module implements FROST's DKG protocol for generating key shares
//! without any single party ever seeing the full private key.
//!
//! # Overview
//!
//! The DKG is a 2-round protocol:
//!
//! **Round 1**: Each participant generates:
//! - A secret polynomial with random coefficients
//! - Commitments to those coefficients (Feldman VSS)
//! - A proof of knowledge of the secret
//!
//! **Round 2**: Each participant:
//! - Sends secret shares to other participants
//! - Verifies received shares against commitments
//! - Computes their final key share
//!
//! # Example (2-of-2 between Mother and Agent)
//!
//! ```ignore
//! use sigil_frost::dkg::{DkgParticipant, DkgRound1, DkgRound2};
//!
//! // Mother device (participant 1)
//! let mut mother = DkgParticipant::new(1, 2, 2)?;
//! let mother_r1 = mother.generate_round1()?;
//!
//! // Agent device (participant 2)
//! let mut agent = DkgParticipant::new(2, 2, 2)?;
//! let agent_r1 = agent.generate_round1()?;
//!
//! // Exchange Round 1 packages (via QR codes)
//! // ...
//!
//! // Mother receives agent's Round 1, generates Round 2
//! let mother_r2 = mother.generate_round2(&[agent_r1])?;
//!
//! // Agent receives mother's Round 1, generates Round 2
//! let agent_r2 = agent.generate_round2(&[mother_r1])?;
//!
//! // Exchange Round 2 packages (via QR codes)
//! // ...
//!
//! // Finalize
//! let mother_keys = mother.finalize(&[agent_r2])?;
//! let agent_keys = agent.finalize(&[mother_r2])?;
//!
//! // Both have the same group public key
//! assert_eq!(mother_keys.verifying_key, agent_keys.verifying_key);
//! ```

mod ceremony;
#[cfg(feature = "dkg")]
mod qr;
mod types;

pub use ceremony::{DkgCeremony, DkgKeyPackage, DkgOutput};
#[cfg(feature = "dkg")]
pub use qr::{DkgQrDecoder, DkgQrEncoder, QrPackage};
pub use types::{
    DkgConfig, DkgRound1Package, DkgRound2Package, DkgState, ParticipantId, ParticipantRole,
};

use crate::error::FrostError;

/// Result type for DKG operations
pub type DkgResult<T> = std::result::Result<T, FrostError>;

/// Trait for scheme-specific DKG implementations
pub trait FrostDkg: Sized {
    /// The key share type for this scheme
    type KeyShare;

    /// The verifying key type for this scheme
    type VerifyingKey;

    /// Start a new DKG ceremony
    fn new_ceremony(config: DkgConfig) -> DkgResult<DkgCeremony<Self>>;

    /// Generate Round 1 package
    fn generate_round1(ceremony: &mut DkgCeremony<Self>) -> DkgResult<DkgRound1Package>;

    /// Process received Round 1 packages and generate Round 2 packages
    fn generate_round2(
        ceremony: &mut DkgCeremony<Self>,
        received_r1: &[DkgRound1Package],
    ) -> DkgResult<Vec<DkgRound2Package>>;

    /// Finalize the ceremony with received Round 2 packages
    fn finalize(
        ceremony: DkgCeremony<Self>,
        received_r2: &[DkgRound2Package],
    ) -> DkgResult<DkgOutput<Self>>;
}
