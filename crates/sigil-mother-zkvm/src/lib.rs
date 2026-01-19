//! Sigil Mother zkVM - SP1 proving for mother device operations
//!
//! This crate provides zero-knowledge proof generation and verification for
//! air-gapped mother device operations:
//!
//! - **Master Key Generation**: Prove `PubKey = [cold_shard]*G + [agent_shard]*G`
//! - **Child Key Derivation**: Prove `child = HKDF(master, path)`, `child_pubkey = [child]*G`
//! - **Presignature Batches**: Prove `R = (k_cold + k_agent)*G` for batch with Merkle commitment
//! - **Hardware Derivation**: Prove signature verification + SHA256 derivation
//!
//! # Features
//!
//! - `sp1-prover` - Enable real SP1 proving (requires SP1 toolchain)
//! - `mock` - Enable mock provers for testing without SP1
//!
//! # Architecture
//!
//! ```text
//! sigil-mother-zkvm/
//!   src/
//!     lib.rs                    # This file - exports
//!     types.rs                  # Input/Output types for all programs
//!     error.rs                  # Error types
//!     merkle.rs                 # Merkle tree utilities for batch proofs
//!     provers/
//!       mod.rs
//!       keygen.rs               # Master key prover
//!       derive.rs               # Child derivation prover
//!       batch_presig.rs         # Batch presig prover
//!       hardware.rs             # Hardware wallet prover
//!     verifiers/
//!       mod.rs
//!       keygen.rs               # Master key verifier
//!       derive.rs               # Child derivation verifier
//!       batch_presig.rs         # Batch presig verifier
//!       hardware.rs             # Hardware wallet verifier
//!     storage/
//!       mod.rs                  # Proof storage and manifest
//!   programs/
//!     keygen/                   # SP1 program for keygen
//!     derive/                   # SP1 program for derivation
//!     batch/                    # SP1 program for batch presigs
//!     hardware/                 # SP1 program for hardware derivation
//! ```

pub mod error;
pub mod merkle;
pub mod provers;
pub mod storage;
pub mod types;
pub mod verifiers;

pub use error::{Result, ZkvmError};
pub use merkle::MerkleTree;
pub use provers::{BatchPresigProver, DeriveProver, HardwareProver, KeygenProver, MotherProver};
pub use storage::{ProofManifest, ProofStorage};
pub use types::*;
