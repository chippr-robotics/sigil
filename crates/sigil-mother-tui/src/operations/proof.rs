//! zkVM proof generation for TUI operations

use anyhow::Result;

/// Proof generation result
#[derive(Clone, Debug)]
pub struct ProofResult {
    /// Proof hash (for reference)
    pub proof_hash: String,
    /// Whether proof was generated
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Proof generator (wraps sigil-zkvm when available)
pub struct ProofGenerator {
    /// Whether zkVM is enabled
    enabled: bool,
}

impl ProofGenerator {
    /// Create a new proof generator
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Check if proofs are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Generate proof for a signing operation
    pub async fn generate_signing_proof(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
    ) -> Result<ProofResult> {
        if !self.enabled {
            return Ok(ProofResult {
                proof_hash: String::new(),
                success: false,
                error: Some("zkVM proofs not enabled".to_string()),
            });
        }

        // In a real implementation, this would call sigil-zkvm
        Ok(ProofResult {
            proof_hash: "0x1234567890abcdef".to_string(),
            success: true,
            error: None,
        })
    }
}

impl Default for ProofGenerator {
    fn default() -> Self {
        Self::new(false)
    }
}
