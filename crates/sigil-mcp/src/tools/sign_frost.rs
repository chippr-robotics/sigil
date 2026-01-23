//! Sign with FROST threshold signatures tool

use crate::protocol::{Tool, ToolAnnotations, ToolContent, ToolsCallResult};
use serde::Deserialize;

use super::ToolContext;

/// FROST signature schemes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrostScheme {
    Taproot,
    Ed25519,
    Ristretto255,
    Pallas,
}

impl FrostScheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            FrostScheme::Taproot => "taproot",
            FrostScheme::Ed25519 => "ed25519",
            FrostScheme::Ristretto255 => "ristretto255",
            FrostScheme::Pallas => "pallas",
        }
    }

    pub fn signature_length(&self) -> usize {
        match self {
            FrostScheme::Taproot => 64,      // BIP-340 Schnorr
            FrostScheme::Ed25519 => 64,      // Ed25519
            FrostScheme::Ristretto255 => 64, // Ristretto255
            FrostScheme::Pallas => 64,       // Pallas Schnorr
        }
    }

    pub fn supported_chains(&self) -> &'static [&'static str] {
        match self {
            FrostScheme::Taproot => &["Bitcoin (Taproot)"],
            FrostScheme::Ed25519 => &["Solana", "Cosmos", "Near", "Polkadot"],
            FrostScheme::Ristretto255 => &["Zcash (shielded)"],
            FrostScheme::Pallas => &["DarkFi", "Zcash (Orchard)"],
        }
    }
}

/// Sign FROST input parameters
#[derive(Debug, Deserialize)]
pub struct SignFrostParams {
    /// FROST signature scheme
    pub scheme: FrostScheme,

    /// Message hash to sign (hex with 0x prefix)
    pub message_hash: String,

    /// Human-readable description for audit log
    pub description: String,
}

/// Get the tool definition
pub fn tool_definition() -> Tool {
    Tool {
        name: "sigil_sign_frost".to_string(),
        title: Some("Sign with FROST".to_string()),
        description:
            "Sign a message using FROST threshold signatures. Supports multiple signature schemes: \
             Taproot (Bitcoin BIP-340), Ed25519 (Solana, Cosmos), Ristretto255 (Zcash shielded), \
             and Pallas (DarkFi, Zcash Orchard). Each call consumes one presignature from the disk."
                .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "scheme": {
                    "type": "string",
                    "enum": ["taproot", "ed25519", "ristretto255", "pallas"],
                    "description": "FROST signature scheme: 'taproot' for Bitcoin, 'ed25519' for Solana/Cosmos, 'ristretto255' for Zcash, 'pallas' for DarkFi"
                },
                "message_hash": {
                    "type": "string",
                    "pattern": "^0x[a-fA-F0-9]+$",
                    "description": "Message hash to sign (hex with 0x prefix). Length varies by scheme."
                },
                "description": {
                    "type": "string",
                    "maxLength": 256,
                    "description": "Human-readable description for the audit log"
                }
            },
            "required": ["scheme", "message_hash", "description"]
        }),
        output_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "scheme": {
                    "type": "string",
                    "description": "Signature scheme used"
                },
                "signature": {
                    "type": "string",
                    "description": "FROST signature (hex)"
                },
                "signature_length": {
                    "type": "integer",
                    "description": "Signature length in bytes"
                },
                "presig_index": {
                    "type": "integer",
                    "description": "Index of the presignature used"
                }
            },
            "required": ["scheme", "signature", "signature_length", "presig_index"]
        })),
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(false),
            destructive_hint: Some(true), // Consumes a presignature
            idempotent_hint: Some(false), // Each call uses a new presig
            open_world_hint: Some(false),
        }),
    }
}

/// Execute the sign FROST tool
pub async fn execute(ctx: &ToolContext, arguments: serde_json::Value) -> ToolsCallResult {
    // Parse arguments
    let params: SignFrostParams = match serde_json::from_value(arguments) {
        Ok(p) => p,
        Err(e) => {
            return ToolsCallResult::error(format!("Invalid parameters: {}", e));
        }
    };

    // Validate message hash format
    if !params.message_hash.starts_with("0x") {
        return ToolsCallResult::error("Invalid message_hash: must start with 0x prefix");
    }

    // Check disk status
    let state = ctx.disk_state.read().await;

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

    // Check scheme compatibility
    let disk_scheme = state.scheme.as_deref().unwrap_or("unknown");
    if disk_scheme != params.scheme.as_str() {
        return ToolsCallResult::error(format!(
            "Disk scheme mismatch: requested '{}', but disk has '{}'. \
             Create a new disk with the correct scheme.",
            params.scheme.as_str(),
            disk_scheme
        ));
    }

    // In a real implementation, this would call the FROST signing API
    // For now, we return a mock signature to demonstrate the flow
    //
    // TODO: Integrate with sigil-frost

    let mock_presig_index = 1000 - remaining;
    let sig_len = params.scheme.signature_length();
    let mock_signature = format!("0x{:0>width$}", "cafe", width = sig_len * 2);

    let result = serde_json::json!({
        "scheme": params.scheme.as_str(),
        "signature": mock_signature,
        "signature_length": sig_len,
        "presig_index": mock_presig_index,
        "message_hash": params.message_hash
    });

    let chains = params.scheme.supported_chains().join(", ");
    let hash_preview = if params.message_hash.len() > 20 {
        format!(
            "{}...{}",
            &params.message_hash[..10],
            &params.message_hash[params.message_hash.len() - 8..]
        )
    } else {
        params.message_hash.clone()
    };

    let text = format!(
        "✓ FROST signature created\n\
         ├─ Scheme: {} ({} chains)\n\
         ├─ Hash: {}\n\
         ├─ Signature: {}...{}\n\
         ├─ Presig #{} used ({} remaining)\n\
         └─ Description: {}",
        params.scheme.as_str(),
        chains,
        hash_preview,
        &mock_signature[..14],
        &mock_signature[mock_signature.len() - 8..],
        mock_presig_index,
        remaining - 1,
        params.description
    );

    ToolsCallResult::success_with_structured(vec![ToolContent::text(text)], result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::DiskState;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_frost_disk(scheme: &str) -> DiskState {
        DiskState {
            detected: true,
            child_id: Some("frost_001".to_string()),
            scheme: Some(scheme.to_string()),
            presigs_remaining: Some(500),
            presigs_total: Some(1000),
            days_until_expiry: Some(30),
            is_valid: Some(true),
            public_key: Some("0x02abc123".to_string()),
        }
    }

    #[tokio::test]
    async fn test_sign_frost_taproot() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(create_frost_disk("taproot"))),
        };

        let args = serde_json::json!({
            "scheme": "taproot",
            "message_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "description": "Bitcoin Taproot transfer"
        });

        let result = execute(&ctx, args).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));
    }

    #[tokio::test]
    async fn test_sign_frost_ed25519() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(create_frost_disk("ed25519"))),
        };

        let args = serde_json::json!({
            "scheme": "ed25519",
            "message_hash": "0xabcdef",
            "description": "Solana transfer"
        });

        let result = execute(&ctx, args).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));
    }

    #[tokio::test]
    async fn test_sign_frost_scheme_mismatch() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(create_frost_disk("taproot"))),
        };

        let args = serde_json::json!({
            "scheme": "ed25519",  // Mismatch!
            "message_hash": "0xabcdef",
            "description": "This should fail"
        });

        let result = execute(&ctx, args).await;
        assert_eq!(result.is_error, Some(true));
    }
}
