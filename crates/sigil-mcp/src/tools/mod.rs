//! Sigil MCP Tool definitions
//!
//! This module defines all the tools that the Sigil MCP server exposes.

mod check_disk;
mod get_address;
mod sign_evm;
mod sign_frost;
mod update_tx_hash;

use crate::protocol::{Tool, ToolAnnotations, ToolContent, ToolsCallResult};
use std::sync::Arc;

/// Tool execution context
pub struct ToolContext {
    /// Disk watcher for checking disk status
    pub disk_state: Arc<tokio::sync::RwLock<DiskState>>,
}

/// Current disk state (simplified for MCP server)
#[derive(Debug, Clone, Default)]
pub struct DiskState {
    pub detected: bool,
    pub child_id: Option<String>,
    pub scheme: Option<String>,
    pub presigs_remaining: Option<u32>,
    pub presigs_total: Option<u32>,
    pub days_until_expiry: Option<u32>,
    pub is_valid: Option<bool>,
    pub public_key: Option<String>,
}

impl DiskState {
    /// Create a mock disk state for testing
    pub fn mock_detected() -> Self {
        Self {
            detected: true,
            child_id: Some("7a3f2c1b".to_string()),
            scheme: Some("ecdsa".to_string()),
            presigs_remaining: Some(847),
            presigs_total: Some(1000),
            days_until_expiry: Some(12),
            is_valid: Some(true),
            public_key: Some(
                "0x04abc123def456789012345678901234567890123456789012345678901234567890".to_string(),
            ),
        }
    }

    /// Create a state with no disk detected
    pub fn no_disk() -> Self {
        Self::default()
    }
}

/// Get all tool definitions
pub fn get_all_tools() -> Vec<Tool> {
    vec![
        check_disk::tool_definition(),
        sign_evm::tool_definition(),
        sign_frost::tool_definition(),
        get_address::tool_definition(),
        update_tx_hash::tool_definition(),
        list_schemes_tool_definition(),
        get_presig_count_tool_definition(),
    ]
}

/// List schemes tool definition
fn list_schemes_tool_definition() -> Tool {
    Tool {
        name: "sigil_list_schemes".to_string(),
        title: Some("List Signature Schemes".to_string()),
        description: "List all supported signature schemes and their associated blockchains."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
        output_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "schemes": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "description": { "type": "string" },
                            "chains": { "type": "array", "items": { "type": "string" } }
                        }
                    }
                }
            }
        })),
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

/// Get presig count tool definition
fn get_presig_count_tool_definition() -> Tool {
    Tool {
        name: "sigil_get_presig_count".to_string(),
        title: Some("Get Presignature Count".to_string()),
        description: "Get the number of remaining and total presignatures on the current disk."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
        output_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "remaining": { "type": "integer" },
                "total": { "type": "integer" },
                "percentage": { "type": "number" }
            }
        })),
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

/// Execute list schemes tool
pub async fn execute_list_schemes() -> ToolsCallResult {
    let schemes = serde_json::json!({
        "schemes": [
            {
                "name": "ecdsa",
                "description": "ECDSA on secp256k1 - Ethereum and EVM-compatible chains",
                "chains": ["Ethereum", "Polygon", "Arbitrum", "Optimism", "Base", "BSC", "Avalanche"]
            },
            {
                "name": "taproot",
                "description": "BIP-340 Schnorr signatures - Bitcoin Taproot",
                "chains": ["Bitcoin (Taproot)"]
            },
            {
                "name": "ed25519",
                "description": "Ed25519 signatures - Solana, Cosmos, and others",
                "chains": ["Solana", "Cosmos", "Near", "Polkadot", "Cardano"]
            },
            {
                "name": "ristretto255",
                "description": "Ristretto255 signatures - Zcash shielded transactions",
                "chains": ["Zcash (shielded)"]
            }
        ]
    });

    ToolsCallResult::success_with_structured(vec![ToolContent::json(&schemes)], schemes)
}

/// Execute get presig count tool
pub async fn execute_get_presig_count(ctx: &ToolContext) -> ToolsCallResult {
    let state = ctx.disk_state.read().await;

    if !state.detected {
        return ToolsCallResult::error("No signing disk detected. Please insert your Sigil disk.");
    }

    let remaining = state.presigs_remaining.unwrap_or(0);
    let total = state.presigs_total.unwrap_or(0);
    let percentage = if total > 0 {
        (remaining as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let result = serde_json::json!({
        "remaining": remaining,
        "total": total,
        "percentage": percentage
    });

    ToolsCallResult::success_with_structured(
        vec![ToolContent::text(format!(
            "Presignatures: {}/{} ({:.1}% remaining)",
            remaining, total, percentage
        ))],
        result,
    )
}

/// Execute a tool by name
pub async fn execute_tool(
    ctx: &ToolContext,
    name: &str,
    arguments: serde_json::Value,
) -> ToolsCallResult {
    match name {
        "sigil_check_disk" => check_disk::execute(ctx).await,
        "sigil_sign_evm" => sign_evm::execute(ctx, arguments).await,
        "sigil_sign_frost" => sign_frost::execute(ctx, arguments).await,
        "sigil_get_address" => get_address::execute(ctx, arguments).await,
        "sigil_update_tx_hash" => update_tx_hash::execute(ctx, arguments).await,
        "sigil_list_schemes" => execute_list_schemes().await,
        "sigil_get_presig_count" => execute_get_presig_count(ctx).await,
        _ => ToolsCallResult::error(format!("Unknown tool: {}", name)),
    }
}
