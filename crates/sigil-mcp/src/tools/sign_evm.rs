//! Sign EVM transaction tool

use crate::protocol::{Tool, ToolAnnotations, ToolContent, ToolsCallResult};
use serde::Deserialize;

use super::ToolContext;

/// Sign EVM input parameters
#[derive(Debug, Deserialize)]
pub struct SignEvmParams {
    /// Transaction hash to sign (hex with 0x prefix)
    pub message_hash: String,

    /// EIP-155 chain ID
    pub chain_id: u32,

    /// Human-readable description for audit log
    pub description: String,
}

/// Get the tool definition
pub fn tool_definition() -> Tool {
    Tool {
        name: "sigil_sign_evm".to_string(),
        title: Some("Sign EVM Transaction".to_string()),
        description:
            "Sign a transaction hash for EVM-compatible chains (Ethereum, Polygon, Arbitrum, etc.) \
             using ECDSA. Requires a valid Sigil disk with remaining presignatures. \
             Each call consumes one presignature."
                .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "message_hash": {
                    "type": "string",
                    "pattern": "^0x[a-fA-F0-9]{64}$",
                    "description": "32-byte transaction hash to sign (hex with 0x prefix)"
                },
                "chain_id": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "EIP-155 chain ID (1=Ethereum, 137=Polygon, 42161=Arbitrum, 10=Optimism, 8453=Base)"
                },
                "description": {
                    "type": "string",
                    "maxLength": 256,
                    "description": "Human-readable description for the audit log (e.g., 'Transfer 0.1 ETH to vitalik.eth')"
                }
            },
            "required": ["message_hash", "chain_id", "description"]
        }),
        output_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "signature": {
                    "type": "string",
                    "description": "Full ECDSA signature (hex)"
                },
                "v": {
                    "type": "integer",
                    "description": "Recovery parameter (27 or 28, or EIP-155 adjusted)"
                },
                "r": {
                    "type": "string",
                    "description": "R component of signature (hex)"
                },
                "s": {
                    "type": "string",
                    "description": "S component of signature (hex)"
                },
                "presig_index": {
                    "type": "integer",
                    "description": "Index of the presignature used"
                },
                "proof_hash": {
                    "type": "string",
                    "description": "ZK proof hash for audit verification (hex)"
                }
            },
            "required": ["signature", "v", "r", "s", "presig_index"]
        })),
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(false),
            destructive_hint: Some(true), // Consumes a presignature
            idempotent_hint: Some(false), // Each call uses a new presig
            open_world_hint: Some(false),
        }),
    }
}

/// Execute the sign EVM tool
pub async fn execute(ctx: &ToolContext, arguments: serde_json::Value) -> ToolsCallResult {
    use crate::client::ClientError;

    // Parse arguments
    let params: SignEvmParams = match serde_json::from_value(arguments) {
        Ok(p) => p,
        Err(e) => {
            return ToolsCallResult::error(format!("Invalid parameters: {}", e));
        }
    };

    // Validate message hash format
    if !params.message_hash.starts_with("0x") || params.message_hash.len() != 66 {
        return ToolsCallResult::error(
            "Invalid message_hash: must be 32 bytes hex with 0x prefix (66 characters total)",
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
            "No signing disk detected. Please insert your Sigil disk to sign transactions.",
        );
    }

    if state.is_valid != Some(true) {
        return ToolsCallResult::error(
            "Signing disk is invalid or expired. Please use a valid Sigil disk.",
        );
    }

    let remaining = state.presigs_remaining.unwrap_or(0);
    if remaining == 0 {
        return ToolsCallResult::error(
            "No presignatures remaining on disk. Please generate a new disk from your mother device.",
        );
    }

    // Check scheme compatibility (if available)
    if let Some(scheme) = &state.scheme {
        if scheme != "ecdsa" {
            return ToolsCallResult::error(format!(
                "Disk scheme mismatch: EVM signing requires 'ecdsa', but disk has '{}'",
                scheme
            ));
        }
    }

    // Call daemon to sign
    let sign_result = match ctx
        .daemon_client
        .sign(&params.message_hash, params.chain_id, &params.description)
        .await
    {
        Ok(r) => r,
        Err(ClientError::NoDiskDetected) => {
            return ToolsCallResult::error("No disk detected");
        }
        Err(ClientError::SigningFailed(msg)) => {
            return ToolsCallResult::error(format!("Signing failed: {}", msg));
        }
        Err(e) => {
            return ToolsCallResult::error(format!("Signing error: {}", e));
        }
    };

    // Parse signature into v, r, s components
    // Signature format from daemon: hex string (may or may not have 0x prefix)
    let sig_hex = sign_result.signature.trim_start_matches("0x");

    // ECDSA signature is 65 bytes: r (32) + s (32) + v (1)
    if sig_hex.len() != 130 {
        return ToolsCallResult::error(format!(
            "Invalid signature length: expected 130 hex chars, got {}",
            sig_hex.len()
        ));
    }

    let r = format!("0x{}", &sig_hex[0..64]);
    let s = format!("0x{}", &sig_hex[64..128]);
    let v = u8::from_str_radix(&sig_hex[128..130], 16).unwrap_or(27) as u32;

    let result = serde_json::json!({
        "signature": format!("0x{}", sig_hex),
        "v": v,
        "r": r,
        "s": s,
        "presig_index": sign_result.presig_index,
        "proof_hash": sign_result.proof_hash,
        "chain_id": params.chain_id,
        "message_hash": params.message_hash
    });

    let chain_name = get_chain_name(params.chain_id);

    let text = format!(
        "✓ Transaction signed successfully\n\
         ├─ Chain: {} (ID: {})\n\
         ├─ Hash: {}...{}\n\
         ├─ Presig #{} used ({} remaining)\n\
         └─ Description: {}",
        chain_name,
        params.chain_id,
        &params.message_hash[..10],
        &params.message_hash[58..],
        sign_result.presig_index,
        remaining - 1,
        params.description
    );

    ToolsCallResult::success_with_structured(vec![ToolContent::text(text)], result)
}

/// Get human-readable chain name from chain ID
fn get_chain_name(chain_id: u32) -> &'static str {
    match chain_id {
        1 => "Ethereum Mainnet",
        5 => "Goerli Testnet",
        11155111 => "Sepolia Testnet",
        137 => "Polygon",
        80001 => "Polygon Mumbai",
        42161 => "Arbitrum One",
        421613 => "Arbitrum Goerli",
        10 => "Optimism",
        420 => "Optimism Goerli",
        8453 => "Base",
        84531 => "Base Goerli",
        56 => "BNB Smart Chain",
        97 => "BNB Testnet",
        43114 => "Avalanche C-Chain",
        43113 => "Avalanche Fuji",
        250 => "Fantom",
        100 => "Gnosis Chain",
        _ => "Unknown Chain",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::DaemonClient;
    use crate::tools::DiskState;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_sign_evm_success() {
        let ctx = ToolContext {
            daemon_client: Arc::new(DaemonClient::new_mock(DiskState::mock_detected())),
        };

        let args = serde_json::json!({
            "message_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "chain_id": 1,
            "description": "Test transfer"
        });

        let result = execute(&ctx, args).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));
    }

    #[tokio::test]
    async fn test_sign_evm_no_disk() {
        let ctx = ToolContext {
            daemon_client: Arc::new(DaemonClient::new_mock(DiskState::default())),
        };

        let args = serde_json::json!({
            "message_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "chain_id": 1,
            "description": "Test transfer"
        });

        let result = execute(&ctx, args).await;
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn test_sign_evm_invalid_hash() {
        let ctx = ToolContext {
            daemon_client: Arc::new(DaemonClient::new_mock(DiskState::mock_detected())),
        };

        let args = serde_json::json!({
            "message_hash": "invalid",
            "chain_id": 1,
            "description": "Test transfer"
        });

        let result = execute(&ctx, args).await;
        assert_eq!(result.is_error, Some(true));
    }
}
