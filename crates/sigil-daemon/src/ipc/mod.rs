//! IPC server for CLI communication
//!
//! Provides a platform-agnostic interface for the CLI to communicate with the daemon.
//! Uses Unix domain sockets on Unix-like systems and named pipes on Windows.

mod client;
mod connection;
mod server;
mod types;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

// Public API
pub use client::IpcClient;
pub use server::IpcServer;
pub use types::{IpcRequest, IpcResponse};
