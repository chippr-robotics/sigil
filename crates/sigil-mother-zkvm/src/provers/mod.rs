//! Prover implementations for mother device operations
//!
//! This module provides provers for generating zero-knowledge proofs of:
//! - Master key generation
//! - Child key derivation
//! - Batch presignature generation
//! - Hardware wallet derivation
//!
//! Two implementations are provided:
//! - Mock provers (always available) - for testing and development
//! - SP1 provers (feature-gated) - for real proof generation

pub mod batch_presig;
pub mod derive;
pub mod hardware;
pub mod keygen;

#[cfg(feature = "mock")]
pub mod mock;

pub use batch_presig::BatchPresigProver;
pub use derive::DeriveProver;
pub use hardware::HardwareProver;
pub use keygen::KeygenProver;

use crate::error::Result;
use crate::types::*;

/// Unified interface for all mother device provers
///
/// This trait provides a single interface for generating proofs of all
/// mother device operations. Implementations can use mock proofs for
/// testing or real SP1 proofs for production.
pub trait MotherProver: Send + Sync {
    /// Generate proof of master key generation
    fn prove_keygen(&self, input: KeygenInput) -> Result<(KeygenOutput, Vec<u8>)>;

    /// Generate proof of child key derivation
    fn prove_derive(&self, input: DeriveInput) -> Result<(DeriveOutput, Vec<u8>)>;

    /// Generate proof of batch presignature generation
    fn prove_batch_presig(&self, input: BatchPresigInput) -> Result<(BatchPresigOutput, Vec<u8>)>;

    /// Generate proof of hardware wallet derivation
    fn prove_hardware(&self, input: HardwareInput) -> Result<(HardwareOutput, Vec<u8>)>;

    /// Check if this is a mock prover
    fn is_mock(&self) -> bool;
}

/// Combined prover that uses the appropriate backend based on configuration
pub struct CombinedProver {
    keygen: Box<dyn KeygenProverTrait>,
    derive: Box<dyn DeriveProverTrait>,
    batch_presig: Box<dyn BatchPresigProverTrait>,
    hardware: Box<dyn HardwareProverTrait>,
    is_mock: bool,
}

impl CombinedProver {
    /// Create a new mock prover
    #[cfg(feature = "mock")]
    pub fn mock() -> Self {
        Self {
            keygen: Box::new(mock::MockKeygenProver),
            derive: Box::new(mock::MockDeriveProver),
            batch_presig: Box::new(mock::MockBatchPresigProver),
            hardware: Box::new(mock::MockHardwareProver),
            is_mock: true,
        }
    }

    /// Create a new SP1 prover
    #[cfg(feature = "sp1-prover")]
    pub fn sp1() -> Result<Self> {
        Ok(Self {
            keygen: Box::new(keygen::Sp1KeygenProver::new()?),
            derive: Box::new(derive::Sp1DeriveProver::new()?),
            batch_presig: Box::new(batch_presig::Sp1BatchPresigProver::new()?),
            hardware: Box::new(hardware::Sp1HardwareProver::new()?),
            is_mock: false,
        })
    }
}

impl MotherProver for CombinedProver {
    fn prove_keygen(&self, input: KeygenInput) -> Result<(KeygenOutput, Vec<u8>)> {
        self.keygen.prove(input)
    }

    fn prove_derive(&self, input: DeriveInput) -> Result<(DeriveOutput, Vec<u8>)> {
        self.derive.prove(input)
    }

    fn prove_batch_presig(&self, input: BatchPresigInput) -> Result<(BatchPresigOutput, Vec<u8>)> {
        self.batch_presig.prove(input)
    }

    fn prove_hardware(&self, input: HardwareInput) -> Result<(HardwareOutput, Vec<u8>)> {
        self.hardware.prove(input)
    }

    fn is_mock(&self) -> bool {
        self.is_mock
    }
}

/// Trait for keygen provers
pub trait KeygenProverTrait: Send + Sync {
    fn prove(&self, input: KeygenInput) -> Result<(KeygenOutput, Vec<u8>)>;
}

/// Trait for derive provers
pub trait DeriveProverTrait: Send + Sync {
    fn prove(&self, input: DeriveInput) -> Result<(DeriveOutput, Vec<u8>)>;
}

/// Trait for batch presig provers
pub trait BatchPresigProverTrait: Send + Sync {
    fn prove(&self, input: BatchPresigInput) -> Result<(BatchPresigOutput, Vec<u8>)>;
}

/// Trait for hardware provers
pub trait HardwareProverTrait: Send + Sync {
    fn prove(&self, input: HardwareInput) -> Result<(HardwareOutput, Vec<u8>)>;
}
