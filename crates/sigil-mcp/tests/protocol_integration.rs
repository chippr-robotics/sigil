//! Integration tests for MCP protocol flow
//!
//! These tests verify the complete MCP protocol implementation including
//! initialization, capability negotiation, and tool/resource/prompt operations.

use sigil_mcp::handlers::{handle_notification, handle_request, McpServerState};
use sigil_mcp::protocol::*;
use sigil_mcp::tools::DiskState;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Helper to create a test server state
fn create_test_state() -> McpServerState {
    let mut state = McpServerState::new();
    state.disk_state = Arc::new(RwLock::new(DiskState::mock_detected()));
    state
}

/// Helper to create an initialized server state
fn create_initialized_state() -> McpServerState {
    let mut state = create_test_state();
    state.initialized = true;
    state.protocol_version = Some(MCP_PROTOCOL_VERSION.to_string());
    state
}

// ============================================================================
// Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_full_initialization_flow() {
    let mut state = create_test_state();

    // Step 1: Send initialize request
    let init_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "initialize".to_string(),
        params: Some(serde_json::json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {
                "roots": { "listChanged": true }
            },
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        })),
    };

    let response = handle_request(&mut state, &init_request).await;

    // Verify response
    assert!(response.error.is_none(), "Initialize should succeed");
    let result = response.result.expect("Should have result");

    // Verify protocol version
    assert_eq!(result["protocolVersion"], "2025-11-25");

    // Verify server capabilities
    let capabilities = &result["capabilities"];
    assert!(capabilities["tools"].is_object());
    assert!(capabilities["resources"].is_object());
    assert!(capabilities["prompts"].is_object());

    // Verify server info
    assert_eq!(result["serverInfo"]["name"], "sigil-mcp");

    // Step 2: Send initialized notification
    let initialized_notification = JsonRpcNotification {
        jsonrpc: "2.0".to_string(),
        method: "notifications/initialized".to_string(),
        params: None,
    };

    handle_notification(&mut state, &initialized_notification).await;

    // Verify state is now initialized
    assert!(state.initialized);
    assert_eq!(state.protocol_version.as_deref(), Some("2025-11-25"));
}

#[tokio::test]
async fn test_request_before_initialize_fails() {
    let mut state = create_test_state();

    // Try to list tools before initialize
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;

    // Should fail with error
    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, -32002); // Server not initialized
}

#[tokio::test]
async fn test_ping_works_without_initialize() {
    let mut state = create_test_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "ping".to_string(),
        params: None,
    };

    let response = handle_request(&mut state, &request).await;

    // Ping should work without initialization
    assert!(response.error.is_none());
}

// ============================================================================
// Tools Tests
// ============================================================================

#[tokio::test]
async fn test_tools_list_returns_all_tools() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let tools = result["tools"].as_array().unwrap();

    // Verify we have all expected tools
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"sigil_check_disk"));
    assert!(tool_names.contains(&"sigil_sign_evm"));
    assert!(tool_names.contains(&"sigil_sign_frost"));
    assert!(tool_names.contains(&"sigil_get_address"));
    assert!(tool_names.contains(&"sigil_update_tx_hash"));
    assert!(tool_names.contains(&"sigil_list_schemes"));
    assert!(tool_names.contains(&"sigil_get_presig_count"));

    // Verify tool structure
    for tool in tools {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        assert!(tool["inputSchema"].is_object());
    }
}

#[tokio::test]
async fn test_tools_call_check_disk() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "sigil_check_disk",
            "arguments": {}
        })),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();

    // Verify content structure
    assert!(result["content"].is_array());
    let content = &result["content"][0];
    assert_eq!(content["type"], "text");

    // Verify structured content
    let structured = &result["structuredContent"];
    assert_eq!(structured["detected"], true);
    assert!(structured["child_id"].is_string());
    assert!(structured["scheme"].is_string());
}

#[tokio::test]
async fn test_tools_call_sign_evm_validation() {
    let mut state = create_initialized_state();

    // Test with invalid hash
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "sigil_sign_evm",
            "arguments": {
                "message_hash": "invalid",
                "chain_id": 1,
                "description": "Test"
            }
        })),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none()); // Tool returns error in result
    let result = response.result.unwrap();
    assert_eq!(result["isError"], true);
}

#[tokio::test]
async fn test_tools_call_unknown_tool() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "unknown_tool",
            "arguments": {}
        })),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(result["isError"], true);
}

// ============================================================================
// Resources Tests
// ============================================================================

#[tokio::test]
async fn test_resources_list() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "resources/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let resources = result["resources"].as_array().unwrap();

    // Should have disk status and other resources
    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();

    assert!(uris.contains(&"sigil://supported-chains"));
    assert!(uris.contains(&"sigil://disk/status")); // Disk is mock_detected
}

#[tokio::test]
async fn test_resources_read_supported_chains() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "resources/read".to_string(),
        params: Some(serde_json::json!({
            "uri": "sigil://supported-chains"
        })),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let contents = result["contents"].as_array().unwrap();

    assert!(!contents.is_empty());
    let content = &contents[0];
    assert_eq!(content["uri"], "sigil://supported-chains");
    assert!(content["text"].is_string());

    // Verify content includes chain info
    let text = content["text"].as_str().unwrap();
    assert!(text.contains("evm_chains"));
    assert!(text.contains("frost_chains"));
}

#[tokio::test]
async fn test_resources_read_unknown_uri() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "resources/read".to_string(),
        params: Some(serde_json::json!({
            "uri": "sigil://unknown"
        })),
    };

    let response = handle_request(&mut state, &request).await;

    // Should return error for unknown resource
    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, -32002); // Resource not found
}

#[tokio::test]
async fn test_resources_templates_list() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "resources/templates/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let templates = result["resourceTemplates"].as_array().unwrap();

    // Should have child template
    let has_children_template = templates
        .iter()
        .any(|t| t["uriTemplate"].as_str() == Some("sigil://children/{child_id}"));
    assert!(has_children_template);
}

// ============================================================================
// Prompts Tests
// ============================================================================

#[tokio::test]
async fn test_prompts_list() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "prompts/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let prompts = result["prompts"].as_array().unwrap();

    let prompt_names: Vec<&str> = prompts
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();

    assert!(prompt_names.contains(&"sign_evm_transfer"));
    assert!(prompt_names.contains(&"sign_bitcoin_taproot"));
    assert!(prompt_names.contains(&"troubleshoot_disk"));
}

#[tokio::test]
async fn test_prompts_get_with_arguments() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "prompts/get".to_string(),
        params: Some(serde_json::json!({
            "name": "sign_evm_transfer",
            "arguments": {
                "to_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f12345",
                "amount": "0.1",
                "chain_id": 1
            }
        })),
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_none());
    let result = response.result.unwrap();

    assert!(result["messages"].is_array());
    let messages = result["messages"].as_array().unwrap();
    assert!(!messages.is_empty());

    let message = &messages[0];
    assert_eq!(message["role"], "user");

    // Verify arguments were substituted
    let content = message["content"]["text"].as_str().unwrap();
    assert!(content.contains("0x742d35Cc6634C0532925a3b844Bc9e7595f12345"));
    assert!(content.contains("0.1"));
}

#[tokio::test]
async fn test_prompts_get_missing_arguments() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "prompts/get".to_string(),
        params: Some(serde_json::json!({
            "name": "sign_evm_transfer",
            "arguments": {}  // Missing required arguments
        })),
    };

    let response = handle_request(&mut state, &request).await;

    // Should fail with invalid params
    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, -32602); // Invalid params
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_unknown_method() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "unknown/method".to_string(),
        params: None,
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, -32601); // Method not found
}

#[tokio::test]
async fn test_invalid_params() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!("invalid")), // Should be object
    };

    let response = handle_request(&mut state, &request).await;

    assert!(response.error.is_some());
    let error = response.error.unwrap();
    assert_eq!(error.code, -32602); // Invalid params
}

// ============================================================================
// JSON-RPC Compliance Tests
// ============================================================================

#[tokio::test]
async fn test_response_contains_same_id() {
    let mut state = create_initialized_state();

    // Test with string ID
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::String("test-id-123".to_string()),
        method: "tools/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;
    assert_eq!(response.id, RequestId::String("test-id-123".to_string()));

    // Test with number ID
    let request2 = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(42),
        method: "tools/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response2 = handle_request(&mut state, &request2).await;
    assert_eq!(response2.id, RequestId::Number(42));
}

#[tokio::test]
async fn test_response_contains_jsonrpc_version() {
    let mut state = create_initialized_state();

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;
    assert_eq!(response.jsonrpc, "2.0");
}

// ============================================================================
// Disk State Tests
// ============================================================================

#[tokio::test]
async fn test_operations_with_no_disk() {
    let mut state = create_initialized_state();

    // Set disk state to no disk
    {
        let mut disk = state.disk_state.write().await;
        *disk = DiskState::no_disk();
    }

    // Check disk should report no disk
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "sigil_check_disk",
            "arguments": {}
        })),
    };

    let response = handle_request(&mut state, &request).await;
    let result = response.result.unwrap();
    let structured = &result["structuredContent"];
    assert_eq!(structured["detected"], false);

    // Sign should fail without disk
    let sign_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(2),
        method: "tools/call".to_string(),
        params: Some(serde_json::json!({
            "name": "sigil_sign_evm",
            "arguments": {
                "message_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                "chain_id": 1,
                "description": "Test"
            }
        })),
    };

    let sign_response = handle_request(&mut state, &sign_request).await;
    let sign_result = sign_response.result.unwrap();
    assert_eq!(sign_result["isError"], true);
}

#[tokio::test]
async fn test_resources_list_without_disk() {
    let mut state = create_initialized_state();

    // Set disk state to no disk
    {
        let mut disk = state.disk_state.write().await;
        *disk = DiskState::no_disk();
    }

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "resources/list".to_string(),
        params: Some(serde_json::json!({})),
    };

    let response = handle_request(&mut state, &request).await;
    let result = response.result.unwrap();
    let resources = result["resources"].as_array().unwrap();

    // Should still have supported-chains but not disk-specific resources
    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();

    assert!(uris.contains(&"sigil://supported-chains"));
    assert!(!uris.contains(&"sigil://disk/status")); // No disk
}
