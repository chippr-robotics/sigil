//! Client for communicating with the Sigil daemon

use std::path::PathBuf;

use sigil_daemon::ipc::{IpcClient, IpcRequest, IpcResponse};

/// Default Unix socket path, matching `DaemonConfig::default_ipc_path`'s
/// fallback. Never `/tmp`: a world-writable directory lets any local user
/// squat the path the CLI connects to.
#[cfg(unix)]
pub const DEFAULT_UNIX_SOCKET_PATH: &str = "/run/sigil/sigil.sock";

/// Client for the Sigil daemon
pub struct SigilClient {
    inner: IpcClient,
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

    #[error("Daemon error: {0}")]
    DaemonError(String),
}

impl ClientError {
    fn from_daemon_error(e: sigil_daemon::error::DaemonError) -> Self {
        match &e {
            sigil_daemon::error::DaemonError::Ipc(msg) if msg.contains("not running") => {
                ClientError::DaemonNotRunning
            }
            _ => ClientError::DaemonError(e.to_string()),
        }
    }
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
    /// Compressed secp256k1 child public key, hex without `0x`.
    pub child_pubkey: Option<String>,
}

/// Result of a signing operation
#[derive(Debug, Clone)]
pub struct SignResult {
    pub signature: String,
    pub presig_index: u32,
    pub proof_hash: String,
}

impl SigilClient {
    /// Create a new client with the default socket path.
    ///
    /// This must resolve the same way `DaemonConfig::default_ipc_path` does,
    /// or the CLI cannot reach a default-configured daemon. It also must not
    /// fall back to `/tmp`: that directory is world-writable, so any local
    /// user could create a socket there and receive the operator's signing
    /// requests. See Constitution Principle VII.
    pub fn new() -> Self {
        #[cfg(unix)]
        let socket_path = std::env::var_os("XDG_RUNTIME_DIR")
            .map(|dir| PathBuf::from(dir).join("sigil.sock"))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_UNIX_SOCKET_PATH));

        #[cfg(windows)]
        let socket_path = PathBuf::from(r"\\.\pipe\sigil");

        Self {
            inner: IpcClient::new(socket_path),
        }
    }

    /// Create a new client with a custom socket path
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self {
            inner: IpcClient::new(socket_path),
        }
    }

    /// Check if the daemon is running
    pub async fn ping(&self) -> Result<String> {
        match self.inner.request(&IpcRequest::Ping).await {
            Ok(IpcResponse::Pong { version }) => Ok(version),
            Ok(IpcResponse::Error { message }) => Err(ClientError::RequestFailed(message)),
            Ok(_) => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
            Err(e) => Err(ClientError::from_daemon_error(e)),
        }
    }

    /// Get the current disk status
    pub async fn get_disk_status(&self) -> Result<DiskStatus> {
        match self
            .inner
            .request(&IpcRequest::GetDiskStatus)
            .await
            .map_err(ClientError::from_daemon_error)?
        {
            IpcResponse::DiskStatus {
                detected,
                child_id,
                presigs_remaining,
                presigs_total,
                days_until_expiry,
                is_valid,
                child_pubkey,
            } => Ok(DiskStatus {
                detected,
                child_id,
                presigs_remaining,
                presigs_total,
                days_until_expiry,
                is_valid,
                child_pubkey,
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

        match self
            .inner
            .request(&request)
            .await
            .map_err(ClientError::from_daemon_error)?
        {
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

        match self
            .inner
            .request(&request)
            .await
            .map_err(ClientError::from_daemon_error)?
        {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// Get the presig count
    pub async fn get_presig_count(&self) -> Result<(u32, u32)> {
        match self
            .inner
            .request(&IpcRequest::GetPresigCount)
            .await
            .map_err(ClientError::from_daemon_error)?
        {
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

        match self
            .inner
            .request(&request)
            .await
            .map_err(ClientError::from_daemon_error)?
        {
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

        match self
            .inner
            .request(&request)
            .await
            .map_err(ClientError::from_daemon_error)?
        {
            IpcResponse::Ok => Ok(()),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }

    /// List imported children
    pub async fn list_children(&self) -> Result<Vec<String>> {
        match self
            .inner
            .request(&IpcRequest::ListChildren)
            .await
            .map_err(ClientError::from_daemon_error)?
        {
            IpcResponse::Children { child_ids } => Ok(child_ids),
            IpcResponse::Error { message } => Err(ClientError::RequestFailed(message)),
            _ => Err(ClientError::RequestFailed(
                "Unexpected response".to_string(),
            )),
        }
    }
}

impl Default for SigilClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// The CLI must look for the daemon where the daemon actually listens.
    ///
    /// These drifted apart: #57 moved the daemon's fallback off world-writable
    /// `/tmp` to `/run/sigil`, and the CLI was missed — so the CLI could not
    /// reach a default-configured daemon at all, and would have connected to
    /// whatever any local user had created at `/tmp/sigil.sock`.
    #[test]
    fn the_default_socket_path_is_not_world_writable() {
        assert!(
            !DEFAULT_UNIX_SOCKET_PATH.starts_with("/tmp"),
            "the CLI must not default to a socket in a world-writable directory, got {DEFAULT_UNIX_SOCKET_PATH}"
        );
    }

    /// Pinned deliberately: this constant and `DaemonConfig::default_ipc_path`
    /// are two halves of one decision, in two crates that cannot see each
    /// other. If the daemon moves again, this fails and names the reason.
    #[test]
    fn the_default_socket_path_matches_the_daemons() {
        assert_eq!(
            DEFAULT_UNIX_SOCKET_PATH, "/run/sigil/sigil.sock",
            "must match DaemonConfig::default_ipc_path's fallback in sigil-daemon"
        );
    }

    #[test]
    fn an_explicit_socket_path_is_honoured() {
        let path = PathBuf::from("/run/sigil/custom.sock");
        let _client = SigilClient::with_socket_path(path);
        // Construction must not panic or rewrite the caller's choice; the
        // connection itself is exercised below.
    }

    /// The operator's signing path fails closed when the daemon is absent —
    /// it does not hang, and it does not report success.
    #[tokio::test]
    async fn a_missing_daemon_is_an_error_not_a_hang() {
        let client = SigilClient::with_socket_path(PathBuf::from(
            "/nonexistent/sigil-cli-test/definitely-not-here.sock",
        ));

        let result =
            tokio::time::timeout(std::time::Duration::from_secs(5), client.get_disk_status()).await;

        let inner = result.expect("connecting to an absent daemon must not hang");
        assert!(
            inner.is_err(),
            "an absent daemon must be an error, got {inner:?}"
        );
    }

    /// Every operation, not just status, must fail closed without a daemon.
    #[tokio::test]
    async fn every_operation_fails_closed_without_a_daemon() {
        let client = SigilClient::with_socket_path(PathBuf::from(
            "/nonexistent/sigil-cli-test/definitely-not-here.sock",
        ));

        assert!(client.ping().await.is_err(), "ping");
        assert!(
            client.sign("0xabc", 1, "test").await.is_err(),
            "sign must never succeed without a daemon"
        );
        assert!(client.get_presig_count().await.is_err(), "presig count");
        assert!(client.list_children().await.is_err(), "list children");
        assert!(
            client.update_tx_hash(0, "0xabc").await.is_err(),
            "update tx hash"
        );
    }
}
