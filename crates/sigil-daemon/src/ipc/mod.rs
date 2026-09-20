//! IPC server for CLI communication
//!
//! Provides a platform-agnostic interface for the CLI to communicate with the daemon.
//! Uses Unix domain sockets on Unix-like systems and named pipes on Windows.

mod client;
pub(crate) mod connection;
mod server;
mod types;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

// Public API
pub use client::IpcClient;
pub use connection::{BindOptions, DEFAULT_SOCKET_MODE};
pub use server::IpcServer;
pub use types::{IpcRequest, IpcResponse};
