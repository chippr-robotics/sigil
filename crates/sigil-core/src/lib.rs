//! Sigil Core - Shared types, disk format, and cryptographic primitives
//!
//! This crate provides the foundational types and utilities for the Sigil
//! MPC-secured floppy disk signing system.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod accumulator;
pub mod agent;
pub mod child;
pub mod crypto;
pub mod disk;
pub mod error;
pub mod expiry;
pub mod presig;
pub mod types;
pub mod usage;

pub use accumulator::{
    AccumulatorWitness, NonMembershipWitness, PresigWithAccumulatorBinding, RsaAccumulator,
    StoredAccumulator, RSA_MODULUS_SIZE,
};
pub use agent::{AgentId, AgentMetadata, AgentRegistryEntry, AgentStatus};
pub use child::{ChildStatus, NullificationReason};
pub use crypto::{ChildKeyPair, DerivationPath, PublicKey};
pub use disk::{DiskFormat, DiskHeader, DISK_MAGIC, PRESIG_TABLE_OFFSET, USAGE_LOG_OFFSET};
pub use error::{Error, Result};
pub use expiry::DiskExpiry;
pub use presig::{PresigColdShare, PresigStatus, PresigTableEntry};
pub use types::{ChildId, MessageHash, Signature, TxHash, ZkProofHash};
pub use usage::UsageLogEntry;

/// Disk format version
pub const VERSION: u32 = 1;

/// Maximum number of presignatures per disk
pub const MAX_PRESIGS: u32 = 1000;

/// Size of each presig entry in bytes
pub const PRESIG_ENTRY_SIZE: usize = 256;

/// Default presignature validity in days
pub const PRESIG_VALIDITY_DAYS: u32 = 30;

/// Default reconciliation deadline in days
pub const RECONCILIATION_DEADLINE_DAYS: u32 = 45;

/// Maximum uses before forced reconciliation
pub const MAX_USES_BEFORE_RECONCILE: u32 = 500;

/// Warning threshold in days before expiry
pub const WARNING_THRESHOLD_DAYS: u32 = 7;

/// Emergency reserve presigs (cannot be used normally)
pub const EMERGENCY_RESERVE: u32 = 50;
