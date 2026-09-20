//! Signing operations with zkVM proof generation

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use sigil_core::{
    accumulator::{NonMembershipWitness, StoredAccumulator},
    agent::AgentId,
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

    /// Stored accumulator for nullification verification
    accumulator: Arc<RwLock<Option<StoredAccumulator>>>,

    /// Non-membership witnesses for agents (keyed by agent_id hex)
    witnesses: Arc<RwLock<std::collections::HashMap<String, NonMembershipWitness>>>,

    /// Agent ID for this daemon (derived from agent's master shard)
    agent_id: Option<AgentId>,
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
            accumulator: Arc::new(RwLock::new(None)),
            witnesses: Arc::new(RwLock::new(std::collections::HashMap::new())),
            agent_id: None,
        }
    }

    /// Create a new signer with agent identity
    pub fn with_agent_id(
        agent_store: Arc<RwLock<AgentStore>>,
        disk_watcher: Arc<DiskWatcher>,
        enable_proving: bool,
        agent_id: AgentId,
    ) -> Self {
        Self {
            agent_store,
            disk_watcher,
            enable_proving,
            accumulator: Arc::new(RwLock::new(None)),
            witnesses: Arc::new(RwLock::new(std::collections::HashMap::new())),
            agent_id: Some(agent_id),
        }
    }

    /// Load accumulator from file
    ///
    /// The accumulator should be exported from the mother device and
    /// transferred via USB or other secure channel.
    pub async fn load_accumulator(&self, path: &std::path::Path) -> Result<()> {
        let bytes = std::fs::read(path)
            .map_err(|e| DaemonError::Store(format!("Failed to read accumulator: {}", e)))?;

        let stored = StoredAccumulator::from_bytes(&bytes)
            .ok_or_else(|| DaemonError::Store("Invalid accumulator format".to_string()))?;

        // Verify version is not going backwards (prevent rollback)
        {
            let current = self.accumulator.read().await;
            if let Some(ref curr) = *current {
                if stored.version() <= curr.version() {
                    return Err(DaemonError::Store(format!(
                        "Cannot load older accumulator (current: {}, new: {})",
                        curr.version(),
                        stored.version()
                    )));
                }
            }
        }

        info!("Loaded accumulator version {}", stored.version());
        *self.accumulator.write().await = Some(stored);

        Ok(())
    }

    /// Load non-membership witness for this agent
    pub async fn load_witness(&self, path: &std::path::Path) -> Result<()> {
        let bytes = std::fs::read(path)
            .map_err(|e| DaemonError::Store(format!("Failed to read witness: {}", e)))?;

        let witness = NonMembershipWitness::from_bytes(&bytes)
            .ok_or_else(|| DaemonError::Store("Invalid witness format".to_string()))?;

        let agent_id_hex = witness.agent_id.to_hex();
        info!("Loaded witness for agent {}", witness.agent_id.short());

        self.witnesses.write().await.insert(agent_id_hex, witness);

        Ok(())
    }

    /// Get the current accumulator version
    pub async fn accumulator_version(&self) -> Option<u64> {
        self.accumulator.read().await.as_ref().map(|a| a.version())
    }

    /// Get the agent ID configured for this signer
    pub fn agent_id(&self) -> Option<&AgentId> {
        self.agent_id.as_ref()
    }

    /// Check if an agent is verified as non-nullified
    pub async fn verify_agent_non_nullified(&self, agent_id: &AgentId) -> Result<bool> {
        let accumulator_guard = self.accumulator.read().await;
        let accumulator = accumulator_guard
            .as_ref()
            .ok_or_else(|| DaemonError::Store("No accumulator loaded".to_string()))?;

        let witnesses_guard = self.witnesses.read().await;
        let witness = witnesses_guard
            .get(&agent_id.to_hex())
            .ok_or_else(|| DaemonError::Store("No witness for agent".to_string()))?;

        // Verify witness version matches accumulator
        if witness.accumulator_version != accumulator.version() {
            return Err(DaemonError::Store(format!(
                "Witness version mismatch (witness: {}, accumulator: {})",
                witness.accumulator_version,
                accumulator.version()
            )));
        }

        // Verify non-membership (using accumulator's verify function)
        let is_valid =
            sigil_core::accumulator::verify_non_membership(&accumulator.accumulator, witness);

        Ok(is_valid)
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

        // 4. Get corresponding agent share, refusing one the agent side has
        //    already recorded as consumed.
        //
        //    The disk's own burn is defeated by restoring an earlier disk
        //    image: the restored disk offers a spent index, and without this
        //    check the agent half is served again. The same nonce k would then
        //    sign two different messages, and two signatures sharing r give
        //    k = (z1 - z2)/(s1 - s2), then d = (s1*k - z1)/r — full key
        //    disclosure from a file restore.
        //
        //    The high-water mark was already being maintained; this consults
        //    it. See specs/002-physical-consent-enforcement/ FR-014.
        let child_id = disk.header.child_id;
        let agent_share = {
            let mut store = self.agent_store.write().await;

            let next_expected = store.load_child(&child_id)?.next_presig_index;
            if presig_index < next_expected {
                warn!(
                    "Refusing presig {}: agent side expects {} or later. Disk may be a \
                     restored image.",
                    presig_index, next_expected
                );
                return Err(DaemonError::PresigAlreadyConsumed {
                    index: presig_index,
                    next_expected,
                });
            }

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
    //! Physical-consent enforcement.
    //!
    //! `Signer::sign` is where Sigil's central claim is either true or false:
    //! a signature exists only if a disk was physically present, and the
    //! presignature it consumed is burned so it cannot be spent twice. Until
    //! this module existed the function had no tests at all — see
    //! `specs/002-physical-consent-enforcement/`.
    //!
    //! The fixtures below produce well-formed but cryptographically
    //! meaningless shares. That is deliberate and sufficient: every property
    //! under test is about *orchestration* — what the signer refuses, what it
    //! consumes, what it persists — not about ECDSA being correct, which
    //! `sigil-core` covers.

    use super::*;
    use crate::agent_store::{AgentChildData, AgentStore};
    use crate::disk_watcher::DiskWatcher;
    use k256::elliptic_curve::sec1::ToEncodedPoint;
    use k256::{AffinePoint, ProjectivePoint, Scalar};
    use sigil_core::crypto::{DerivationPath, PublicKey};
    use sigil_core::disk::{DiskFormat, DiskHeader};
    use sigil_core::presig::PresigStatus;
    use sigil_core::presig::{PresigAgentShare, PresigColdShare};
    use sigil_core::types::{ChainId, ChildId, MessageHash};
    use std::path::PathBuf;

    /// A scratch directory unique to each test.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "sigil-signer-{}-{}-{:?}",
                name,
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch dir");
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The signer verifies its own output with `verify_prehash` before
    /// returning, so fixtures must be cryptographically consistent, not merely
    /// well-formed. For a presignature at index `i`:
    ///
    /// ```text
    /// d     = chi_cold + chi_agent      (the child private key)
    /// k     = k_cold   + k_agent        (the per-signature nonce)
    /// R     = k * G                     (the nonce commitment)
    /// pub   = d * G                     (what the disk header carries)
    /// ```
    ///
    /// which makes `s = k⁻¹(z + r·d)` a valid ECDSA signature under `pub`.
    fn scalar(value: u64) -> Scalar {
        Scalar::from(value.max(1))
    }

    fn compress(point: ProjectivePoint) -> [u8; 33] {
        let encoded = AffinePoint::from(point).to_encoded_point(true);
        let mut out = [0u8; 33];
        out.copy_from_slice(encoded.as_bytes());
        out
    }

    fn scalar_to_bytes(s: Scalar) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(&s.to_bytes());
        out
    }

    /// A valid compressed point that is *not* the R point of any presignature,
    /// for constructing deliberately mismatched shares.
    fn unrelated_point() -> [u8; 33] {
        compress(ProjectivePoint::GENERATOR * scalar(9_999))
    }

    /// The child private key these fixtures sign under.
    fn child_private_key() -> Scalar {
        scalar(0x5161_1CA1)
    }

    fn child_public_key() -> PublicKey {
        PublicKey::new(compress(ProjectivePoint::GENERATOR * child_private_key()))
    }

    /// Matched cold and agent share pairs, consistent with `child_public_key`.
    fn presig_pairs(count: u32) -> (Vec<PresigColdShare>, Vec<PresigAgentShare>) {
        let d = child_private_key();
        let mut cold = Vec::new();
        let mut agent = Vec::new();

        for i in 0..count {
            let i = u64::from(i);

            // Nonce, split across the two halves.
            let k_cold = scalar(i * 2 + 11);
            let k_agent = scalar(i * 2 + 12);
            let r_point = compress(ProjectivePoint::GENERATOR * (k_cold + k_agent));

            // Key share, split so the halves sum to the private key.
            let chi_cold = scalar(i + 101);
            let chi_agent = d - chi_cold;

            cold.push(PresigColdShare::new(
                r_point,
                scalar_to_bytes(k_cold),
                scalar_to_bytes(chi_cold),
            ));
            agent.push(PresigAgentShare::new(
                r_point,
                scalar_to_bytes(k_agent),
                scalar_to_bytes(chi_agent),
            ));
        }

        (cold, agent)
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    struct Fixture {
        signer: Signer,
        disk_watcher: Arc<DiskWatcher>,
        disk_path: PathBuf,
        child_id: ChildId,
        _scratch: Scratch,
    }

    /// Build a signer with `presig_count` matched presignature pairs, a disk
    /// file on disk, and an agent store holding the agent halves.
    async fn fixture(name: &str, presig_count: u32) -> Fixture {
        fixture_with(name, presig_count, |_, _| {}).await
    }

    /// As `fixture`, but `tamper` may mutate the shares before they are
    /// written, so tests can construct mismatched material.
    async fn fixture_with(
        name: &str,
        presig_count: u32,
        tamper: impl FnOnce(&mut Vec<PresigColdShare>, &mut Vec<PresigAgentShare>),
    ) -> Fixture {
        let scratch = Scratch::new(name);

        let (mut cold, mut agent) = presig_pairs(presig_count);
        tamper(&mut cold, &mut agent);

        let child_id = ChildId::new([7u8; 32]);
        let header = DiskHeader::new(
            child_id,
            child_public_key(),
            DerivationPath::new(&[44, 60, 0, 0, 1]).expect("valid derivation path"),
            presig_count,
            now(),
        );

        let disk = DiskFormat::new(header, cold);
        let disk_path = scratch.path("SIGIL.img");
        std::fs::write(&disk_path, disk.to_bytes()).expect("write disk image");

        let mut store = AgentStore::new(scratch.path("agent_store")).expect("agent store");
        store
            .store_child(AgentChildData::new(child_id, agent))
            .expect("store child");
        let agent_store = Arc::new(RwLock::new(store));

        let disk_watcher = Arc::new(DiskWatcher::new(
            scratch
                .path("never-matches-*")
                .to_string_lossy()
                .to_string(),
        ));
        disk_watcher
            .insert_disk_for_test(disk_path.clone())
            .await
            .expect("insert disk");

        let signer = Signer::new(
            Arc::clone(&agent_store),
            Arc::clone(&disk_watcher),
            false, // zkVM proving off: these tests are about consent, not proofs
        );

        Fixture {
            signer,
            disk_watcher,
            disk_path,
            child_id,
            _scratch: scratch,
        }
    }

    fn request(description: &str) -> SigningRequest {
        SigningRequest {
            message_hash: MessageHash::new([9u8; 32]),
            chain_id: ChainId::new(1),
            description: description.to_string(),
        }
    }

    fn reload(path: &PathBuf) -> DiskFormat {
        DiskFormat::from_bytes(&std::fs::read(path).expect("read disk")).expect("parse disk")
    }

    // ------------------------------------------------------------------
    // Fail closed: no disk, no signature
    // ------------------------------------------------------------------

    /// The whole product in one assertion.
    #[tokio::test]
    async fn refuses_to_sign_without_a_disk() {
        let f = fixture("no-disk", 4).await;
        f.disk_watcher.remove_disk_for_test().await;

        let result = f.signer.sign(request("no disk inserted")).await;

        assert!(
            matches!(result, Err(DaemonError::NoDiskDetected)),
            "signing without a physically present disk must fail closed, got {result:?}"
        );
    }

    /// Removal between operations must be caught, because `load_full_disk`
    /// re-reads the block device rather than trusting a cached copy.
    #[tokio::test]
    async fn refuses_to_sign_after_the_disk_is_removed_mid_session() {
        let f = fixture("removed-midway", 4).await;

        f.signer
            .sign(request("first signature"))
            .await
            .expect("a present disk should sign");

        // Physical removal: the file is gone.
        std::fs::remove_file(&f.disk_path).expect("remove disk image");

        let result = f.signer.sign(request("after removal")).await;
        assert!(
            result.is_err(),
            "signing must fail once the disk is no longer readable, got {result:?}"
        );
    }

    /// Presignatures are finite. Exhaustion is the bound on damage if a disk
    /// is stolen, so it must be enforced rather than wrapped around.
    #[tokio::test]
    async fn refuses_to_sign_once_presignatures_are_exhausted() {
        let count = 3;
        let f = fixture("exhausted", count).await;

        for i in 0..count {
            f.signer
                .sign(request(&format!("signature {i}")))
                .await
                .unwrap_or_else(|e| panic!("signature {i} should succeed: {e:?}"));
        }

        let result = f.signer.sign(request("one too many")).await;
        assert!(
            result.is_err(),
            "signing past the presignature supply must fail, got {result:?}"
        );

        assert_eq!(
            reload(&f.disk_path).header.presigs_remaining(),
            0,
            "an exhausted disk must report zero remaining"
        );
    }

    /// A cold share whose R point disagrees with its agent counterpart means
    /// the two halves are not from the same presignature. Completing anyway
    /// would produce a signature from mismatched material.
    #[tokio::test]
    async fn refuses_to_sign_when_cold_and_agent_shares_disagree() {
        let f = fixture_with("r-mismatch", 4, |_cold, agent| {
            // Give the agent half a different R point than the cold half.
            let chi_agent = agent[0].chi_agent;
            let k_agent = agent[0].k_agent;
            agent[0] = PresigAgentShare::new(unrelated_point(), k_agent, chi_agent);
        })
        .await;

        let result = f.signer.sign(request("mismatched shares")).await;

        assert!(
            matches!(result, Err(DaemonError::PresigMismatch(_))),
            "mismatched cold/agent shares must be rejected, got {result:?}"
        );

        assert_eq!(
            reload(&f.disk_path).header.presig_used,
            0,
            "a rejected signing attempt must not consume a presignature"
        );
    }

    // ------------------------------------------------------------------
    // Burn on use
    // ------------------------------------------------------------------

    /// A presignature is consumed by use and the burn is persisted to the
    /// disk, not merely held in memory.
    #[tokio::test]
    async fn a_successful_signature_burns_its_presignature_on_disk() {
        let f = fixture("burn", 5).await;

        assert_eq!(reload(&f.disk_path).header.presig_used, 0);

        let result = f
            .signer
            .sign(request("burn one"))
            .await
            .expect("should sign");

        let on_disk = reload(&f.disk_path);
        assert_eq!(
            on_disk.header.presig_used, 1,
            "the burn must be written to the disk, not just the in-memory copy"
        );
        assert_eq!(on_disk.header.presigs_remaining(), 4);
        assert_eq!(
            on_disk.presigs[result.presig_index as usize].status,
            PresigStatus::Used,
            "the specific presignature used must be marked Used"
        );
    }

    /// Each signature consumes a distinct presignature. A repeated index would
    /// mean a nonce reused across two signatures, which leaks the key.
    #[tokio::test]
    async fn every_signature_consumes_a_distinct_presignature() {
        let count = 5;
        let f = fixture("distinct", count).await;

        let mut seen = Vec::new();
        for i in 0..count {
            let result = f
                .signer
                .sign(request(&format!("signature {i}")))
                .await
                .expect("should sign");
            assert!(
                !seen.contains(&result.presig_index),
                "presignature index {} was returned twice; a reused ECDSA nonce \
                 discloses the private key",
                result.presig_index
            );
            seen.push(result.presig_index);
        }

        assert_eq!(seen.len(), count as usize);
        assert_eq!(reload(&f.disk_path).header.presig_used, count);
    }

    /// The usage log is the evidence reconciliation works from. One entry per
    /// signature, carrying the index that was spent.
    #[tokio::test]
    async fn each_signature_appends_one_usage_log_entry() {
        let f = fixture("usage-log", 4).await;

        let first = f.signer.sign(request("first")).await.expect("should sign");
        let second = f.signer.sign(request("second")).await.expect("should sign");

        let entries = reload(&f.disk_path).usage_log.entries;
        assert_eq!(entries.len(), 2, "one usage log entry per signature");
        assert_eq!(entries[0].presig_index, first.presig_index);
        assert_eq!(entries[1].presig_index, second.presig_index);
        assert_eq!(
            entries[0].description, "first",
            "the operator's description must be recorded for audit"
        );
    }

    /// The agent side records consumption as a high-water mark, and `sign`
    /// consults it.
    #[tokio::test]
    async fn the_agent_side_records_consumption() {
        let f = fixture("agent-burn", 4).await;

        let result = f
            .signer
            .sign(request("burn agent"))
            .await
            .expect("should sign");

        let mut store = f.signer.agent_store.write().await;
        let data = store.load_child(&f.child_id).expect("child data");
        assert!(
            data.next_presig_index > result.presig_index,
            "the agent side must record that index {} was consumed",
            result.presig_index
        );
    }

    // ------------------------------------------------------------------
    // FR-014: disk rollback detection
    // ------------------------------------------------------------------

    /// Restoring an earlier disk image must not re-spend a presignature.
    ///
    /// Without this guard the restored disk offers a spent index, the agent
    /// store serves the matching half, and the same nonce `k` signs two
    /// different messages. Two signatures sharing `r` give
    /// `k = (z1 - z2)/(s1 - s2)` and then `d = (s1*k - z1)/r` — the private
    /// key, from a file restore.
    #[tokio::test]
    async fn refuses_a_presignature_the_agent_side_has_already_consumed() {
        let f = fixture("rollback", 4).await;

        // Take a backup of the pristine disk, then spend two presignatures.
        let pristine = std::fs::read(&f.disk_path).expect("read disk");
        f.signer.sign(request("first")).await.expect("should sign");
        f.signer.sign(request("second")).await.expect("should sign");

        // Roll the disk back to before either signature.
        std::fs::write(&f.disk_path, &pristine).expect("restore disk image");
        assert_eq!(
            reload(&f.disk_path).header.presig_used,
            0,
            "the restored image should look unused, which is the whole problem"
        );

        let result = f.signer.sign(request("replay after rollback")).await;

        match result {
            Err(DaemonError::PresigAlreadyConsumed {
                index,
                next_expected,
            }) => {
                assert_eq!(index, 0, "the rolled-back disk offered index 0 again");
                assert_eq!(next_expected, 2, "the agent side had recorded two spends");
            }
            other => panic!(
                "a rolled-back disk must be refused, got {other:?}. Signing here \
                 would reuse an ECDSA nonce and disclose the private key."
            ),
        }
    }

    /// The guard must not fire in normal operation, where the disk and the
    /// agent side advance together.
    #[tokio::test]
    async fn the_rollback_guard_does_not_fire_during_normal_signing() {
        let count = 4;
        let f = fixture("no-false-positive", count).await;

        for i in 0..count {
            f.signer
                .sign(request(&format!("signature {i}")))
                .await
                .unwrap_or_else(|e| {
                    panic!("signature {i} must not be refused by the rollback guard: {e:?}")
                });
        }
    }

    /// Refill resets the agent-side mark, so a refilled disk signs from index
    /// zero again.
    ///
    /// Without the reset the guard would brick every refilled child: the new
    /// presignature table starts at index 0 while the mark still points past
    /// the end of the old one.
    #[tokio::test]
    async fn refill_resets_the_agent_mark_so_a_refilled_disk_signs_again() {
        let count = 3;
        let f = fixture("refill", count).await;

        // Spend the disk.
        for i in 0..count {
            f.signer
                .sign(request(&format!("signature {i}")))
                .await
                .expect("should sign");
        }
        assert!(
            f.signer.sign(request("exhausted")).await.is_err(),
            "the disk should now be spent"
        );

        // Refill: a fresh presignature table on the disk, and the matching
        // agent halves imported as a mother refill would deliver them.
        let (cold, agent) = presig_pairs(count);
        let header = DiskHeader::new(
            f.child_id,
            child_public_key(),
            DerivationPath::new(&[44, 60, 0, 0, 1]).expect("valid path"),
            count,
            now(),
        );
        let refilled = DiskFormat::new(header, cold);
        std::fs::write(&f.disk_path, refilled.to_bytes()).expect("write refilled disk");

        {
            let mut store = f.signer.agent_store.write().await;
            store
                .import_child_shares(AgentChildData::new(f.child_id, agent))
                .expect("import refilled agent shares");
        }

        // The refilled disk offers index 0 again, and that must now be allowed.
        let result = f
            .signer
            .sign(request("after refill"))
            .await
            .expect("a refilled disk must sign again");
        assert_eq!(result.presig_index, 0);
        assert_eq!(reload(&f.disk_path).header.presig_used, 1);
    }

    /// The reset is enforced at import rather than trusted from the payload.
    ///
    /// `ImportChildShares` deserializes `AgentChildData` straight from JSON,
    /// so a stale `next_presig_index` in that payload would otherwise carry
    /// into the new table and reject every signature.
    #[tokio::test]
    async fn importing_shares_resets_the_mark_regardless_of_the_payload() {
        let scratch = Scratch::new("import-reset");
        let mut store = AgentStore::new(scratch.path("agent_store")).expect("agent store");

        let (_, agent) = presig_pairs(4);
        let child_id = ChildId::new([7u8; 32]);

        let mut data = AgentChildData::new(child_id, agent);
        data.next_presig_index = 999; // as a stale or crafted payload might carry

        store.import_child_shares(data).expect("import");

        assert_eq!(
            store
                .load_child(&child_id)
                .expect("child")
                .next_presig_index,
            0,
            "import must reset the high-water mark rather than trust the payload"
        );
    }

    /// The signer re-reads the disk each time rather than trusting a cached
    /// copy, so state written by anything else is observed.
    #[tokio::test]
    async fn the_disk_is_re_read_on_every_signing_operation() {
        let f = fixture("re-read", 6).await;

        f.signer.sign(request("first")).await.expect("should sign");

        // Simulate the disk being advanced out-of-band, as reconciliation or
        // a second reader would: mark everything used and write it back.
        let mut disk = reload(&f.disk_path);
        for index in 0..disk.presigs.len() as u32 {
            let _ = disk.mark_presig_used(index);
        }
        std::fs::write(&f.disk_path, disk.to_bytes()).expect("rewrite disk");

        let result = f.signer.sign(request("after external burn")).await;
        assert!(
            result.is_err(),
            "the signer must observe the disk's current state, not a cached one, got {result:?}"
        );
    }
}
