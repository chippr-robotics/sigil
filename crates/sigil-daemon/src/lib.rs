//! Sigil Daemon - System daemon for disk watching and MPC signing
//!
//! This crate provides:
//! - Disk detection and monitoring via udev
//! - Agent shard storage and management
//! - Signing operations with zkVM proof generation
//! - Memory management with plain markdown storage
//! - IPC server for CLI communication

pub mod agent_store;
pub mod config;
pub mod disk_watcher;
pub mod error;
pub mod ipc;
pub mod memory_manager;
pub mod signer;

pub use agent_store::AgentStore;
pub use config::DaemonConfig;
pub use disk_watcher::DiskWatcher;
pub use error::{DaemonError, Result};
pub use ipc::IpcServer;
pub use memory_manager::MemoryManager;
pub use signer::Signer;
