//! Claude CLI tool definitions
//!
//! These tools can be exposed to Claude for blockchain transaction signing.

use serde::{Deserialize, Serialize};

use crate::client::{ClientError, SigilClient};

/// Tool: sign_blockchain_transaction
///
/// Signs a blockchain transaction using MPC presignatures.
/// Requires a Sigil disk to be inserted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTransactionTool {
    /// Transaction hash to sign (keccak256 of RLP-encoded transaction)
    pub transaction_hash: String,

    /// Chain ID (e.g., 1 for Ethereum mainnet)
    pub chain_id: u32,

    /// Human-readable description of the transaction
    pub description: String,
}

/// Result from the sign transaction tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTransactionResult {
    /// Success or failure
    pub success: bool,

    /// The ECDSA signature (hex encoded, 64 bytes)
    pub signature: Option<String>,

    /// v value for Ethereum (recovery id + chain_id * 2 + 35)
    pub v: Option<u32>,

    /// r component of signature (hex)
    pub r: Option<String>,

    /// s component of signature (hex)
    pub s: Option<String>,

    /// Presig index that was used
    pub presig_index: Option<u32>,

    /// zkVM proof hash
    pub proof_hash: Option<String>,

    /// Error message if signing failed
    pub error: Option<String>,
}

/// Tool: check_signing_disk
///
/// Checks the status of the currently inserted Sigil signing disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDiskTool {}

/// Result from the check disk tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckDiskResult {
    /// Whether a disk is detected
    pub detected: bool,

    /// Short identifier for the child disk
    pub disk_id: Option<String>,

    /// Number of presigs remaining
    pub presigs_remaining: Option<u32>,

    /// Total presigs on disk
    pub presigs_total: Option<u32>,

    /// Days until disk expiry
    pub days_until_expiry: Option<u32>,

    /// Whether disk is valid for signing
    pub is_valid: Option<bool>,

    /// Human-readable status message
    pub message: String,
}

/// Tool: estimate_transaction
///
/// Estimates gas and shows transaction summary without signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionTool {
    /// Target address
    pub to: String,

    /// Value in wei (hex or decimal string)
    pub value: String,

    /// Transaction data (hex)
    pub data: Option<String>,

    /// Chain ID
    pub chain_id: u32,
}

/// Result from the estimate transaction tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimateTransactionResult {
    /// Whether estimation succeeded
    pub success: bool,

    /// Estimated gas limit
    pub gas_limit: Option<u64>,

    /// Current gas price in gwei
    pub gas_price_gwei: Option<f64>,

    /// Estimated cost in ETH
    pub estimated_cost_eth: Option<f64>,

    /// Value being sent in ETH
    pub value_eth: Option<f64>,

    /// Human-readable summary
    pub summary: String,

    /// Error message if estimation failed
    pub error: Option<String>,
}

/// Execute the sign transaction tool
pub async fn execute_sign_transaction(tool: SignTransactionTool) -> SignTransactionResult {
    let client = SigilClient::new();

    // First check disk status
    match client.get_disk_status().await {
        Ok(status) if !status.detected => {
            return SignTransactionResult {
                success: false,
                signature: None,
                v: None,
                r: None,
                s: None,
                presig_index: None,
                proof_hash: None,
                error: Some(
                    "No signing disk detected. Please insert your Sigil floppy disk.".to_string(),
                ),
            };
        }
        Ok(status) if !status.is_valid.unwrap_or(false) => {
            return SignTransactionResult {
                success: false,
                signature: None,
                v: None,
                r: None,
                s: None,
                presig_index: None,
                proof_hash: None,
                error: Some(
                    "Signing disk is not valid. It may be expired or require reconciliation."
                        .to_string(),
                ),
            };
        }
        Err(ClientError::DaemonNotRunning) => {
            return SignTransactionResult {
                success: false,
                signature: None,
                v: None,
                r: None,
                s: None,
                presig_index: None,
                proof_hash: None,
                error: Some("Sigil daemon is not running. Start it with: sigil-daemon".to_string()),
            };
        }
        Err(e) => {
            return SignTransactionResult {
                success: false,
                signature: None,
                v: None,
                r: None,
                s: None,
                presig_index: None,
                proof_hash: None,
                error: Some(format!("Failed to check disk status: {}", e)),
            };
        }
        Ok(_) => {}
    }

    // Perform signing
    match client
        .sign(&tool.transaction_hash, tool.chain_id, &tool.description)
        .await
    {
        Ok(result) => {
            // Parse signature into r, s components
            let sig_bytes = hex::decode(&result.signature).unwrap_or_default();
            let (r, s) = if sig_bytes.len() == 64 {
                (hex::encode(&sig_bytes[..32]), hex::encode(&sig_bytes[32..]))
            } else {
                (String::new(), String::new())
            };

            // Calculate v (EIP-155)
            // v = recovery_id + chain_id * 2 + 35
            // For simplicity, we use v = 27 or 28 (legacy) here
            // A real implementation would determine recovery_id from the signature
            let v = 27; // Placeholder - would need actual recovery id calculation

            SignTransactionResult {
                success: true,
                signature: Some(format!("0x{}", result.signature)),
                v: Some(v),
                r: Some(format!("0x{}", r)),
                s: Some(format!("0x{}", s)),
                presig_index: Some(result.presig_index),
                proof_hash: Some(format!("0x{}", result.proof_hash)),
                error: None,
            }
        }
        Err(e) => SignTransactionResult {
            success: false,
            signature: None,
            v: None,
            r: None,
            s: None,
            presig_index: None,
            proof_hash: None,
            error: Some(format!("Signing failed: {}", e)),
        },
    }
}

/// Execute the check disk tool
pub async fn execute_check_disk(_tool: CheckDiskTool) -> CheckDiskResult {
    let client = SigilClient::new();

    match client.get_disk_status().await {
        Ok(status) => {
            let message = if !status.detected {
                "No signing disk detected. Please insert your Sigil floppy disk.".to_string()
            } else if !status.is_valid.unwrap_or(false) {
                format!(
                    "Disk sigil_{} detected but not valid for signing",
                    status.child_id.as_deref().unwrap_or("unknown")
                )
            } else {
                format!(
                    "Disk sigil_{} ready. {}/{} presigs, {} days remaining",
                    status.child_id.as_deref().unwrap_or("unknown"),
                    status.presigs_remaining.unwrap_or(0),
                    status.presigs_total.unwrap_or(0),
                    status.days_until_expiry.unwrap_or(0)
                )
            };

            CheckDiskResult {
                detected: status.detected,
                disk_id: status.child_id,
                presigs_remaining: status.presigs_remaining,
                presigs_total: status.presigs_total,
                days_until_expiry: status.days_until_expiry,
                is_valid: status.is_valid,
                message,
            }
        }
        Err(ClientError::DaemonNotRunning) => CheckDiskResult {
            detected: false,
            disk_id: None,
            presigs_remaining: None,
            presigs_total: None,
            days_until_expiry: None,
            is_valid: None,
            message: "Sigil daemon is not running. Start it with: sigil-daemon".to_string(),
        },
        Err(e) => CheckDiskResult {
            detected: false,
            disk_id: None,
            presigs_remaining: None,
            presigs_total: None,
            days_until_expiry: None,
            is_valid: None,
            message: format!("Failed to check disk status: {}", e),
        },
    }
}

/// Format disk status for Claude to display to user
pub fn format_disk_status_for_display(status: &CheckDiskResult) -> String {
    if !status.detected {
        return "🔐 Please insert your signing disk.".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!(
        "✓ Disk detected (sigil_{})\n",
        status.disk_id.as_deref().unwrap_or("?")
    ));
    output.push_str(&format!(
        "├─ Presigs: {}/{} remaining\n",
        status.presigs_remaining.unwrap_or(0),
        status.presigs_total.unwrap_or(0)
    ));
    output.push_str(&format!(
        "└─ Expires: {} days",
        status.days_until_expiry.unwrap_or(0)
    ));

    output
}

/// Format signing result for Claude to display to user
pub fn format_signing_result_for_display(result: &SignTransactionResult) -> String {
    if !result.success {
        return format!(
            "❌ Signing failed: {}",
            result.error.as_deref().unwrap_or("Unknown error")
        );
    }

    let mut output = String::new();
    output.push_str("✓ Signing... ✓ Proving... ✓ Done\n");
    output.push_str(&format!(
        "└─ Signature: {}",
        result
            .signature
            .as_deref()
            .map(truncate_for_display)
            .unwrap_or_else(|| "?".to_string())
    ));

    output
}

/// Shorten a value for display without panicking.
///
/// This was `&s[..18]`, which panics two ways: on a signature shorter than 18
/// bytes, and on a multi-byte character straddling the boundary. Both are
/// reachable — the shortening runs on whatever the daemon returned — and a
/// panic in the operator's signing path is a bad way to learn a signature was
/// malformed.
fn truncate_for_display(value: &str) -> String {
    const DISPLAY_CHARS: usize = 18;

    let shortened: String = value.chars().take(DISPLAY_CHARS).collect();
    if value.chars().count() > DISPLAY_CHARS {
        format!("{shortened}...")
    } else {
        shortened
    }
}

#[cfg(test)]
mod tests {
    //! `sigil-cli` is the operator's signing path and had zero tests. See
    //! `specs/003-operator-cli/`.

    use super::*;

    fn failed(error: &str) -> SignTransactionResult {
        SignTransactionResult {
            success: false,
            signature: None,
            v: None,
            r: None,
            s: None,
            presig_index: None,
            proof_hash: None,
            error: Some(error.to_string()),
        }
    }

    fn succeeded(signature: &str) -> SignTransactionResult {
        SignTransactionResult {
            success: true,
            signature: Some(signature.to_string()),
            v: Some(27),
            r: Some("0x11".to_string()),
            s: Some("0x22".to_string()),
            presig_index: Some(3),
            proof_hash: Some("0x33".to_string()),
            error: None,
        }
    }

    // ------------------------------------------------------------------
    // Display formatting must not panic
    // ------------------------------------------------------------------

    /// This was `&s[..18]`, which panics on a signature shorter than 18 bytes.
    /// A panic in the operator's signing path is a bad way to learn that the
    /// daemon returned something malformed.
    #[test]
    fn a_short_signature_does_not_panic_the_display() {
        let rendered = format_signing_result_for_display(&succeeded("0xabc"));
        assert!(rendered.contains("0xabc"));
    }

    /// The same slice also panicked when a multi-byte character straddled byte
    /// 18, because `str` indexing is by byte and must land on a char boundary.
    #[test]
    fn a_multibyte_signature_does_not_panic_the_display() {
        let rendered = format_signing_result_for_display(&succeeded("0x✓✓✓✓✓✓✓✓✓✓"));
        assert!(rendered.contains('✓'));
    }

    #[test]
    fn an_empty_signature_does_not_panic_the_display() {
        let rendered = format_signing_result_for_display(&succeeded(""));
        assert!(rendered.contains("Signature:"));
    }

    #[test]
    fn a_long_signature_is_shortened_with_an_ellipsis() {
        let long = format!("0x{}", "ab".repeat(64));
        let rendered = format_signing_result_for_display(&succeeded(&long));
        assert!(
            rendered.contains("..."),
            "a long signature should be elided"
        );
        assert!(
            !rendered.contains(&long),
            "the full signature should not be printed"
        );
    }

    #[test]
    fn a_failed_result_shows_the_error_not_a_signature() {
        let rendered = format_signing_result_for_display(&failed("no disk inserted"));
        assert!(rendered.contains("no disk inserted"));
        assert!(!rendered.contains("Done"));
    }

    #[test]
    fn a_failed_result_without_an_error_message_still_renders() {
        let mut result = failed("placeholder");
        result.error = None;
        let rendered = format_signing_result_for_display(&result);
        assert!(rendered.contains("Unknown error"));
    }

    // ------------------------------------------------------------------
    // Disk status display
    // ------------------------------------------------------------------

    /// No disk is the common case, and the operator's instruction must be the
    /// whole message — not buried under counts that do not exist.
    #[test]
    fn an_absent_disk_asks_the_operator_to_insert_one() {
        let status = CheckDiskResult {
            detected: false,
            disk_id: None,
            presigs_remaining: None,
            presigs_total: None,
            days_until_expiry: None,
            is_valid: None,
            message: String::new(),
        };

        let rendered = format_disk_status_for_display(&status);
        assert!(rendered.contains("insert"), "got: {rendered}");
        assert!(!rendered.contains("Presigs"));
    }

    #[test]
    fn a_detected_disk_shows_its_remaining_presignatures_and_expiry() {
        let status = CheckDiskResult {
            detected: true,
            disk_id: Some("9e8d7c6b".to_string()),
            presigs_remaining: Some(742),
            presigs_total: Some(1000),
            days_until_expiry: Some(30),
            is_valid: Some(true),
            message: String::new(),
        };

        let rendered = format_disk_status_for_display(&status);
        assert!(rendered.contains("9e8d7c6b"));
        assert!(rendered.contains("742/1000"));
        assert!(rendered.contains("30"));
    }

    /// Missing counts must render as zero rather than panicking or printing a
    /// misleading number.
    #[test]
    fn a_detected_disk_with_unknown_counts_renders_without_panicking() {
        let status = CheckDiskResult {
            detected: true,
            disk_id: None,
            presigs_remaining: None,
            presigs_total: None,
            days_until_expiry: None,
            is_valid: None,
            message: String::new(),
        };

        let rendered = format_disk_status_for_display(&status);
        assert!(rendered.contains("0/0"));
    }
}
