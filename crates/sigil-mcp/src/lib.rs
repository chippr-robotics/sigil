//! Sigil MCP Server
//!
//! This crate implements a Model Context Protocol (MCP) server for Sigil,
//! enabling any MCP-compatible AI agent to securely sign blockchain transactions
//! using Sigil's MPC infrastructure.
//!
//! # Features
//!
//! - **Tools**: Signing operations for EVM chains (ECDSA) and FROST chains
//!   (Bitcoin Taproot, Solana, Cosmos, Zcash)
//! - **Resources**: Disk status, presignature counts, supported chains
//! - **Prompts**: Guided workflows for common signing operations
//!
//! # Transport Support
//!
//! - **stdio**: Standard input/output (for Claude Desktop, VS Code, etc.)
//! - **HTTP+SSE**: Coming soon (for web clients and remote agents)
//!
//! # Example Usage
//!
//! ```no_run
//! use sigil_mcp::McpServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServer::new();
//!     server.run_stdio().await.expect("Server failed");
//! }
//! ```
//!
//! # Protocol Version
//!
//! This implementation targets MCP version 2025-11-25.

pub mod handlers;
pub mod invariants;
pub mod prompts;
pub mod protocol;
pub mod resources;
pub mod server;
pub mod tools;
pub mod transport;

pub use protocol::{
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, JsonRpcError,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, MCP_PROTOCOL_VERSION,
    ServerCapabilities, ServerInfo, Tool, ToolContent, ToolsCallResult,
};
pub use server::McpServer;
pub use tools::{DiskState, ToolContext};
