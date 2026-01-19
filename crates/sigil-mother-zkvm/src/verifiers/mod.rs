//! Verifier implementations for mother device proofs
//!
//! This module provides verifiers for checking zero-knowledge proofs of:
//! - Master key generation
//! - Child key derivation
//! - Batch presignature generation
//! - Hardware wallet derivation
//!
//! Verifiers can check both mock proofs (for testing) and real SP1 proofs.

pub mod batch_presig;
pub mod derive;
pub mod hardware;
pub mod keygen;

pub use batch_presig::BatchPresigVerifier;
pub use derive::DeriveVerifier;
pub use hardware::HardwareVerifier;
pub use keygen::KeygenVerifier;

use crate::error::{Result, ZkvmError};
use crate::types::*;

/// Unified interface for verifying mother device proofs
pub trait MotherVerifier: Send + Sync {
    /// Verify a keygen proof
    fn verify_keygen(&self, proof: &[u8], expected_output: &KeygenOutput) -> Result<bool>;

    /// Verify a derive proof
    fn verify_derive(&self, proof: &[u8], expected_output: &DeriveOutput) -> Result<bool>;

    /// Verify a batch presig proof
    fn verify_batch_presig(
        &self,
        proof: &[u8],
        expected_output: &BatchPresigOutput,
    ) -> Result<bool>;

    /// Verify a hardware proof
    fn verify_hardware(&self, proof: &[u8], expected_output: &HardwareOutput) -> Result<bool>;
}

/// Combined verifier that handles both mock and SP1 proofs
pub struct CombinedVerifier {
    #[cfg(feature = "sp1-prover")]
    keygen_vkey: Option<sp1_sdk::SP1VerifyingKey>,
    #[cfg(feature = "sp1-prover")]
    derive_vkey: Option<sp1_sdk::SP1VerifyingKey>,
    #[cfg(feature = "sp1-prover")]
    batch_vkey: Option<sp1_sdk::SP1VerifyingKey>,
    #[cfg(feature = "sp1-prover")]
    hardware_vkey: Option<sp1_sdk::SP1VerifyingKey>,
}

impl CombinedVerifier {
    /// Create a new verifier (for mock proofs only)
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "sp1-prover")]
            keygen_vkey: None,
            #[cfg(feature = "sp1-prover")]
            derive_vkey: None,
            #[cfg(feature = "sp1-prover")]
            batch_vkey: None,
            #[cfg(feature = "sp1-prover")]
            hardware_vkey: None,
        }
    }

    /// Create a verifier with SP1 verification keys
    #[cfg(feature = "sp1-prover")]
    pub fn with_sp1_keys(
        keygen_vkey: sp1_sdk::SP1VerifyingKey,
        derive_vkey: sp1_sdk::SP1VerifyingKey,
        batch_vkey: sp1_sdk::SP1VerifyingKey,
        hardware_vkey: sp1_sdk::SP1VerifyingKey,
    ) -> Self {
        Self {
            keygen_vkey: Some(keygen_vkey),
            derive_vkey: Some(derive_vkey),
            batch_vkey: Some(batch_vkey),
            hardware_vkey: Some(hardware_vkey),
        }
    }
}

impl Default for CombinedVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MotherVerifier for CombinedVerifier {
    fn verify_keygen(&self, proof: &[u8], expected_output: &KeygenOutput) -> Result<bool> {
        #[cfg(feature = "mock")]
        {
            use crate::provers::mock;
            if mock::is_mock_proof(proof) {
                let extracted: KeygenOutput = mock::extract_mock_output(proof)?;
                return Ok(extracted == *expected_output);
            }
        }

        #[cfg(feature = "sp1-prover")]
        {
            if let Some(ref vkey) = self.keygen_vkey {
                return keygen::KeygenVerifier::verify_sp1(proof, vkey, expected_output);
            }
        }

        Err(ZkvmError::FeatureNotEnabled(
            "Proof verification requires mock or sp1-prover feature".into(),
        ))
    }

    fn verify_derive(&self, proof: &[u8], expected_output: &DeriveOutput) -> Result<bool> {
        #[cfg(feature = "mock")]
        {
            use crate::provers::mock;
            if mock::is_mock_proof(proof) {
                let extracted: DeriveOutput = mock::extract_mock_output(proof)?;
                return Ok(extracted == *expected_output);
            }
        }

        #[cfg(feature = "sp1-prover")]
        {
            if let Some(ref vkey) = self.derive_vkey {
                return derive::DeriveVerifier::verify_sp1(proof, vkey, expected_output);
            }
        }

        Err(ZkvmError::FeatureNotEnabled(
            "Proof verification requires mock or sp1-prover feature".into(),
        ))
    }

    fn verify_batch_presig(
        &self,
        proof: &[u8],
        expected_output: &BatchPresigOutput,
    ) -> Result<bool> {
        #[cfg(feature = "mock")]
        {
            use crate::provers::mock;
            if mock::is_mock_proof(proof) {
                let extracted: BatchPresigOutput = mock::extract_mock_output(proof)?;
                return Ok(extracted == *expected_output);
            }
        }

        #[cfg(feature = "sp1-prover")]
        {
            if let Some(ref vkey) = self.batch_vkey {
                return batch_presig::BatchPresigVerifier::verify_sp1(proof, vkey, expected_output);
            }
        }

        Err(ZkvmError::FeatureNotEnabled(
            "Proof verification requires mock or sp1-prover feature".into(),
        ))
    }

    fn verify_hardware(&self, proof: &[u8], expected_output: &HardwareOutput) -> Result<bool> {
        #[cfg(feature = "mock")]
        {
            use crate::provers::mock;
            if mock::is_mock_proof(proof) {
                let extracted: HardwareOutput = mock::extract_mock_output(proof)?;
                return Ok(extracted == *expected_output);
            }
        }

        #[cfg(feature = "sp1-prover")]
        {
            if let Some(ref vkey) = self.hardware_vkey {
                return hardware::HardwareVerifier::verify_sp1(proof, vkey, expected_output);
            }
        }

        Err(ZkvmError::FeatureNotEnabled(
            "Proof verification requires mock or sp1-prover feature".into(),
        ))
    }
}
