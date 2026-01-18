//! MCP lifecycle management
//!
//! Handles initialization, capability negotiation, and shutdown.

use serde::{Deserialize, Serialize};

use super::capabilities::{ClientCapabilities, ClientInfo, ServerCapabilities, ServerInfo};

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Protocol version the client wants to use
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client information
    pub client_info: ClientInfo,
}

/// Initialize response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version the server is using
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server information
    pub server_info: ServerInfo,

    /// Optional instructions for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

impl InitializeResult {
    pub fn new(protocol_version: String) -> Self {
        Self {
            protocol_version,
            capabilities: ServerCapabilities::default(),
            server_info: ServerInfo::default(),
            instructions: Some(
                "Sigil MCP server provides secure MPC-based blockchain transaction signing. \
                 Use sigil_check_disk to verify disk status before signing operations."
                    .to_string(),
            ),
        }
    }
}

/// Initialized notification (sent by client after receiving InitializeResult)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InitializedNotification {}

/// Ping request/response for keepalive
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PingParams {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PingResult {}

/// Cancellation notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelledNotification {
    /// The ID of the request to cancel
    pub request_id: super::jsonrpc::RequestId,

    /// Optional reason for cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Progress notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotification {
    /// Token identifying this progress stream
    pub progress_token: String,

    /// Progress value (0.0 to 1.0)
    pub progress: f64,

    /// Optional total for the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// Log level for logging messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Logging message notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMessage {
    /// Log level
    pub level: LogLevel,

    /// Logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,

    /// Log message data
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_params_deserialize() {
        let json = r#"{
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }"#;

        let params: InitializeParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.protocol_version, "2025-11-25");
        assert_eq!(params.client_info.name, "test-client");
    }

    #[test]
    fn test_initialize_result_serialize() {
        let result = InitializeResult::new("2025-11-25".to_string());
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("sigil-mcp"));
    }
}
