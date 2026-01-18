//! Signing operations with zkVM proof generation

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use sigil_core::{
    presig::PresigAgentShare,
    types::{ChainId, MessageHash, Signature, TxHash, ZkProofHash},
    usage::UsageLogEntry,
};

use crate::agent_store::AgentStore;
use crate::disk_watcher::DiskWatcher;
use crate::error::{DaemonError, Result};

/// Signer handles MPC signature completion
pub struct Signer {
    /// Agent shard store
    agent_store: Arc<RwLock<AgentStore>>,

    /// Disk watcher
    disk_watcher: Arc<DiskWatcher>,

    /// Whether to enable zkVM proving
    enable_proving: bool,
}

/// Result of a signing operation
#[derive(Debug, Clone)]
pub struct SigningResult {
    /// The produced signature
    pub signature: Signature,

    /// Index of the presig used
    pub presig_index: u32,

    /// Hash of the zkVM proof
    pub proof_hash: ZkProofHash,

    /// The message that was signed
    pub message_hash: MessageHash,
}

/// Request for a signing operation
#[derive(Debug, Clone)]
pub struct SigningRequest {
    /// Message hash to sign
    pub message_hash: MessageHash,

    /// Chain ID for logging
    pub chain_id: ChainId,

    /// Human-readable description
    pub description: String,
}

impl Signer {
    /// Create a new signer
    pub fn new(
        agent_store: Arc<RwLock<AgentStore>>,
        disk_watcher: Arc<DiskWatcher>,
        enable_proving: bool,
    ) -> Self {
        Self {
            agent_store,
            disk_watcher,
            enable_proving,
        }
    }

    /// Sign a message
    pub async fn sign(&self, request: SigningRequest) -> Result<SigningResult> {
        info!("Starting signing operation");

        // 1. Load disk
        let mut disk = self.disk_watcher.load_full_disk().await?;

        // 2. Validate disk
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        disk.validate(current_time)?;

        // 3. Get next available presig from disk
        let (presig_index, cold_share) = disk.get_next_presig()?;
        debug!("Using presig index: {}", presig_index);

        // 4. Get corresponding agent share
        let child_id = disk.header.child_id;
        let agent_share = {
            let mut store = self.agent_store.write().await;
            store.get_presig_share(&child_id, presig_index)?.clone()
        };

        // 5. Verify R points match
        if cold_share.r_point != agent_share.r_point {
            return Err(DaemonError::PresigMismatch(format!(
                "R point mismatch at index {}",
                presig_index
            )));
        }

        // 6. Complete the signature
        let (signature, proof_hash) = self
            .complete_signature(
                &disk.header.child_pubkey,
                &request.message_hash,
                presig_index,
                cold_share,
                &agent_share,
            )
            .await?;

        // 7. Mark presig as used on disk
        disk.mark_presig_used(presig_index)?;

        // 8. Create usage log entry
        // Note: tx_hash would be populated after broadcast
        let log_entry = UsageLogEntry::new(
            presig_index,
            current_time,
            request.message_hash,
            signature,
            request.chain_id,
            TxHash::new([0u8; 32]), // Placeholder until broadcast
            proof_hash,
            request.description,
        );

        disk.usage_log.push(log_entry)?;

        // 9. Write updated disk
        self.disk_watcher.write_disk(&disk).await?;

        // 10. Mark agent presig as used
        {
            let mut store = self.agent_store.write().await;
            store.mark_presig_used(&child_id, presig_index)?;
        }

        info!("Signing complete, presig index: {}", presig_index);

        Ok(SigningResult {
            signature,
            presig_index,
            proof_hash,
            message_hash: request.message_hash,
        })
    }

    /// Complete the ECDSA signature from presig shares
    async fn complete_signature(
        &self,
        pubkey: &sigil_core::PublicKey,
        message_hash: &MessageHash,
        presig_index: u32,
        cold_share: &sigil_core::presig::PresigColdShare,
        agent_share: &PresigAgentShare,
    ) -> Result<(Signature, ZkProofHash)> {
        use k256::{
            elliptic_curve::{
                ops::Reduce,
                sec1::{FromEncodedPoint, ToEncodedPoint},
                PrimeField,
            },
            AffinePoint, Scalar, U256,
        };

        // Decode R point
        let r_encoded = k256::EncodedPoint::from_bytes(cold_share.r_point)
            .map_err(|e| DaemonError::SigningFailed(format!("Invalid R point: {}", e)))?;

        let r_affine = AffinePoint::from_encoded_point(&r_encoded);
        if r_affine.is_none().into() {
            return Err(DaemonError::SigningFailed(
                "Invalid R curve point".to_string(),
            ));
        }
        let r_affine = r_affine.unwrap();

        // Get r = x-coordinate of R (mod n)
        let r_x = r_affine.to_encoded_point(false);
        let r_x_bytes = r_x.x().ok_or_else(|| {
            DaemonError::SigningFailed("Failed to get R x-coordinate".to_string())
        })?;

        let r = <Scalar as Reduce<U256>>::reduce_bytes(r_x_bytes);

        // Combine nonce shares: k = k_cold + k_agent
        let k_cold = Scalar::from_repr(cold_share.k_cold.into());
        let k_agent = Scalar::from_repr(agent_share.k_agent.into());

        if k_cold.is_none().into() || k_agent.is_none().into() {
            return Err(DaemonError::SigningFailed(
                "Invalid nonce share".to_string(),
            ));
        }

        let k = k_cold.unwrap() + k_agent.unwrap();
        let k_inv = k.invert();
        if k_inv.is_none().into() {
            return Err(DaemonError::SigningFailed("Nonce is zero".to_string()));
        }
        let k_inv = k_inv.unwrap();

        // Decode message hash
        let z = <Scalar as Reduce<U256>>::reduce_bytes(message_hash.as_bytes().into());

        // Combine chi values
        let chi_cold = Scalar::from_repr(cold_share.chi_cold.into());
        let chi_agent = Scalar::from_repr(agent_share.chi_agent.into());

        if chi_cold.is_none().into() || chi_agent.is_none().into() {
            return Err(DaemonError::SigningFailed("Invalid chi share".to_string()));
        }

        let chi = chi_cold.unwrap() + chi_agent.unwrap();

        // Compute s = k_inv * (z + r * chi)
        let s = k_inv * (z + r * chi);

        // Normalize s to low-S form (BIP-62)
        let s = normalize_s_low(s);

        // Encode signature
        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].copy_from_slice(&r.to_bytes());
        sig_bytes[32..].copy_from_slice(&s.to_bytes());

        let signature = Signature::new(sig_bytes);

        // Verify signature using prehash (message is already hashed)
        let verifying_key = k256::ecdsa::VerifyingKey::from_sec1_bytes(pubkey.as_bytes())
            .map_err(|e| DaemonError::SigningFailed(format!("Invalid public key: {}", e)))?;

        let ecdsa_sig = k256::ecdsa::Signature::from_slice(&sig_bytes)
            .map_err(|e| DaemonError::SigningFailed(format!("Invalid signature format: {}", e)))?;

        // Use verify_prehash since message_hash is already the Keccak256 digest
        use k256::ecdsa::signature::hazmat::PrehashVerifier;
        verifying_key
            .verify_prehash(message_hash.as_bytes(), &ecdsa_sig)
            .map_err(|_| DaemonError::SigningFailed("Signature verification failed".to_string()))?;

        // Generate proof hash
        let proof_hash = if self.enable_proving {
            self.generate_zkvm_proof(
                pubkey,
                message_hash,
                presig_index,
                cold_share,
                agent_share,
                &signature,
            )
            .await?
        } else {
            // In non-proving mode, just hash the signature as placeholder
            let hash = sigil_core::crypto::sha256(&sig_bytes);
            ZkProofHash::new(hash)
        };

        Ok((signature, proof_hash))
    }

    /// Generate a zkVM proof of the signing operation
    async fn generate_zkvm_proof(
        &self,
        pubkey: &sigil_core::PublicKey,
        message_hash: &MessageHash,
        presig_index: u32,
        cold_share: &sigil_core::presig::PresigColdShare,
        _agent_share: &PresigAgentShare,
        signature: &Signature,
    ) -> Result<ZkProofHash> {
        // In a full implementation, this would:
        // 1. Prepare inputs for the SP1 program
        // 2. Execute the program in the zkVM
        // 3. Generate and verify the proof
        // 4. Return the proof hash

        // For now, we create a deterministic hash that could be verified
        // against the proof when available
        let proof_input = sigil_core::crypto::sha256_multi(&[
            pubkey.as_bytes(),
            message_hash.as_bytes(),
            &presig_index.to_le_bytes(),
            &cold_share.r_point,
            signature.as_bytes(),
        ]);

        debug!("Generated proof hash (proving disabled or placeholder)");

        Ok(ZkProofHash::new(proof_input))
    }

    /// Update transaction hash in usage log after broadcast
    #[allow(dead_code)]
    pub async fn update_tx_hash(&self, presig_index: u32, tx_hash: TxHash) -> Result<()> {
        let mut disk = self.disk_watcher.load_full_disk().await?;

        // Find the log entry and update it
        for entry in &mut disk.usage_log.entries {
            if entry.presig_index == presig_index {
                entry.tx_hash = tx_hash;
                break;
            }
        }

        self.disk_watcher.write_disk(&disk).await?;

        Ok(())
    }
}

/// Normalize s to low-S form per BIP-62
fn normalize_s_low(s: k256::Scalar) -> k256::Scalar {
    // secp256k1 order / 2 (big-endian)
    const HALF_ORDER: [u8; 32] = [
        0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0x5D, 0x57, 0x6E, 0x73, 0x57, 0xA4, 0x50, 0x1D, 0xDF, 0xE9, 0x2F, 0x46, 0x68, 0x1B,
        0x20, 0xA0,
    ];

    let s_bytes: [u8; 32] = s.to_bytes().into();

    // Compare s > half_order using byte comparison
    let is_high = scalar_gt_bytes(&s_bytes, &HALF_ORDER);

    if is_high {
        -s
    } else {
        s
    }
}

/// Compare if a > b (big-endian byte arrays)
fn scalar_gt_bytes(a: &[u8], b: &[u8; 32]) -> bool {
    let mut gt = false;
    let mut eq = true;

    for i in 0..32 {
        if eq {
            if a[i] > b[i] {
                gt = true;
                eq = false;
            } else if a[i] < b[i] {
                gt = false;
                eq = false;
            }
        }
    }

    gt
}

#[cfg(test)]
mod tests {
    // Integration tests would require full setup with disk and agent store
}
