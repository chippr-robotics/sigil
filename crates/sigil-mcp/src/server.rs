//! MCP Server implementation
//!
//! The main server that handles the MCP protocol over various transports.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::client::{ClientError, DaemonClient};
use crate::handlers::{handle_notification, handle_request, McpServerState};
use crate::protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, JSONRPC_VERSION};
use crate::tools::DiskState;
use crate::transport::stdio::AsyncStdioTransport;

/// MCP Server
pub struct McpServer {
    state: Arc<RwLock<McpServerState>>,
}

impl McpServer {
    /// Create a new MCP server with mock data
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(McpServerState::new_with_mock())),
        }
    }

    /// Create a server with mock daemon client
    pub fn with_mock() -> Self {
        Self {
            state: Arc::new(RwLock::new(McpServerState::new_with_mock())),
        }
    }

    /// Create a server that connects to the real daemon
    pub fn with_daemon() -> Result<Self, ClientError> {
        Ok(Self {
            state: Arc::new(RwLock::new(McpServerState::new_with_daemon()?)),
        })
    }

    /// Create a server with a specific disk state (for testing)
    #[allow(dead_code)]
    pub fn with_disk_state(disk_state: DiskState) -> Self {
        Self {
            state: Arc::new(RwLock::new(McpServerState {
                protocol_version: None,
                initialized: false,
                client_capabilities: None,
                client_info: None,
                daemon_client: Arc::new(DaemonClient::new_mock(disk_state)),
                subscriptions: std::collections::HashMap::new(),
            })),
        }
    }

    /// Run the server using stdio transport
    pub async fn run_stdio(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting Sigil MCP server (stdio transport)");

        let mut transport = AsyncStdioTransport::new();

        loop {
            // Read message
            let message = match transport.read_message().await {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    info!("EOF received, shutting down");
                    break;
                }
                Err(e) => {
                    error!("Error reading message: {}", e);
                    continue;
                }
            };

            // Parse JSON
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&message);
            let json = match parsed {
                Ok(v) => v,
                Err(e) => {
                    warn!("Failed to parse JSON: {}", e);
                    let error_response = JsonRpcResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        id: crate::protocol::RequestId::Null,
                        result: None,
                        error: Some(crate::protocol::JsonRpcError::parse_error()),
                    };
                    if let Err(e) = transport.write_response(&error_response).await {
                        error!("Failed to write error response: {}", e);
                    }
                    continue;
                }
            };

            // Determine message type
            if json.get("id").is_some() && json.get("method").is_some() {
                // It's a request
                let request: JsonRpcRequest = match serde_json::from_value(json) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Failed to parse request: {}", e);
                        continue;
                    }
                };

                let mut state = self.state.write().await;
                let response = handle_request(&mut state, &request).await;

                if let Err(e) = transport.write_response(&response).await {
                    error!("Failed to write response: {}", e);
                }
            } else if json.get("method").is_some() && json.get("id").is_none() {
                // It's a notification
                let notification: JsonRpcNotification = match serde_json::from_value(json) {
                    Ok(n) => n,
                    Err(e) => {
                        warn!("Failed to parse notification: {}", e);
                        continue;
                    }
                };

                let mut state = self.state.write().await;
                if let Some(response_notification) =
                    handle_notification(&mut state, &notification).await
                {
                    if let Err(e) = transport.write_notification(&response_notification).await {
                        error!("Failed to write notification: {}", e);
                    }
                }
            } else if json.get("id").is_some() && json.get("result").is_some()
                || json.get("error").is_some()
            {
                // It's a response (from client to server's request)
                debug!("Received response from client (ignored for now)");
            } else {
                warn!("Unknown message type: {:?}", json);
            }
        }

        info!("Sigil MCP server stopped");
        Ok(())
    }

    /// Get current disk state
    pub async fn get_disk_state(&self) -> Result<DiskState, ClientError> {
        let state = self.state.read().await;
        state.daemon_client.get_disk_status().await
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let server = McpServer::new();
        let disk_state = server.get_disk_state().await.unwrap();
        assert!(disk_state.detected); // Default is mock_detected
    }

    #[tokio::test]
    async fn test_server_with_custom_disk() {
        let server = McpServer::with_disk_state(DiskState::default());
        let disk_state = server.get_disk_state().await.unwrap();
        assert!(!disk_state.detected);
    }
}
