//! Get signing address tool

use crate::protocol::{Tool, ToolAnnotations, ToolContent, ToolsCallResult};
use serde::Deserialize;

use super::ToolContext;

/// Address format options
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressFormat {
    #[default]
    Hex,
    Evm,
    Bitcoin,
    Solana,
    Cosmos,
}

/// Get address input parameters
#[derive(Debug, Default, Deserialize)]
pub struct GetAddressParams {
    /// Signature scheme (defaults to disk's native scheme)
    #[serde(default)]
    pub scheme: Option<String>,

    /// Address format to return
    #[serde(default)]
    pub format: Option<AddressFormat>,

    /// Bech32 prefix for Cosmos chains
    #[serde(default)]
    pub cosmos_prefix: Option<String>,
}

/// Get the tool definition
pub fn tool_definition() -> Tool {
    Tool {
        name: "sigil_get_address".to_string(),
        title: Some("Get Signing Address".to_string()),
        description: "Get the blockchain address associated with the current Sigil disk. \
             The address format depends on the signature scheme and target chain."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "scheme": {
                    "type": "string",
                    "enum": ["ecdsa", "taproot", "ed25519", "ristretto255"],
                    "description": "Signature scheme (defaults to disk's native scheme)"
                },
                "format": {
                    "type": "string",
                    "enum": ["hex", "evm", "bitcoin", "solana", "cosmos"],
                    "default": "hex",
                    "description": "Address format to return"
                },
                "cosmos_prefix": {
                    "type": "string",
                    "description": "Bech32 prefix for Cosmos chains (e.g., 'cosmos', 'osmo', 'juno')"
                }
            }
        }),
        output_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "string",
                    "description": "The formatted address"
                },
                "format": {
                    "type": "string",
                    "description": "Address format used"
                },
                "scheme": {
                    "type": "string",
                    "description": "Signature scheme"
                },
                "public_key": {
                    "type": "string",
                    "description": "Public key (hex)"
                }
            },
            "required": ["address", "format", "scheme"]
        })),
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

/// Execute the get address tool
pub async fn execute(ctx: &ToolContext, arguments: serde_json::Value) -> ToolsCallResult {
    // Parse arguments (allow empty object)
    let params: GetAddressParams = serde_json::from_value(arguments).unwrap_or_default();

    // Check disk status
    let state = ctx.disk_state.read().await;

    if !state.detected {
        return ToolsCallResult::error(
            "No signing disk detected. Please insert your Sigil disk to get the address.",
        );
    }

    let scheme = params
        .scheme
        .as_deref()
        .or(state.scheme.as_deref())
        .unwrap_or("ecdsa");

    let format = params.format.unwrap_or(AddressFormat::Hex);

    // In a real implementation, this would derive the address from the public key
    // For now, we return mock addresses based on format
    //
    // TODO: Integrate with sigil-core address derivation

    let public_key = state
        .public_key
        .clone()
        .unwrap_or_else(|| "0x04abcdef1234567890".to_string());

    let (address, format_name) = match format {
        AddressFormat::Hex => (public_key.clone(), "hex"),
        AddressFormat::Evm => {
            // Mock EVM address (would be keccak256 of pubkey)
            (
                "0x742d35Cc6634C0532925a3b844Bc9e7595f12345".to_string(),
                "evm",
            )
        }
        AddressFormat::Bitcoin => {
            match scheme {
                "taproot" => {
                    // Mock Taproot address
                    (
                        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
                            .to_string(),
                        "bitcoin_taproot",
                    )
                }
                _ => {
                    // Mock legacy address
                    (
                        "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".to_string(),
                        "bitcoin_legacy",
                    )
                }
            }
        }
        AddressFormat::Solana => {
            // Mock Solana address (base58)
            (
                "7EcDhSYGxXVqPpPwuN3hYFvBNuEbC6M3sGdMrfgVfcNr".to_string(),
                "solana",
            )
        }
        AddressFormat::Cosmos => {
            let prefix = params.cosmos_prefix.as_deref().unwrap_or("cosmos");
            // Mock Cosmos address (bech32)
            (
                format!("{}1qypqxpq9qcrsszgszyfpx9q4zct3sxfq0fzduj", prefix),
                "cosmos",
            )
        }
    };

    let result = serde_json::json!({
        "address": address,
        "format": format_name,
        "scheme": scheme,
        "public_key": public_key,
        "child_id": state.child_id
    });

    let text = format!(
        "Address for disk sigil_{}\n\
         ├─ Scheme: {}\n\
         ├─ Format: {}\n\
         └─ Address: {}",
        state.child_id.as_deref().unwrap_or("unknown"),
        scheme,
        format_name,
        address
    );

    ToolsCallResult::success_with_structured(vec![ToolContent::text(text)], result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::DiskState;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_get_address_default() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(DiskState::mock_detected())),
        };

        let result = execute(&ctx, serde_json::json!({})).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));
    }

    #[tokio::test]
    async fn test_get_address_evm() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(DiskState::mock_detected())),
        };

        let args = serde_json::json!({
            "format": "evm"
        });

        let result = execute(&ctx, args).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));

        if let Some(structured) = &result.structured_content {
            let address = structured["address"].as_str().unwrap();
            assert!(address.starts_with("0x"));
        }
    }

    #[tokio::test]
    async fn test_get_address_cosmos() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(DiskState::mock_detected())),
        };

        let args = serde_json::json!({
            "format": "cosmos",
            "cosmos_prefix": "osmo"
        });

        let result = execute(&ctx, args).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));

        if let Some(structured) = &result.structured_content {
            let address = structured["address"].as_str().unwrap();
            assert!(address.starts_with("osmo1"));
        }
    }

    #[tokio::test]
    async fn test_get_address_no_disk() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(DiskState::no_disk())),
        };

        let result = execute(&ctx, serde_json::json!({})).await;
        assert_eq!(result.is_error, Some(true));
    }
}
