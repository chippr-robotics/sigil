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
//! - `ledger` - Enable Ledger hardware wallet support for secure key generation

pub mod ceremony;
pub mod error;
pub mod keygen;
pub mod ledger;
pub mod presig_gen;
pub mod reconciliation;
pub mod registry;
pub mod storage;

pub use ceremony::{CreateChildCeremony, ReconcileCeremony, RefillCeremony};
pub use error::{MotherError, Result};
pub use keygen::MasterKeyGenerator;
pub use ledger::LedgerDevice;
pub use presig_gen::PresigGenerator;
pub use registry::ChildRegistry;
pub use storage::MotherStorage;
