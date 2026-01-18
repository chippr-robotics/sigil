//! MCP request handlers
//!
//! This module contains handlers for all MCP protocol methods.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::protocol::*;
use crate::prompts;
use crate::resources;
use crate::tools::{self, DiskState, ToolContext};

/// MCP Server state
pub struct McpServerState {
    /// Protocol version negotiated
    pub protocol_version: Option<String>,

    /// Whether initialized
    pub initialized: bool,

    /// Client capabilities
    pub client_capabilities: Option<ClientCapabilities>,

    /// Client info
    pub client_info: Option<ClientInfo>,

    /// Disk state
    pub disk_state: Arc<RwLock<DiskState>>,

    /// Resource subscriptions
    pub subscriptions: HashMap<String, Vec<String>>, // uri -> session_ids
}

impl McpServerState {
    pub fn new() -> Self {
        Self {
            protocol_version: None,
            initialized: false,
            client_capabilities: None,
            client_info: None,
            disk_state: Arc::new(RwLock::new(DiskState::mock_detected())), // Start with mock for testing
            subscriptions: HashMap::new(),
        }
    }

    pub fn tool_context(&self) -> ToolContext {
        ToolContext {
            disk_state: Arc::clone(&self.disk_state),
        }
    }
}

impl Default for McpServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle an incoming JSON-RPC request
pub async fn handle_request(
    state: &mut McpServerState,
    request: &JsonRpcRequest,
) -> JsonRpcResponse {
    debug!("Handling request: {} (id: {})", request.method, request.id);

    // Check if initialized (except for initialize itself)
    if !state.initialized && request.method != "initialize" && request.method != "ping" {
        return JsonRpcResponse::error(
            request.id.clone(),
            JsonRpcError::new(-32002, "Server not initialized"),
        );
    }

    let result = match request.method.as_str() {
        // Lifecycle
        "initialize" => handle_initialize(state, request).await,
        "ping" => handle_ping().await,

        // Tools
        "tools/list" => handle_tools_list(state, request).await,
        "tools/call" => handle_tools_call(state, request).await,

        // Resources
        "resources/list" => handle_resources_list(state, request).await,
        "resources/read" => handle_resources_read(state, request).await,
        "resources/templates/list" => handle_resources_templates_list(state, request).await,
        "resources/subscribe" => handle_resources_subscribe(state, request).await,

        // Prompts
        "prompts/list" => handle_prompts_list(state, request).await,
        "prompts/get" => handle_prompts_get(state, request).await,

        // Unknown method
        _ => Err(JsonRpcError::method_not_found(&request.method)),
    };

    match result {
        Ok(value) => JsonRpcResponse::success(request.id.clone(), value),
        Err(error) => JsonRpcResponse::error(request.id.clone(), error),
    }
}

/// Handle an incoming notification
pub async fn handle_notification(
    state: &mut McpServerState,
    notification: &JsonRpcNotification,
) -> Option<JsonRpcNotification> {
    debug!("Handling notification: {}", notification.method);

    match notification.method.as_str() {
        "notifications/initialized" => {
            info!("Client sent initialized notification");
            state.initialized = true;
            None
        }
        "notifications/cancelled" => {
            // Handle cancellation
            if let Some(params) = &notification.params {
                if let Some(request_id) = params.get("requestId") {
                    warn!("Request cancelled: {:?}", request_id);
                }
            }
            None
        }
        _ => {
            debug!("Unknown notification: {}", notification.method);
            None
        }
    }
}

// ============================================================================
// Lifecycle Handlers
// ============================================================================

async fn handle_initialize(
    state: &mut McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let params: InitializeParams = request
        .params
        .as_ref()
        .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone())
                .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))
        })?;

    info!(
        "Initialize request from {} (version: {})",
        params.client_info.name, params.protocol_version
    );

    // Check protocol version
    if params.protocol_version != MCP_PROTOCOL_VERSION {
        warn!(
            "Protocol version mismatch: client={}, server={}",
            params.protocol_version, MCP_PROTOCOL_VERSION
        );
        // We'll accept it anyway for compatibility
    }

    // Store client info
    state.protocol_version = Some(params.protocol_version.clone());
    state.client_capabilities = Some(params.capabilities);
    state.client_info = Some(params.client_info);

    // Build response
    let result = InitializeResult::new(MCP_PROTOCOL_VERSION.to_string());

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_ping() -> Result<serde_json::Value, JsonRpcError> {
    Ok(serde_json::json!({}))
}

// ============================================================================
// Tools Handlers
// ============================================================================

async fn handle_tools_list(
    _state: &McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let _params: ToolsListParams = request
        .params
        .as_ref()
        .map(|p| serde_json::from_value(p.clone()))
        .transpose()
        .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?
        .unwrap_or_default();

    let tools = tools::get_all_tools();
    let result = ToolsListResult {
        tools,
        next_cursor: None,
    };

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_tools_call(
    state: &McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let params: ToolsCallParams = request
        .params
        .as_ref()
        .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone())
                .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))
        })?;

    debug!("Calling tool: {}", params.name);

    let ctx = state.tool_context();
    let result = tools::execute_tool(&ctx, &params.name, params.arguments).await;

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

// ============================================================================
// Resources Handlers
// ============================================================================

async fn handle_resources_list(
    state: &McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let _params: ResourcesListParams = request
        .params
        .as_ref()
        .map(|p| serde_json::from_value(p.clone()))
        .transpose()
        .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?
        .unwrap_or_default();

    let disk_state = state.disk_state.read().await;
    let resources_list = resources::get_all_resources(&disk_state);

    let result = ResourcesListResult {
        resources: resources_list,
        next_cursor: None,
    };

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_resources_read(
    state: &McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let params: ResourcesReadParams = request
        .params
        .as_ref()
        .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone())
                .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))
        })?;

    debug!("Reading resource: {}", params.uri);

    let ctx = state.tool_context();
    let result = resources::read_resource(&ctx, &params.uri)
        .await
        .map_err(|e| JsonRpcError::resource_not_found(&e))?;

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_resources_templates_list(
    _state: &McpServerState,
    _request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let templates = resources::get_all_templates();

    let result = ResourceTemplatesListResult {
        resource_templates: templates,
        next_cursor: None,
    };

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_resources_subscribe(
    state: &mut McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let params: ResourcesSubscribeParams = request
        .params
        .as_ref()
        .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone())
                .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))
        })?;

    debug!("Subscribing to resource: {}", params.uri);

    // Add subscription (simplified - in real impl would track by session)
    state
        .subscriptions
        .entry(params.uri)
        .or_default()
        .push("default".to_string());

    Ok(serde_json::json!({}))
}

// ============================================================================
// Prompts Handlers
// ============================================================================

async fn handle_prompts_list(
    _state: &McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let _params: PromptsListParams = request
        .params
        .as_ref()
        .map(|p| serde_json::from_value(p.clone()))
        .transpose()
        .map_err(|e| JsonRpcError::invalid_params(e.to_string()))?
        .unwrap_or_default();

    let prompts_list = prompts::get_all_prompts();

    let result = PromptsListResult {
        prompts: prompts_list,
        next_cursor: None,
    };

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

async fn handle_prompts_get(
    _state: &McpServerState,
    request: &JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcError> {
    let params: PromptsGetParams = request
        .params
        .as_ref()
        .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))
        .and_then(|p| {
            serde_json::from_value(p.clone())
                .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {}", e)))
        })?;

    debug!("Getting prompt: {}", params.name);

    // Convert params.arguments to HashMap
    let args: Option<HashMap<String, serde_json::Value>> =
        params.arguments.map(|m| m.into_iter().collect());

    let result = prompts::get_prompt(&params.name, args.as_ref())
        .map_err(|e| JsonRpcError::invalid_params(e))?;

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_initialize() {
        let mut state = McpServerState::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            })),
        };

        let response = handle_request(&mut state, &request).await;
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_tools_list() {
        let mut state = McpServerState::new();
        state.initialized = true;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "tools/list".to_string(),
            params: Some(serde_json::json!({})),
        };

        let response = handle_request(&mut state, &request).await;
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert!(result["tools"].is_array());
    }

    #[tokio::test]
    async fn test_handle_prompts_list() {
        let mut state = McpServerState::new();
        state.initialized = true;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "prompts/list".to_string(),
            params: Some(serde_json::json!({})),
        };

        let response = handle_request(&mut state, &request).await;
        assert!(response.result.is_some());
    }
}
