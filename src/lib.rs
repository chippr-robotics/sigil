//! Sigil - A physical containment system for agentic MPC management
//!
//! This library provides tools for managing keyshards stored on physical media
//! (such as floppy disks) and integrating with blockchain transaction systems
//! via the Claude CLI.

pub mod keyshard;
pub mod storage;
pub mod blockchain;
pub mod crypto;
pub mod error;

pub use error::{Result, SigilError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_import() {
        // Basic sanity test
        assert!(true);
    }
}
