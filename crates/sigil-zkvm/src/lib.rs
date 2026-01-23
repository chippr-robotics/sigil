//! Sigil zkVM - SP1 signing program for MPC signatures
//!
//! This crate contains the program that runs inside SP1 zkVM to produce
//! provable ECDSA signatures from combined presignature shares.

#![no_std]

pub mod signing;
pub mod types;

pub use signing::{complete_presig, complete_presig_v2, verify_signature};
pub use types::{
    AccumulatorInput, NonMembershipWitnessInput, PresigShareInputV2, SigningInput, SigningInputV2,
    SigningOutput, SigningOutputV2,
};
