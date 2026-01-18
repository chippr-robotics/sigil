//! Sigil MCP Resource definitions
//!
//! Resources provide readable context data to agents.

use crate::protocol::{Resource, ResourceContent, ResourceTemplate, ResourcesReadResult};
use crate::tools::{DiskState, ToolContext};

/// Get all resource definitions
pub fn get_all_resources(disk_state: &DiskState) -> Vec<Resource> {
    let mut resources = vec![Resource {
        uri: "sigil://supported-chains".to_string(),
        name: "Supported Chains".to_string(),
        title: Some("Supported Blockchain Networks".to_string()),
        description: Some(
            "List of blockchain networks supported for signing with their chain IDs".to_string(),
        ),
        mime_type: Some("application/json".to_string()),
        size: None,
        annotations: None,
    }];

    // Add disk status resource if disk is detected
    if disk_state.detected {
        resources.push(Resource {
            uri: "sigil://disk/status".to_string(),
            name: "Disk Status".to_string(),
            title: Some("Current Sigil Disk Status".to_string()),
            description: Some(
                "Real-time status of the inserted signing disk including validity, \
                 remaining presignatures, and expiry information"
                    .to_string(),
            ),
            mime_type: Some("application/json".to_string()),
            size: None,
            annotations: None,
        });

        resources.push(Resource {
            uri: "sigil://presigs/info".to_string(),
            name: "Presignature Info".to_string(),
            title: Some("Presignature Statistics".to_string()),
            description: Some(
                "Detailed information about presignature consumption and availability".to_string(),
            ),
            mime_type: Some("application/json".to_string()),
            size: None,
            annotations: None,
        });
    }

    resources
}

/// Get all resource templates
pub fn get_all_templates() -> Vec<ResourceTemplate> {
    vec![ResourceTemplate {
        uri_template: "sigil://children/{child_id}".to_string(),
        name: "Child Disk Info".to_string(),
        title: Some("Child Disk Information".to_string()),
        description: Some("Information about a specific child disk by ID".to_string()),
        mime_type: Some("application/json".to_string()),
    }]
}

/// Read a resource by URI
pub async fn read_resource(ctx: &ToolContext, uri: &str) -> Result<ResourcesReadResult, String> {
    match uri {
        "sigil://disk/status" => read_disk_status(ctx).await,
        "sigil://presigs/info" => read_presigs_info(ctx).await,
        "sigil://supported-chains" => read_supported_chains().await,
        _ if uri.starts_with("sigil://children/") => {
            let child_id = uri.strip_prefix("sigil://children/").unwrap();
            read_child_info(ctx, child_id).await
        }
        _ => Err(format!("Unknown resource: {}", uri)),
    }
}

/// Read disk status resource
async fn read_disk_status(ctx: &ToolContext) -> Result<ResourcesReadResult, String> {
    let state = ctx.disk_state.read().await;

    if !state.detected {
        return Err("No disk detected".to_string());
    }

    let content = serde_json::json!({
        "detected": true,
        "child_id": state.child_id,
        "scheme": state.scheme,
        "presigs_remaining": state.presigs_remaining,
        "presigs_total": state.presigs_total,
        "days_until_expiry": state.days_until_expiry,
        "is_valid": state.is_valid,
        "public_key": state.public_key,
        "addresses": {
            "evm": "0x742d35Cc6634C0532925a3b844Bc9e7595f12345",
            "bitcoin_legacy": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"
        }
    });

    Ok(ResourcesReadResult {
        contents: vec![ResourceContent {
            uri: "sigil://disk/status".to_string(),
            mime_type: Some("application/json".to_string()),
            text: Some(serde_json::to_string_pretty(&content).unwrap()),
            blob: None,
            annotations: None,
        }],
    })
}

/// Read presigs info resource
async fn read_presigs_info(ctx: &ToolContext) -> Result<ResourcesReadResult, String> {
    let state = ctx.disk_state.read().await;

    if !state.detected {
        return Err("No disk detected".to_string());
    }

    let remaining = state.presigs_remaining.unwrap_or(0);
    let total = state.presigs_total.unwrap_or(0);
    let used = total - remaining;
    let percentage_remaining = if total > 0 {
        (remaining as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let content = serde_json::json!({
        "remaining": remaining,
        "total": total,
        "used": used,
        "percentage_remaining": percentage_remaining,
        "low_warning": remaining < 100,
        "critical_warning": remaining < 10,
        "estimated_days_at_current_rate": null  // Would need usage history
    });

    Ok(ResourcesReadResult {
        contents: vec![ResourceContent {
            uri: "sigil://presigs/info".to_string(),
            mime_type: Some("application/json".to_string()),
            text: Some(serde_json::to_string_pretty(&content).unwrap()),
            blob: None,
            annotations: None,
        }],
    })
}

/// Read supported chains resource
async fn read_supported_chains() -> Result<ResourcesReadResult, String> {
    let content = serde_json::json!({
        "evm_chains": [
            { "name": "Ethereum Mainnet", "chain_id": 1, "symbol": "ETH", "scheme": "ecdsa" },
            { "name": "Goerli Testnet", "chain_id": 5, "symbol": "ETH", "scheme": "ecdsa" },
            { "name": "Sepolia Testnet", "chain_id": 11155111, "symbol": "ETH", "scheme": "ecdsa" },
            { "name": "Polygon", "chain_id": 137, "symbol": "MATIC", "scheme": "ecdsa" },
            { "name": "Arbitrum One", "chain_id": 42161, "symbol": "ETH", "scheme": "ecdsa" },
            { "name": "Optimism", "chain_id": 10, "symbol": "ETH", "scheme": "ecdsa" },
            { "name": "Base", "chain_id": 8453, "symbol": "ETH", "scheme": "ecdsa" },
            { "name": "BNB Smart Chain", "chain_id": 56, "symbol": "BNB", "scheme": "ecdsa" },
            { "name": "Avalanche C-Chain", "chain_id": 43114, "symbol": "AVAX", "scheme": "ecdsa" },
            { "name": "Fantom", "chain_id": 250, "symbol": "FTM", "scheme": "ecdsa" },
            { "name": "Gnosis Chain", "chain_id": 100, "symbol": "xDAI", "scheme": "ecdsa" }
        ],
        "frost_chains": {
            "taproot": [
                { "name": "Bitcoin Mainnet", "symbol": "BTC", "address_prefix": "bc1p" },
                { "name": "Bitcoin Testnet", "symbol": "tBTC", "address_prefix": "tb1p" }
            ],
            "ed25519": [
                { "name": "Solana", "symbol": "SOL" },
                { "name": "Cosmos Hub", "symbol": "ATOM", "bech32_prefix": "cosmos" },
                { "name": "Osmosis", "symbol": "OSMO", "bech32_prefix": "osmo" },
                { "name": "Near", "symbol": "NEAR" },
                { "name": "Polkadot", "symbol": "DOT" }
            ],
            "ristretto255": [
                { "name": "Zcash (shielded)", "symbol": "ZEC", "address_prefix": "zs" }
            ]
        }
    });

    Ok(ResourcesReadResult {
        contents: vec![ResourceContent {
            uri: "sigil://supported-chains".to_string(),
            mime_type: Some("application/json".to_string()),
            text: Some(serde_json::to_string_pretty(&content).unwrap()),
            blob: None,
            annotations: None,
        }],
    })
}

/// Read child disk info resource
async fn read_child_info(ctx: &ToolContext, child_id: &str) -> Result<ResourcesReadResult, String> {
    let state = ctx.disk_state.read().await;

    // Check if this is the current disk
    if state.child_id.as_deref() == Some(child_id) {
        let content = serde_json::json!({
            "child_id": child_id,
            "is_current": true,
            "scheme": state.scheme,
            "presigs_remaining": state.presigs_remaining,
            "presigs_total": state.presigs_total,
            "days_until_expiry": state.days_until_expiry,
            "is_valid": state.is_valid
        });

        Ok(ResourcesReadResult {
            contents: vec![ResourceContent {
                uri: format!("sigil://children/{}", child_id),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&content).unwrap()),
                blob: None,
                annotations: None,
            }],
        })
    } else {
        Err(format!(
            "Child disk '{}' not found or not currently inserted",
            child_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_get_all_resources_no_disk() {
        let state = DiskState::no_disk();
        let resources = get_all_resources(&state);
        // Should still have supported-chains
        assert!(resources
            .iter()
            .any(|r| r.uri == "sigil://supported-chains"));
        // Should not have disk-specific resources
        assert!(!resources.iter().any(|r| r.uri == "sigil://disk/status"));
    }

    #[tokio::test]
    async fn test_get_all_resources_with_disk() {
        let state = DiskState::mock_detected();
        let resources = get_all_resources(&state);
        assert!(resources.iter().any(|r| r.uri == "sigil://disk/status"));
        assert!(resources.iter().any(|r| r.uri == "sigil://presigs/info"));
    }

    #[tokio::test]
    async fn test_read_supported_chains() {
        let result = read_supported_chains().await.unwrap();
        assert_eq!(result.contents.len(), 1);
        assert!(result.contents[0].text.is_some());
    }

    #[tokio::test]
    async fn test_read_disk_status() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(DiskState::mock_detected())),
        };

        let result = read_disk_status(&ctx).await.unwrap();
        assert_eq!(result.contents.len(), 1);

        let text = result.contents[0].text.as_ref().unwrap();
        assert!(text.contains("detected"));
    }
}
