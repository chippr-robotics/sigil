//! Sigil Core - MPC-Secured Floppy Disk Signing System
//!
//! This library provides the core types, disk format, and cryptographic operations
//! for the Sigil MPC signing system.

pub mod error;
pub mod blockchain;
pub mod mpc;
pub mod presig;
pub mod disk;
pub mod hd;

pub use error::{Result, SigilError};
