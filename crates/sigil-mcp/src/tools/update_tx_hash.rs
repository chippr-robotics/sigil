//! Update transaction hash tool

use crate::protocol::{Tool, ToolAnnotations, ToolContent, ToolsCallResult};
use serde::Deserialize;

use super::ToolContext;

/// Update tx hash input parameters
#[derive(Debug, Deserialize)]
pub struct UpdateTxHashParams {
    /// Presig index from the signing response
    pub presig_index: u32,

    /// Actual transaction hash after broadcast
    pub tx_hash: String,
}

/// Get the tool definition
pub fn tool_definition() -> Tool {
    Tool {
        name: "sigil_update_tx_hash".to_string(),
        title: Some("Update Transaction Hash".to_string()),
        description:
            "After broadcasting a signed transaction, record the actual transaction hash in the \
             disk's audit log. This links the presignature index to the on-chain transaction \
             for reconciliation and auditing."
                .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "presig_index": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "The presig_index returned from the signing operation"
                },
                "tx_hash": {
                    "type": "string",
                    "pattern": "^0x[a-fA-F0-9]{64}$",
                    "description": "The transaction hash from the blockchain after broadcast (hex with 0x prefix)"
                }
            },
            "required": ["presig_index", "tx_hash"]
        }),
        output_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "success": {
                    "type": "boolean",
                    "description": "Whether the update was successful"
                },
                "presig_index": {
                    "type": "integer",
                    "description": "The presig index that was updated"
                },
                "tx_hash": {
                    "type": "string",
                    "description": "The transaction hash that was recorded"
                }
            },
            "required": ["success", "presig_index", "tx_hash"]
        })),
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(false),
            destructive_hint: Some(false), // Just updates metadata
            idempotent_hint: Some(true),   // Can safely retry
            open_world_hint: Some(false),
        }),
    }
}

/// Execute the update tx hash tool
pub async fn execute(ctx: &ToolContext, arguments: serde_json::Value) -> ToolsCallResult {
    use crate::client::ClientError;

    // Parse arguments
    let params: UpdateTxHashParams = match serde_json::from_value(arguments) {
        Ok(p) => p,
        Err(e) => {
            return ToolsCallResult::error(format!("Invalid parameters: {}", e));
        }
    };

    // Validate tx hash format
    if !params.tx_hash.starts_with("0x") || params.tx_hash.len() != 66 {
        return ToolsCallResult::error(
            "Invalid tx_hash: must be 32 bytes hex with 0x prefix (66 characters total)",
        );
    }

    // Check disk status first
    let state = match ctx.daemon_client.get_disk_status().await {
        Ok(s) => s,
        Err(ClientError::DaemonNotRunning) => {
            return ToolsCallResult::error(
                "Sigil daemon is not running. Start it with: sigil-daemon start",
            );
        }
        Err(e) => {
            return ToolsCallResult::error(format!("Failed to check disk: {}", e));
        }
    };

    if !state.detected {
        return ToolsCallResult::error(
            "No signing disk detected. Please insert your Sigil disk to update the audit log.",
        );
    }

    // Update the tx hash in the daemon
    if let Err(e) = ctx
        .daemon_client
        .update_tx_hash(params.presig_index, &params.tx_hash)
        .await
    {
        return ToolsCallResult::error(format!("Failed to update tx hash: {}", e));
    }

    let result = serde_json::json!({
        "success": true,
        "presig_index": params.presig_index,
        "tx_hash": params.tx_hash,
        "child_id": state.child_id
    });

    let text = format!(
        "✓ Audit log updated\n\
         ├─ Disk: sigil_{}\n\
         ├─ Presig index: {}\n\
         └─ TX hash: {}...{}",
        state.child_id.as_deref().unwrap_or("unknown"),
        params.presig_index,
        &params.tx_hash[..10],
        &params.tx_hash[58..]
    );

    ToolsCallResult::success_with_structured(vec![ToolContent::text(text)], result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::DaemonClient;
    use crate::tools::DiskState;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_update_tx_hash_success() {
        let ctx = ToolContext {
            daemon_client: Arc::new(DaemonClient::new_mock(DiskState::mock_detected())),
        };

        let args = serde_json::json!({
            "presig_index": 153,
            "tx_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        });

        let result = execute(&ctx, args).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));

        if let Some(structured) = &result.structured_content {
            assert_eq!(structured["success"], true);
            assert_eq!(structured["presig_index"], 153);
        }
    }

    #[tokio::test]
    async fn test_update_tx_hash_invalid() {
        let ctx = ToolContext {
            daemon_client: Arc::new(DaemonClient::new_mock(DiskState::mock_detected())),
        };

        let args = serde_json::json!({
            "presig_index": 153,
            "tx_hash": "invalid"
        });

        let result = execute(&ctx, args).await;
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn test_update_tx_hash_no_disk() {
        let ctx = ToolContext {
            daemon_client: Arc::new(DaemonClient::new_mock(DiskState::default())),
        };

        let args = serde_json::json!({
            "presig_index": 153,
            "tx_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        });

        let result = execute(&ctx, args).await;
        assert_eq!(result.is_error, Some(true));
    }
}
