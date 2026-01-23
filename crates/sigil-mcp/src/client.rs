//! Daemon client abstraction layer
//!
//! Provides a unified interface for interacting with the Sigil daemon in both mock and real modes.

use crate::tools::DiskState;
use sigil_cli::client::{ClientError as CliClientError, SigilClient};

/// Daemon operation mode
pub enum DaemonMode {
    /// Mock mode - returns predefined data without connecting to daemon
    Mock(DiskState),
    /// Real mode - connects to actual daemon via IPC
    Real(SigilClient),
}

/// Abstraction layer for daemon communication
pub struct DaemonClient {
    mode: DaemonMode,
}

/// Result type for daemon client operations
pub type Result<T> = std::result::Result<T, ClientError>;

/// Client error types
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(String),

    #[error("No signing disk detected")]
    NoDiskDetected,

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Daemon error: {0}")]
    DaemonError(String),
}

impl From<CliClientError> for ClientError {
    fn from(e: CliClientError) -> Self {
        match e {
            CliClientError::DaemonNotRunning => ClientError::DaemonNotRunning,
            CliClientError::ConnectionFailed(msg) => ClientError::ConnectionFailed(msg),
            CliClientError::NoDiskDetected => ClientError::NoDiskDetected,
            CliClientError::SigningFailed(msg) => ClientError::SigningFailed(msg),
            CliClientError::RequestFailed(msg) => ClientError::RequestFailed(msg),
            CliClientError::DaemonError(msg) => ClientError::DaemonError(msg),
            CliClientError::Io(e) => ClientError::ConnectionFailed(e.to_string()),
            CliClientError::Serialization(e) => ClientError::RequestFailed(e.to_string()),
        }
    }
}

/// Result of a signing operation
#[derive(Debug, Clone)]
pub struct SignResult {
    pub signature: String,
    pub presig_index: u32,
    pub proof_hash: String,
}

impl DaemonClient {
    /// Create a new client in mock mode
    pub fn new_mock(state: DiskState) -> Self {
        Self {
            mode: DaemonMode::Mock(state),
        }
    }

    /// Create a new client that connects to the real daemon
    pub fn new_real() -> Result<Self> {
        let client = SigilClient::new();
        Ok(Self {
            mode: DaemonMode::Real(client),
        })
    }

    /// Check if in mock mode
    pub fn is_mock(&self) -> bool {
        matches!(self.mode, DaemonMode::Mock(_))
    }

    /// Get current disk status
    pub async fn get_disk_status(&self) -> Result<DiskState> {
        match &self.mode {
            DaemonMode::Mock(state) => Ok(state.clone()),
            DaemonMode::Real(client) => {
                let status = client.get_disk_status().await?;

                // Convert daemon's DiskStatus to MCP's DiskState
                Ok(DiskState {
                    detected: status.detected,
                    child_id: status.child_id,
                    scheme: None, // TODO: Add scheme to daemon's DiskStatus
                    presigs_remaining: status.presigs_remaining,
                    presigs_total: status.presigs_total,
                    days_until_expiry: status.days_until_expiry,
                    is_valid: status.is_valid,
                    public_key: None, // TODO: Add public_key to daemon's DiskStatus
                })
            }
        }
    }

    /// Sign a message hash
    pub async fn sign(
        &self,
        message_hash: &str,
        chain_id: u32,
        description: &str,
    ) -> Result<SignResult> {
        match &self.mode {
            DaemonMode::Mock(_) => {
                // Return mock signature
                Ok(SignResult {
                    signature: "0xaabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd11223344556677889900aabbccdd1122334400".to_string(),
                    presig_index: 0,
                    proof_hash: "0x1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff".to_string(),
                })
            }
            DaemonMode::Real(client) => {
                let result = client.sign(message_hash, chain_id, description).await?;

                Ok(SignResult {
                    signature: result.signature,
                    presig_index: result.presig_index,
                    proof_hash: result.proof_hash,
                })
            }
        }
    }

    /// Update transaction hash in audit log
    pub async fn update_tx_hash(&self, presig_index: u32, tx_hash: &str) -> Result<()> {
        match &self.mode {
            DaemonMode::Mock(_) => {
                // Mock mode - no-op
                Ok(())
            }
            DaemonMode::Real(client) => {
                client.update_tx_hash(presig_index, tx_hash).await?;
                Ok(())
            }
        }
    }

    /// Get presignature count
    pub async fn get_presig_count(&self) -> Result<(u32, u32)> {
        match &self.mode {
            DaemonMode::Mock(state) => {
                let remaining = state.presigs_remaining.unwrap_or(0);
                let total = state.presigs_total.unwrap_or(0);
                Ok((remaining, total))
            }
            DaemonMode::Real(client) => {
                let (remaining, total) = client.get_presig_count().await?;
                Ok((remaining, total))
            }
        }
    }
}
