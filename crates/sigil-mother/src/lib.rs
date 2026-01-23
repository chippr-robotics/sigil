//! Sigil Mother - Air-gapped mother device tooling
//!
//! This crate provides tools for the air-gapped mother device:
//! - Master shard generation and storage
//! - Child disk creation
//! - Presignature generation
//! - Reconciliation and refill
//! - Nullification
//! - Agent registry and management
//!
//! # Security Model
//!
//! **The mother device is protected by PIN-based authentication.**
//!
//! All access to the master shard MUST go through the `auth` module.
//! The master shard is encrypted at rest using ChaCha20-Poly1305 with
//! a key derived from the PIN via Argon2id.
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

pub mod accumulator_publish;
pub mod agent_registry;
pub mod agent_shard_encryption;
pub mod auth;
pub mod ceremony;
pub mod disk_ops;
pub mod error;
#[cfg(any(feature = "ledger", feature = "trezor", feature = "pkcs11"))]
pub mod hardware;
pub mod keygen;
pub mod ledger; // Backwards compatibility re-export
pub mod nullification;
pub mod presig_gen;
pub mod reconciliation;
pub mod registry;
pub mod storage;
#[cfg(feature = "zkvm")]
pub mod zkvm;

pub use accumulator_publish::{AccumulatorExport, AccumulatorPublisher};
pub use agent_registry::AgentRegistry;
pub use agent_shard_encryption::{
    decrypt_agent_shard, encode_for_qr, encrypt_agent_shard, AgentShardData, EncryptedAgentShard,
    Passcode, ENCRYPTED_SHARD_PREFIX,
};
pub use auth::{
    AuthError, AuthState, EncryptedMotherStorage, LockoutPolicy, PinConfig, PinManager, Session,
    SessionConfig, MAX_PIN_LENGTH, MIN_PIN_LENGTH,
};
pub use ceremony::{CreateChildCeremony, ReconcileCeremony, RefillCeremony};
pub use disk_ops::{
    get_device_info, get_mount_point, list_all_block_devices, list_removable_devices, BlockDevice,
    DiskStatus, FloppyManager, FormatType, MountMethod, FLOPPY_SIZE_144MB, FLOPPY_SIZE_TOLERANCE,
};
pub use error::{MotherError, Result};
#[cfg(any(feature = "ledger", feature = "trezor", feature = "pkcs11"))]
pub use hardware::HardwareSigner;
pub use keygen::MasterKeyGenerator;
pub use nullification::{NullificationManager, NullificationResult};
pub use presig_gen::PresigGenerator;
pub use registry::ChildRegistry;
pub use storage::MotherStorage;

#[cfg(feature = "zkvm")]
pub use zkvm::ProofGenerator;

// Backwards compatibility: re-export LedgerDevice from old location
#[cfg(feature = "ledger")]
pub use hardware::ledger::LedgerDevice;
