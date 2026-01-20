//! IPC client implementation

use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::error::Result;

use super::connection::{IpcClientTransport, PlatformClient};
use super::types::{IpcRequest, IpcResponse};

/// IPC client for CLI use
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    /// Create a new IPC client
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Send a request and get a response
    pub async fn request(&self, request: &IpcRequest) -> Result<IpcResponse> {
        let stream = PlatformClient::connect(&self.socket_path).await?;

        let (reader, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(reader);

        // Send request
        let json = serde_json::to_string(request)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read response
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: IpcResponse = serde_json::from_str(&line)?;
        Ok(response)
    }

    /// Check if daemon is running
    pub async fn ping(&self) -> bool {
        matches!(
            self.request(&IpcRequest::Ping).await,
            Ok(IpcResponse::Pong { .. })
        )
    }
}
