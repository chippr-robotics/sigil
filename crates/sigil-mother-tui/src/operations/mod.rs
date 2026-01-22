//! Operation orchestration and ceremony execution
//!
//! This module wraps sigil-mother ceremonies and provides async execution
//! with progress reporting for the TUI.

use anyhow::Result;

/// Ceremony execution wrapper
pub mod ceremony;
/// Disk I/O operations
pub mod disk_io;
/// Proof generation (optional zkVM)
pub mod proof;

pub use ceremony::CeremonyExecutor;
