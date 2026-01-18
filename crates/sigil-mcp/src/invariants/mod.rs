//! Invariants and validation for Sigil MCP server
//!
//! This module contains runtime invariant checks, input validation,
//! and protocol compliance verification.

use crate::protocol::{JsonRpcError, JsonRpcRequest, RequestId};

/// Validation result
pub type ValidationResult<T> = Result<T, JsonRpcError>;

// ============================================================================
// Protocol Invariants
// ============================================================================

/// Validate that a JSON-RPC request is well-formed
pub fn validate_request(request: &JsonRpcRequest) -> ValidationResult<()> {
    // Invariant: JSON-RPC version must be "2.0"
    if request.jsonrpc != "2.0" {
        return Err(JsonRpcError::invalid_request());
    }

    // Invariant: Method must not be empty
    if request.method.is_empty() {
        return Err(JsonRpcError::invalid_request());
    }

    // Invariant: Method must not start with "rpc." (reserved)
    if request.method.starts_with("rpc.") {
        return Err(JsonRpcError::invalid_request());
    }

    Ok(())
}

/// Validate request ID is not null for requests (notifications are different)
pub fn validate_request_id(id: &RequestId) -> ValidationResult<()> {
    match id {
        RequestId::Null => Err(JsonRpcError::invalid_request()),
        _ => Ok(()),
    }
}

// ============================================================================
// Input Validation
// ============================================================================

/// Validate a hex string with 0x prefix
pub fn validate_hex_string(s: &str, expected_bytes: Option<usize>) -> ValidationResult<Vec<u8>> {
    // Invariant: Must start with 0x
    if !s.starts_with("0x") {
        return Err(JsonRpcError::invalid_params(
            "Hex string must start with '0x' prefix",
        ));
    }

    let hex_part = &s[2..];

    // Invariant: Must have even length
    if hex_part.len() % 2 != 0 {
        return Err(JsonRpcError::invalid_params(
            "Hex string must have even number of characters",
        ));
    }

    // Invariant: Must be valid hex
    let bytes = hex::decode(hex_part).map_err(|e| {
        JsonRpcError::invalid_params(format!("Invalid hex string: {}", e))
    })?;

    // Invariant: If expected length specified, must match
    if let Some(expected) = expected_bytes {
        if bytes.len() != expected {
            return Err(JsonRpcError::invalid_params(format!(
                "Expected {} bytes, got {}",
                expected,
                bytes.len()
            )));
        }
    }

    Ok(bytes)
}

/// Validate a transaction hash (32 bytes)
pub fn validate_tx_hash(hash: &str) -> ValidationResult<[u8; 32]> {
    let bytes = validate_hex_string(hash, Some(32))?;
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Validate an EVM chain ID
pub fn validate_chain_id(chain_id: u32) -> ValidationResult<u32> {
    // Invariant: Chain ID must be positive
    if chain_id == 0 {
        return Err(JsonRpcError::invalid_params("Chain ID must be positive"));
    }

    // Known chain IDs (not exhaustive, just for reference)
    // 1 = Ethereum, 137 = Polygon, 42161 = Arbitrum, etc.
    // We allow any positive chain ID

    Ok(chain_id)
}

/// Validate a signature scheme
pub fn validate_scheme(scheme: &str) -> ValidationResult<&str> {
    match scheme {
        "ecdsa" | "taproot" | "ed25519" | "ristretto255" => Ok(scheme),
        _ => Err(JsonRpcError::invalid_params(format!(
            "Unknown signature scheme: {}. Supported: ecdsa, taproot, ed25519, ristretto255",
            scheme
        ))),
    }
}

/// Validate a description string
pub fn validate_description(description: &str, max_len: usize) -> ValidationResult<&str> {
    // Invariant: Description must not be empty
    if description.is_empty() {
        return Err(JsonRpcError::invalid_params("Description must not be empty"));
    }

    // Invariant: Description must not exceed max length
    if description.len() > max_len {
        return Err(JsonRpcError::invalid_params(format!(
            "Description exceeds maximum length of {} characters",
            max_len
        )));
    }

    // Invariant: Description must not contain control characters (except newlines)
    for c in description.chars() {
        if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
            return Err(JsonRpcError::invalid_params(
                "Description contains invalid control characters",
            ));
        }
    }

    Ok(description)
}

/// Validate an EVM address
pub fn validate_evm_address(address: &str) -> ValidationResult<[u8; 20]> {
    // Invariant: Must start with 0x
    if !address.starts_with("0x") {
        return Err(JsonRpcError::invalid_params(
            "EVM address must start with '0x'",
        ));
    }

    // Invariant: Must be exactly 42 characters (0x + 40 hex chars)
    if address.len() != 42 {
        return Err(JsonRpcError::invalid_params(
            "EVM address must be 42 characters (0x + 40 hex)",
        ));
    }

    let bytes = validate_hex_string(address, Some(20))?;
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Validate a resource URI
pub fn validate_resource_uri(uri: &str) -> ValidationResult<&str> {
    // Invariant: Must start with sigil://
    if !uri.starts_with("sigil://") {
        return Err(JsonRpcError::invalid_params(
            "Resource URI must start with 'sigil://'",
        ));
    }

    // Invariant: Must have a path after sigil://
    let path = &uri[8..]; // After "sigil://"
    if path.is_empty() {
        return Err(JsonRpcError::invalid_params(
            "Resource URI must have a path after 'sigil://'",
        ));
    }

    // Invariant: Path must not contain ".."
    if path.contains("..") {
        return Err(JsonRpcError::invalid_params(
            "Resource URI must not contain '..'",
        ));
    }

    Ok(uri)
}

// ============================================================================
// State Invariants
// ============================================================================

/// Validate disk state invariants
pub fn validate_disk_state(
    detected: bool,
    presigs_remaining: Option<u32>,
    presigs_total: Option<u32>,
) -> ValidationResult<()> {
    // Invariant: If disk not detected, no presig counts should be present
    if !detected {
        if presigs_remaining.is_some() || presigs_total.is_some() {
            // This is a warning, not an error - log it
            tracing::warn!(
                "Disk state invariant: presig counts present but disk not detected"
            );
        }
        return Ok(());
    }

    // Invariant: If both counts present, remaining <= total
    if let (Some(remaining), Some(total)) = (presigs_remaining, presigs_total) {
        if remaining > total {
            return Err(JsonRpcError::internal_error(
                "Disk state invariant violated: remaining > total presigs",
            ));
        }
    }

    Ok(())
}

/// Validate presig index
pub fn validate_presig_index(index: u32, max_index: u32) -> ValidationResult<u32> {
    // Invariant: Index must be less than max
    if index >= max_index {
        return Err(JsonRpcError::invalid_params(format!(
            "Presig index {} out of range (max: {})",
            index, max_index
        )));
    }
    Ok(index)
}

// ============================================================================
// Debug Assertions (only in debug builds)
// ============================================================================

/// Assert a protocol invariant (panics in debug, logs warning in release)
#[macro_export]
macro_rules! assert_invariant {
    ($cond:expr, $msg:expr) => {
        if cfg!(debug_assertions) {
            assert!($cond, "Invariant violated: {}", $msg);
        } else if !$cond {
            tracing::error!("Invariant violated: {}", $msg);
        }
    };
}

/// Assert that a value is Some (panics in debug, returns error in release)
#[macro_export]
macro_rules! require_some {
    ($opt:expr, $msg:expr) => {
        match $opt {
            Some(v) => v,
            None => {
                if cfg!(debug_assertions) {
                    panic!("Required value missing: {}", $msg);
                } else {
                    return Err(JsonRpcError::internal_error(format!(
                        "Required value missing: {}",
                        $msg
                    )));
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_hex_string_valid() {
        let result = validate_hex_string("0xabcd", Some(2));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0xab, 0xcd]);
    }

    #[test]
    fn test_validate_hex_string_no_prefix() {
        let result = validate_hex_string("abcd", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_hex_string_wrong_length() {
        let result = validate_hex_string("0xabcd", Some(4));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_tx_hash_valid() {
        let hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = validate_tx_hash(hash);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tx_hash_wrong_length() {
        let result = validate_tx_hash("0xabcd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_chain_id_valid() {
        assert!(validate_chain_id(1).is_ok());
        assert!(validate_chain_id(137).is_ok());
    }

    #[test]
    fn test_validate_chain_id_zero() {
        assert!(validate_chain_id(0).is_err());
    }

    #[test]
    fn test_validate_scheme_valid() {
        assert!(validate_scheme("ecdsa").is_ok());
        assert!(validate_scheme("taproot").is_ok());
        assert!(validate_scheme("ed25519").is_ok());
        assert!(validate_scheme("ristretto255").is_ok());
    }

    #[test]
    fn test_validate_scheme_invalid() {
        assert!(validate_scheme("unknown").is_err());
        assert!(validate_scheme("rsa").is_err());
    }

    #[test]
    fn test_validate_description_valid() {
        assert!(validate_description("Transfer 0.1 ETH", 256).is_ok());
    }

    #[test]
    fn test_validate_description_empty() {
        assert!(validate_description("", 256).is_err());
    }

    #[test]
    fn test_validate_description_too_long() {
        let long_desc = "a".repeat(300);
        assert!(validate_description(&long_desc, 256).is_err());
    }

    #[test]
    fn test_validate_evm_address_valid() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f12345";
        assert!(validate_evm_address(addr).is_ok());
    }

    #[test]
    fn test_validate_evm_address_wrong_length() {
        let addr = "0x742d35Cc6634";
        assert!(validate_evm_address(addr).is_err());
    }

    #[test]
    fn test_validate_resource_uri_valid() {
        assert!(validate_resource_uri("sigil://disk/status").is_ok());
        assert!(validate_resource_uri("sigil://presigs/info").is_ok());
    }

    #[test]
    fn test_validate_resource_uri_invalid_prefix() {
        assert!(validate_resource_uri("http://disk/status").is_err());
    }

    #[test]
    fn test_validate_resource_uri_path_traversal() {
        assert!(validate_resource_uri("sigil://../etc/passwd").is_err());
    }

    #[test]
    fn test_validate_disk_state_valid() {
        // Detected with valid counts
        assert!(validate_disk_state(true, Some(500), Some(1000)).is_ok());

        // Not detected
        assert!(validate_disk_state(false, None, None).is_ok());
    }

    #[test]
    fn test_validate_disk_state_invalid() {
        // Remaining > total is invalid
        assert!(validate_disk_state(true, Some(1500), Some(1000)).is_err());
    }

    #[test]
    fn test_validate_request_valid() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "tools/list".to_string(),
            params: None,
        };
        assert!(validate_request(&request).is_ok());
    }

    #[test]
    fn test_validate_request_wrong_version() {
        let request = JsonRpcRequest {
            jsonrpc: "1.0".to_string(),
            id: RequestId::Number(1),
            method: "tools/list".to_string(),
            params: None,
        };
        assert!(validate_request(&request).is_err());
    }

    #[test]
    fn test_validate_request_reserved_method() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "rpc.internal".to_string(),
            params: None,
        };
        assert!(validate_request(&request).is_err());
    }
}
