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

pub mod ceremony;
pub mod error;
pub mod hardware;
pub mod keygen;
pub mod ledger; // Backwards compatibility re-export
pub mod presig_gen;
pub mod reconciliation;
pub mod registry;
pub mod storage;

pub use ceremony::{CreateChildCeremony, ReconcileCeremony, RefillCeremony};
pub use error::{MotherError, Result};
pub use hardware::HardwareSigner;
pub use keygen::MasterKeyGenerator;
pub use presig_gen::PresigGenerator;
pub use registry::ChildRegistry;
pub use storage::MotherStorage;

// Backwards compatibility: re-export LedgerDevice from old location
#[cfg(feature = "ledger")]
pub use hardware::ledger::LedgerDevice;
