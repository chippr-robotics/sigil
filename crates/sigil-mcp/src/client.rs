//! Daemon client abstraction layer
//!
//! Provides a unified interface for interacting with the Sigil daemon in both
//! mock and real modes.
//!
//! # Mock mode never signs
//!
//! Mock mode exists so agent clients can be harnessed without a physical disk.
//! It fabricates *status* — disk present, N presignatures remaining — because
//! status is not consent. It does **not** fabricate signatures: signing in mock
//! mode returns [`ClientError::MockSigningDisabled`].
//!
//! A mock that returned plausible signature bytes on the same response path as
//! a real signature would make Sigil's central claim untestable by its own
//! callers. See `.specify/memory/constitution.md`, Principle II.
//!
//! The whole mock surface is gated behind the non-default `mock` cargo feature
//! (plus `cfg(test)` for this crate's own tests), so a default-feature release
//! binary cannot construct a mock signer at all.

use crate::tools::DiskState;
use sigil_cli::client::{ClientError as CliClientError, SigilClient};

/// Daemon operation mode
pub enum DaemonMode {
    /// Mock mode - returns predefined *status* without connecting to a daemon.
    /// Signing in this mode is an error, never a fabricated signature.
    #[cfg(any(test, feature = "mock"))]
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

    #[error(
        "Signing is disabled in mock mode. Sigil never fabricates a signature: \
         a signature requires a presignature share read from a physically \
         inserted disk. Run against a real daemon with a disk inserted."
    )]
    MockSigningDisabled,
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
    /// Create a new client in mock mode.
    ///
    /// Mock clients report fabricated disk status and refuse to sign.
    #[cfg(any(test, feature = "mock"))]
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
        #[cfg(any(test, feature = "mock"))]
        {
            matches!(self.mode, DaemonMode::Mock(_))
        }
        #[cfg(not(any(test, feature = "mock")))]
        {
            false
        }
    }

    /// Get current disk status
    pub async fn get_disk_status(&self) -> Result<DiskState> {
        match &self.mode {
            #[cfg(any(test, feature = "mock"))]
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
            // Deliberately an error, not a plausible-looking signature. See the
            // module docs: mock status is fine, a mock signature is a lie.
            #[cfg(any(test, feature = "mock"))]
            DaemonMode::Mock(_) => Err(ClientError::MockSigningDisabled),
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
            #[cfg(any(test, feature = "mock"))]
            DaemonMode::Mock(_) => {
                // Mock mode - no-op. Recording a tx hash is bookkeeping, not
                // consent, so there is nothing to fabricate here.
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
            #[cfg(any(test, feature = "mock"))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_mode_refuses_to_sign() {
        let client = DaemonClient::new_mock(DiskState::mock_detected());

        let result = client.sign("0xdeadbeef", 1, "test transfer").await;

        let err = result.expect_err("mock mode must never return a signature");
        assert!(
            matches!(err, ClientError::MockSigningDisabled),
            "expected MockSigningDisabled, got {err:?}"
        );
    }

    #[tokio::test]
    async fn mock_signing_error_mentions_physical_consent() {
        let message = ClientError::MockSigningDisabled.to_string();
        assert!(
            message.contains("physically inserted disk"),
            "the error must explain why, got: {message}"
        );
    }

    #[tokio::test]
    async fn mock_mode_still_reports_status() {
        let client = DaemonClient::new_mock(DiskState::mock_detected());

        // Status is not consent: fabricating it is allowed and useful.
        let status = client
            .get_disk_status()
            .await
            .expect("mock status must still work");
        assert!(status.detected);

        client
            .get_presig_count()
            .await
            .expect("mock presig count must still work");
    }

    #[tokio::test]
    async fn mock_mode_is_reported_as_mock() {
        let client = DaemonClient::new_mock(DiskState::mock_detected());
        assert!(client.is_mock());
    }
}
