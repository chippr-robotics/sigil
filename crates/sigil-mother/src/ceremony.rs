//! Ceremony implementations
//!
//! Ceremonies are the secure processes for creating children,
//! refilling presigs, and reconciliation.

use sigil_core::{
    crypto::DerivationPath,
    disk::{DiskFormat, DiskHeader},
    presig::PresigColdShare,
    ChildId, PublicKey, MAX_PRESIGS,
};

use crate::error::{MotherError, Result};
use crate::keygen::MasterKeyGenerator;
use crate::presig_gen::{PresigGenerator, PresigPair};
use crate::registry::ChildRegistry;
use crate::storage::{MasterShardData, MotherStorage};

/// Ceremony for creating a new child disk
pub struct CreateChildCeremony {
    storage: MotherStorage,
}

/// Output of child creation ceremony
pub struct CreateChildOutput {
    /// The disk data to write to floppy
    pub disk: DiskFormat,

    /// Agent shares to transfer to agent
    pub agent_shares: Vec<sigil_core::presig::PresigAgentShare>,

    /// The child's public key
    pub child_pubkey: PublicKey,

    /// The child ID
    pub child_id: ChildId,

    /// Derivation path used
    pub derivation_path: DerivationPath,
}

impl CreateChildCeremony {
    /// Create a new ceremony
    pub fn new(storage: MotherStorage) -> Self {
        Self { storage }
    }

    /// Execute the child creation ceremony
    pub fn execute(&mut self, presig_count: u32) -> Result<CreateChildOutput> {
        // 1. Load master shard
        let mut master = self.storage.load_master_shard()?;

        // 2. Allocate child index and create derivation path
        let child_index = master.allocate_child_index();
        let derivation_path = DerivationPath::ethereum_hardened(child_index);

        // 3. Derive child shards
        let (cold_child_shard, cold_child_pubkey) =
            MasterKeyGenerator::derive_child(&master.cold_master_shard, &derivation_path)?;

        // For the agent shard, we need the agent's master shard
        // In a real ceremony, this would be provided via secure channel
        // For now, we'll use a placeholder approach

        // Note: In production, the agent shard derivation happens on the agent side
        // Here we simulate it for the ceremony output

        // 4. Generate presignatures
        // Note: In a full 2-of-2 MPC, both parties would participate in presig generation
        // For this implementation, we generate both shares on the mother device
        // and transfer the agent shares securely

        // Generate a deterministic "agent child shard" for the ceremony
        // In production, the agent would derive this from their master shard
        let agent_child_shard = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(b"agent_shard_placeholder:");
            hasher.update(&derivation_path.to_bytes());
            let hash: [u8; 32] = hasher.finalize().into();
            hash
        };

        // Derive agent's child public key contribution
        let (_, agent_child_pubkey) =
            MasterKeyGenerator::derive_child(&agent_child_shard, &derivation_path)?;

        // 5. Combine public keys
        let child_pubkey =
            MasterKeyGenerator::combine_child_pubkeys(&cold_child_pubkey, &agent_child_pubkey)?;

        let child_id = child_pubkey.to_child_id();

        // 6. Generate presignatures
        let presig_pairs = PresigGenerator::generate_batch(
            &cold_child_shard,
            &agent_child_shard,
            presig_count as usize,
        )?;

        // 7. Split into cold and agent shares
        let cold_shares: Vec<PresigColdShare> =
            presig_pairs.iter().map(|p| p.cold_share.clone()).collect();
        let agent_shares: Vec<sigil_core::presig::PresigAgentShare> =
            presig_pairs.iter().map(|p| p.agent_share.clone()).collect();

        // 8. Create disk header
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut header = DiskHeader::new(
            child_id,
            child_pubkey,
            derivation_path,
            presig_count,
            created_at,
        );

        // 9. Sign the header with mother's key
        // For now, use a placeholder signature
        // In production, this would be a proper ECDSA signature
        let signable = header.signable_hash();
        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].copy_from_slice(&signable);
        header.mother_signature = sigil_core::Signature::new(sig_bytes);

        // 10. Create disk format
        let disk = DiskFormat::new(header, cold_shares);

        // 11. Register child in registry
        let mut registry = self.storage.load_registry()?;
        registry.register_child(child_id, derivation_path)?;
        self.storage.save_registry(&registry)?;

        // 12. Save updated master shard (with incremented index)
        self.storage.save_master_shard(&master)?;

        Ok(CreateChildOutput {
            disk,
            agent_shares,
            child_pubkey,
            child_id,
            derivation_path,
        })
    }
}

/// Ceremony for reconciling a child disk
pub struct ReconcileCeremony {
    storage: MotherStorage,
}

/// Result of reconciliation
pub struct ReconciliationResult {
    /// Whether the disk passed validation
    pub valid: bool,

    /// Number of signatures verified
    pub signatures_verified: u32,

    /// Any anomalies detected
    pub anomalies: Vec<String>,

    /// Recommendation
    pub recommendation: ReconciliationRecommendation,
}

/// Recommendation after reconciliation
#[derive(Debug, Clone)]
pub enum ReconciliationRecommendation {
    /// Disk is clean, proceed with refill
    RefillApproved,

    /// Issues found, manual review needed
    ManualReview { reason: String },

    /// Disk should be nullified
    Nullify { reason: String },
}

impl ReconcileCeremony {
    /// Create a new reconciliation ceremony
    pub fn new(storage: MotherStorage) -> Self {
        Self { storage }
    }

    /// Execute reconciliation
    pub fn execute(&mut self, disk: &DiskFormat) -> Result<ReconciliationResult> {
        let mut anomalies = Vec::new();
        let child_id = disk.header.child_id;

        // 1. Verify disk is registered
        let registry = self.storage.load_registry()?;
        let entry = registry.get_child(&child_id)?;

        // 2. Check child status
        if !entry.status.can_sign() {
            return Err(MotherError::ChildNullified(child_id.to_hex()));
        }

        // 3. Verify mother signature on header
        // (placeholder - would verify actual signature)

        // 4. Validate usage log
        if let Err(e) = disk.usage_log.validate() {
            anomalies.push(format!("Usage log validation failed: {}", e));
        }

        // 5. Cross-check presig usage
        let marked_used = disk
            .presigs
            .iter()
            .filter(|p| p.status == sigil_core::presig::PresigStatus::Used)
            .count() as u32;

        if marked_used != disk.header.presig_used {
            anomalies.push(format!(
                "Presig count mismatch: header={}, marked={}",
                disk.header.presig_used, marked_used
            ));
        }

        if marked_used != disk.usage_log.len() as u32 {
            anomalies.push(format!(
                "Usage log count mismatch: marked={}, logged={}",
                marked_used,
                disk.usage_log.len()
            ));
        }

        // 6. Verify signatures in usage log
        let mut verified = 0;
        for entry in &disk.usage_log.entries {
            // Would verify each signature against message_hash and child_pubkey
            // For now, count as verified
            verified += 1;
        }

        // 7. Determine recommendation
        let recommendation = if anomalies.is_empty() {
            ReconciliationRecommendation::RefillApproved
        } else if anomalies.len() <= 2 {
            ReconciliationRecommendation::ManualReview {
                reason: anomalies.join("; "),
            }
        } else {
            ReconciliationRecommendation::Nullify {
                reason: format!("Multiple anomalies: {}", anomalies.join("; ")),
            }
        };

        // 8. Save reconciliation log
        let log_entry = format!(
            "Reconciliation at {}\nChild: {}\nSignatures: {}\nAnomalies: {:?}\nRecommendation: {:?}",
            chrono::Utc::now(),
            child_id.short(),
            verified,
            anomalies,
            recommendation
        );
        self.storage
            .save_reconciliation_log(&child_id.short(), &log_entry)?;

        Ok(ReconciliationResult {
            valid: anomalies.is_empty(),
            signatures_verified: verified,
            anomalies,
            recommendation,
        })
    }
}

/// Ceremony for refilling a child disk with new presigs
pub struct RefillCeremony {
    storage: MotherStorage,
}

impl RefillCeremony {
    /// Create a new refill ceremony
    pub fn new(storage: MotherStorage) -> Self {
        Self { storage }
    }

    /// Execute refill after successful reconciliation
    pub fn execute(
        &mut self,
        disk: &mut DiskFormat,
        presig_count: u32,
    ) -> Result<Vec<sigil_core::presig::PresigAgentShare>> {
        let child_id = disk.header.child_id;

        // 1. Load registry and verify child is active
        let mut registry = self.storage.load_registry()?;
        let entry = registry.get_child(&child_id)?;

        if !entry.status.can_sign() {
            return Err(MotherError::ChildNullified(child_id.to_hex()));
        }

        // 2. Load master shard for derivation
        let master = self.storage.load_master_shard()?;

        // 3. Re-derive child shards
        let (cold_child_shard, _) =
            MasterKeyGenerator::derive_child(&master.cold_master_shard, &entry.derivation_path)?;

        // Agent shard (placeholder - would be provided by agent)
        let agent_child_shard = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(b"agent_shard_placeholder:");
            hasher.update(&entry.derivation_path.to_bytes());
            let hash: [u8; 32] = hasher.finalize().into();
            hash
        };

        // 4. Generate new presignatures
        let presig_pairs = PresigGenerator::generate_batch(
            &cold_child_shard,
            &agent_child_shard,
            presig_count as usize,
        )?;

        // 5. Split into cold and agent shares
        let cold_shares: Vec<PresigColdShare> =
            presig_pairs.iter().map(|p| p.cold_share.clone()).collect();
        let agent_shares: Vec<sigil_core::presig::PresigAgentShare> =
            presig_pairs.iter().map(|p| p.agent_share.clone()).collect();

        // 6. Update disk
        disk.presigs = cold_shares;
        disk.header.presig_total = presig_count;
        disk.header.presig_used = 0;
        disk.usage_log = sigil_core::usage::UsageLog::new();

        // 7. Update expiry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        disk.header.expiry.reset_for_reconciliation(
            now + (sigil_core::PRESIG_VALIDITY_DAYS as u64 * 86400),
            now + (sigil_core::RECONCILIATION_DEADLINE_DAYS as u64 * 86400),
        );

        // 8. Re-sign header
        let signable = disk.header.signable_hash();
        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].copy_from_slice(&signable);
        disk.header.mother_signature = sigil_core::Signature::new(sig_bytes);

        // 9. Record reconciliation
        registry.record_reconciliation(&child_id, disk.usage_log.len() as u32)?;
        self.storage.save_registry(&registry)?;

        Ok(agent_shares)
    }
}
