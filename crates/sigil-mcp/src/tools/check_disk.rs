//! Check disk status tool

use crate::protocol::{Tool, ToolAnnotations, ToolContent, ToolsCallResult};

use super::ToolContext;

/// Get the tool definition
pub fn tool_definition() -> Tool {
    Tool {
        name: "sigil_check_disk".to_string(),
        title: Some("Check Sigil Disk Status".to_string()),
        description:
            "Check if a Sigil signing disk is inserted, valid, and has remaining presignatures. \
             Call this before any signing operation to verify the disk is ready."
                .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
        output_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "detected": {
                    "type": "boolean",
                    "description": "Whether a Sigil disk is inserted"
                },
                "child_id": {
                    "type": "string",
                    "description": "Short ID of the child disk"
                },
                "scheme": {
                    "type": "string",
                    "enum": ["ecdsa", "taproot", "ed25519", "ristretto255"],
                    "description": "Signature scheme supported by this disk"
                },
                "presigs_remaining": {
                    "type": "integer",
                    "description": "Number of presignatures remaining"
                },
                "presigs_total": {
                    "type": "integer",
                    "description": "Total presignatures on disk"
                },
                "days_until_expiry": {
                    "type": "integer",
                    "description": "Days until the disk expires"
                },
                "is_valid": {
                    "type": "boolean",
                    "description": "Whether the disk passes validation"
                }
            },
            "required": ["detected"]
        })),
        annotations: Some(ToolAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
    }
}

/// Execute the check disk tool
pub async fn execute(ctx: &ToolContext) -> ToolsCallResult {
    let state = ctx.disk_state.read().await;

    if !state.detected {
        let result = serde_json::json!({
            "detected": false
        });

        return ToolsCallResult::success_with_structured(
            vec![ToolContent::text(
                "No Sigil disk detected. Please insert your signing disk.",
            )],
            result,
        );
    }

    let result = serde_json::json!({
        "detected": true,
        "child_id": state.child_id,
        "scheme": state.scheme,
        "presigs_remaining": state.presigs_remaining,
        "presigs_total": state.presigs_total,
        "days_until_expiry": state.days_until_expiry,
        "is_valid": state.is_valid
    });

    // Build human-readable status
    let mut status_lines = vec![format!(
        "Disk detected (sigil_{})",
        state.child_id.as_deref().unwrap_or("unknown")
    )];

    if let (Some(remaining), Some(total)) = (state.presigs_remaining, state.presigs_total) {
        status_lines.push(format!("├─ Presigs: {}/{} remaining", remaining, total));

        // Add warning if low
        if remaining < 100 {
            status_lines.push(format!(
                "│  ⚠️  Warning: Only {} presignatures remaining!",
                remaining
            ));
        }
    }

    if let Some(scheme) = &state.scheme {
        status_lines.push(format!("├─ Scheme: {}", scheme));
    }

    if let Some(days) = state.days_until_expiry {
        if days < 7 {
            status_lines.push(format!("├─ Expires: {} days ⚠️", days));
        } else {
            status_lines.push(format!("├─ Expires: {} days", days));
        }
    }

    if let Some(valid) = state.is_valid {
        if valid {
            status_lines.push("└─ Status: ✓ Valid".to_string());
        } else {
            status_lines.push("└─ Status: ✗ Invalid".to_string());
        }
    }

    let status_text = status_lines.join("\n");

    ToolsCallResult::success_with_structured(vec![ToolContent::text(status_text)], result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::DiskState;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_check_disk_detected() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(DiskState::mock_detected())),
        };

        let result = execute(&ctx).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));
        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn test_check_disk_not_detected() {
        let ctx = ToolContext {
            disk_state: Arc::new(RwLock::new(DiskState::no_disk())),
        };

        let result = execute(&ctx).await;
        assert!(result.is_error.is_none() || result.is_error == Some(false));

        // Should indicate no disk
        if let Some(structured) = &result.structured_content {
            assert_eq!(structured["detected"], false);
        }
    }
}
