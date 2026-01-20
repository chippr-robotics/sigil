//! IPC protocol types
//!
//! Platform-agnostic message types for daemon-CLI communication.

use serde::{Deserialize, Serialize};
use sigil_core::types::{MessageHash, TxHash};

/// IPC request types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcRequest {
    /// Check if daemon is running
    Ping,

    /// Get disk status
    GetDiskStatus,

    /// Sign a message
    Sign {
        message_hash: String, // hex encoded
        chain_id: u32,
        description: String,
    },

    /// Update transaction hash after broadcast
    UpdateTxHash {
        presig_index: u32,
        tx_hash: String, // hex encoded
    },

    /// List all stored children
    ListChildren,

    /// Get remaining presigs for current disk
    GetPresigCount,

    /// Import agent master shard (agent's portion of master key)
    ImportAgentShard {
        agent_shard_hex: String, // hex encoded 32 bytes
    },

    /// Import child presignature shares
    ImportChildShares {
        shares_json: String, // JSON-encoded AgentChildData
        replace: bool,       // Replace existing shares if true
    },
}

/// IPC response types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcResponse {
    /// Success with no data
    Ok,

    /// Pong response
    Pong { version: String },

    /// Error response
    Error { message: String },

    /// Disk status response
    DiskStatus {
        detected: bool,
        child_id: Option<String>,
        presigs_remaining: Option<u32>,
        presigs_total: Option<u32>,
        days_until_expiry: Option<u32>,
        is_valid: Option<bool>,
    },

    /// Signing result
    SignResult {
        signature: String, // hex encoded
        presig_index: u32,
        proof_hash: String, // hex encoded
    },

    /// List of children
    Children { child_ids: Vec<String> },

    /// Presig count
    PresigCount { remaining: u32, total: u32 },
}

/// Parse a hex-encoded message hash
pub(super) fn parse_message_hash(s: &str) -> std::result::Result<MessageHash, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(s, &mut bytes).map_err(|e| e.to_string())?;
    Ok(MessageHash::new(bytes))
}

/// Parse a hex-encoded transaction hash
pub(super) fn parse_tx_hash(s: &str) -> std::result::Result<TxHash, String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(s, &mut bytes).map_err(|e| e.to_string())?;
    Ok(TxHash::new(bytes))
}
