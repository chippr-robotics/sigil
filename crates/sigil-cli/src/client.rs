//! Client for communicating with the Sigil daemon

use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use sigil_daemon::ipc::{IpcRequest, IpcResponse};

/// Client for the Sigil daemon
pub struct SigilClient {
    socket_path: PathBuf,
}

/// Error type for client operations
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(String),

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("No signing disk detected")]
    NoDiskDetected,

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// Status of the signing disk
#[derive(Debug, Clone)]
pub struct DiskStatus {
    pub detected: bool,
    pub child_id: Option<String>,
    pub presigs_remaining: Option<u32>,
    pub presigs_total: Option<u32>,
    pub days_until_expiry: Option<u32>,
    pub is_valid: Option<bool>,
}

/// Result of a signing operation
#[derive(Debug, Clone)]
pub struct SignResult {
    pub signature: String,
    pub presig_index: u32,
    pub proof_hash: String,
}

impl SigilClient {
    /// Create a new client with the default socket path
    pub fn new() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/sigil.sock"),
        }
    }

    /// Create a new client with a custom socket path
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Check if the daemon is running
    pub async fn ping(&self) -> Result<String> {
        match self.request(&IpcRequest::Ping).await? {
            IpcResponse::Pong { version } => Ok(version),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Get the current disk status
    pub async fn get_disk_status(&self) -> Result<DiskStatus> {
        match self.request(&IpcRequest::GetDiskStatus).await? {
            IpcResponse::DiskStatus {
                detected,
                child_id,
                presigs_remaining,
                presigs_total,
                days_until_expiry,
                is_valid,
            } => Ok(DiskStatus {
                detected,
                child_id,
                presigs_remaining,
                presigs_total,
                days_until_expiry,
                is_valid,
            }),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Sign a message hash
    pub async fn sign(
        &self,
        message_hash: &str,
        chain_id: u32,
        description: &str,
    ) -> Result<SignResult> {
        let request = IpcRequest::Sign {
            message_hash: message_hash.to_string(),
            chain_id,
            description: description.to_string(),
        };

        match self.request(&request).await? {
            IpcResponse::SignResult {
                signature,
                presig_index,
                proof_hash,
            } => Ok(SignResult {
                signature,
                presig_index,
                proof_hash,
            }),
            IpcResponse::Error { message } => Err(ClientError::SigningFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Update transaction hash after broadcast
    pub async fn update_tx_hash(&self, presig_index: u32, tx_hash: &str) -> Result<()> {
        let request = IpcRequest::UpdateTxHash {
            presig_index,
            tx_hash: tx_hash.to_string(),
        };

        match self.request(&request).await? {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Get the presig count
    pub async fn get_presig_count(&self) -> Result<(u32, u32)> {
        match self.request(&IpcRequest::GetPresigCount).await? {
            IpcResponse::PresigCount { remaining, total } => Ok((remaining, total)),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Import agent master shard
    pub async fn import_agent_shard(&self, agent_shard_hex: &str) -> Result<()> {
        let request = IpcRequest::ImportAgentShard {
            agent_shard_hex: agent_shard_hex.to_string(),
        };

        match self.request(&request).await? {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Import child presignature shares
    pub async fn import_child_shares(&self, shares_json: &str, replace: bool) -> Result<()> {
        let request = IpcRequest::ImportChildShares {
            shares_json: shares_json.to_string(),
            replace,
        };

        match self.request(&request).await? {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// List imported children
    pub async fn list_children(&self) -> Result<Vec<String>> {
        match self.request(&IpcRequest::ListChildren).await? {
            IpcResponse::Children { child_ids } => Ok(child_ids),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Send a request to the daemon
    async fn request(&self, request: &IpcRequest) -> Result<IpcResponse> {
        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound
                || e.kind() == std::io::ErrorKind::ConnectionRefused
            {
                ClientError::DaemonNotRunning
            } else {
                ClientError::ConnectionFailed(e.to_string())
            }
        })?;

        let (reader, mut writer) = stream.into_split();
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
}

impl Default for SigilClient {
    fn default() -> Self {
        Self::new()
    }
}
