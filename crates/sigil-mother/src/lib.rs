//! Sigil Mother - Air-gapped mother device tooling
//!
//! This crate provides tools for the air-gapped mother device:
//! - Master shard generation and storage
//! - Child disk creation
//! - Presignature generation
//! - Reconciliation and refill
//! - Nullification
//!
//! # Optional Features
//!
//! - `ledger` - Enable Ledger hardware wallet support
//! - `trezor` - Enable Trezor hardware wallet support
//! - `pkcs11` - Enable PKCS#11 HSM support (YubiHSM, SoftHSM, etc.)
//! - `hardware-all` - Enable all hardware signer backends
//! - `zkvm` - Enable zkVM proving for mother operations
//! - `zkvm-mock` - Use mock provers for testing
//! - `zkvm-sp1` - Use real SP1 provers (requires SP1 toolchain)

pub mod ceremony;
pub mod error;
#[cfg(any(feature = "ledger", feature = "trezor", feature = "pkcs11"))]
pub mod hardware;
pub mod keygen;
pub mod ledger; // Backwards compatibility re-export
pub mod presig_gen;
pub mod reconciliation;
pub mod registry;
pub mod storage;
#[cfg(feature = "zkvm")]
pub mod zkvm;

pub use ceremony::{CreateChildCeremony, ReconcileCeremony, RefillCeremony};
pub use error::{MotherError, Result};
#[cfg(any(feature = "ledger", feature = "trezor", feature = "pkcs11"))]
pub use hardware::HardwareSigner;
pub use keygen::MasterKeyGenerator;
pub use presig_gen::PresigGenerator;
pub use registry::ChildRegistry;
pub use storage::MotherStorage;

#[cfg(feature = "zkvm")]
pub use zkvm::ProofGenerator;

// Backwards compatibility: re-export LedgerDevice from old location
#[cfg(feature = "ledger")]
pub use hardware::ledger::LedgerDevice;
