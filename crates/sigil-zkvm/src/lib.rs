//! Sigil zkVM - SP1 signing program for MPC signatures
//!
//! This crate contains the program that runs inside SP1 zkVM to produce
//! provable ECDSA signatures from combined presignature shares.

#![no_std]

pub mod signing;
pub mod types;

pub use signing::{complete_presig, verify_signature};
pub use types::{SigningInput, SigningOutput};
