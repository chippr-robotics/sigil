//! End-to-end workflow tests for the Sigil system
//!
//! These tests verify the complete workflow from key generation through
//! disk creation, signing, and reconciliation.

use sigil_core::{
    crypto::{DerivationPath, PublicKey},
    disk::{DiskFormat, DiskHeader},
    expiry::DiskExpiry,
    presig::{PresigAgentShare, PresigColdShare},
    types::{ChainId, ChildId, MessageHash, Signature, TxHash, ZkProofHash},
    usage::{UsageLog, UsageLogEntry},
    ChildStatus, NullificationReason,
};

use sigil_mother::{
    keygen::MasterKeyGenerator,
    presig_gen::PresigGenerator,
    reconciliation::{analyze_disk, generate_report},
    registry::ChildRegistry,
};

/// Simulates the complete lifecycle of a child disk
#[test]
fn test_full_disk_lifecycle() {
    // ==========================================
    // STEP 1: Initialize mother device
    // ==========================================
    let master_output = MasterKeyGenerator::generate().unwrap();
    let mut registry = ChildRegistry::new();

    // ==========================================
    // STEP 2: Create a child disk
    // ==========================================
    let child_index = 0u32;
    let derivation_path = DerivationPath::ethereum_hardened(child_index);

    // In production, child shards would be derived from master shards
    // For testing, we simulate this
    let cold_child_shard = [0x11; 32];
    let agent_child_shard = [0x22; 32];

    // Generate presignatures
    let presig_count = 100;
    let presig_pairs =
        PresigGenerator::generate_batch(&cold_child_shard, &agent_child_shard, presig_count)
            .unwrap();

    // Split presigs into cold and agent shares
    let cold_shares: Vec<PresigColdShare> =
        presig_pairs.iter().map(|p| p.cold_share.clone()).collect();
    let agent_shares: Vec<PresigAgentShare> =
        presig_pairs.iter().map(|p| p.agent_share.clone()).collect();

    // Create child public key (simulated - in production this combines the shard pubkeys)
    let child_pubkey = PublicKey::new([0x02; 33]);
    let child_id = child_pubkey.to_child_id();

    // Create disk
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let header = DiskHeader::new(
        child_id,
        child_pubkey,
        derivation_path,
        presig_count as u32,
        current_time,
    );

    let mut disk = DiskFormat::new(header, cold_shares);

    // Register child in mother's registry
    registry.register_child(child_id, derivation_path).unwrap();

    // ==========================================
    // STEP 3: Simulate signing operations
    // ==========================================
    let num_signatures = 10;

    for i in 0..num_signatures {
        // Get next available presig
        let (presig_index, cold_share) = disk.get_next_presig().unwrap();
        let agent_share = &agent_shares[presig_index as usize];

        // Verify R points match
        assert_eq!(cold_share.r_point, agent_share.r_point);

        // Simulate signature (in production this would use the zkVM)
        let message_hash = MessageHash::new([i as u8; 32]);
        let signature = Signature::new([i as u8; 64]);

        // Mark presig as used
        disk.mark_presig_used(presig_index).unwrap();

        // Log the usage
        let entry = UsageLogEntry::new(
            presig_index,
            current_time + (i as u64 * 60), // 1 minute apart
            message_hash,
            signature,
            ChainId::ETHEREUM,
            TxHash::new([i as u8; 32]),
            ZkProofHash::new([i as u8; 32]),
            format!("Transaction {}", i),
        );
        disk.usage_log.push(entry).unwrap();
    }

    // Verify state after signing
    assert_eq!(disk.header.presig_used, num_signatures as u32);
    assert_eq!(disk.usage_log.len(), num_signatures);
    assert_eq!(
        disk.header.presigs_remaining(),
        (presig_count - num_signatures) as u32
    );

    // ==========================================
    // STEP 4: Reconciliation
    // ==========================================
    let analysis = analyze_disk(&disk);

    // Should be clean (no anomalies)
    assert!(
        analysis.anomalies.is_empty(),
        "Unexpected anomalies: {:?}",
        analysis.anomalies
    );
    assert_eq!(analysis.used_presigs, num_signatures as u32);
    assert!(analysis.passed);

    // Generate report
    let report = generate_report(&analysis);
    assert!(report.contains("passed"));

    // Record reconciliation in registry
    registry
        .record_reconciliation(&child_id, num_signatures as u32)
        .unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert_eq!(child.total_signatures, num_signatures as u64);

    // ==========================================
    // STEP 5: Refill disk
    // ==========================================
    // Generate new presigs
    let new_presig_pairs =
        PresigGenerator::generate_batch(&cold_child_shard, &agent_child_shard, presig_count)
            .unwrap();

    let new_cold_shares: Vec<PresigColdShare> = new_presig_pairs
        .iter()
        .map(|p| p.cold_share.clone())
        .collect();

    // Reset disk
    disk.header.presig_used = 0;
    disk.header.expiry = DiskExpiry::new(current_time);
    disk.presigs = new_cold_shares;
    disk.usage_log = UsageLog::new();

    // Record another reconciliation (simulating refill)
    registry.record_reconciliation(&child_id, 0).unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert_eq!(child.refill_count, 2);

    // Verify disk is ready for new signatures
    assert_eq!(disk.header.presigs_remaining(), presig_count as u32);
    assert!(disk.usage_log.is_empty());

    // Silence unused variable warning
    let _ = master_output;
}

/// Test anomaly detection during reconciliation
#[test]
fn test_reconciliation_anomaly_detection() {
    let child_id = ChildId::new([0x01; 32]);
    let child_pubkey = PublicKey::new([0x02; 33]);
    let current_time = 1700000000u64;

    // ==========================================
    // Test 1: Presig count mismatch
    // ==========================================
    let mut header = DiskHeader::new(
        child_id,
        child_pubkey,
        DerivationPath::ethereum_hardened(0),
        100,
        current_time,
    );
    header.presig_used = 50; // Header claims 50 used

    let presigs: Vec<PresigColdShare> = (0..100)
        .map(|i| {
            let mut share = PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]);
            // Only 30 actually marked used
            if i < 30 {
                share.mark_used();
            }
            share
        })
        .collect();

    let disk = DiskFormat::new(header, presigs);
    let analysis = analyze_disk(&disk);

    assert!(
        !analysis.anomalies.is_empty(),
        "Should detect count mismatch"
    );
    assert!(!analysis.passed);
}

/// Test disk expiration handling
#[test]
fn test_disk_expiration() {
    let current_time = 1700000000u64;
    let mut header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        100,
        current_time - (60 * 24 * 60 * 60), // Created 60 days ago
    );

    // Set expiry to the past
    header.expiry.expires_at = current_time - (10 * 24 * 60 * 60); // Expired 10 days ago

    // Validation should fail
    assert!(header.validate(current_time).is_err());
    assert!(header.expiry.is_expired(current_time));
}

/// Test max uses before reconciliation
#[test]
fn test_max_uses_enforcement() {
    let current_time = 1700000000u64;
    let mut header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        1000,
        current_time,
    );

    // Set max uses and simulate exceeding it
    header.expiry.max_uses_before_reconcile = 100;
    header.expiry.uses_since_reconcile = 101;

    // Validation should fail
    let result = header.validate(current_time);
    assert!(result.is_err());
    assert!(header.expiry.is_max_uses_exceeded());
}

/// Test nullification workflow
#[test]
fn test_nullification_workflow() {
    let mut registry = ChildRegistry::new();

    // Create and register child
    let child_id = ChildId::new([0x01; 32]);
    registry
        .register_child(child_id, DerivationPath::ethereum_hardened(0))
        .unwrap();

    // Simulate some usage via reconciliation
    registry.record_reconciliation(&child_id, 50).unwrap();

    // Nullify due to suspected compromise
    let last_valid_index = 45; // Last known good signature
    registry
        .nullify_child(
            &child_id,
            NullificationReason::ReconciliationAnomaly {
                description: "Test anomaly".to_string(),
            },
            last_valid_index,
        )
        .unwrap();

    // Verify nullification
    let child = registry.get_child(&child_id).unwrap();
    match &child.status {
        ChildStatus::Nullified {
            reason,
            last_valid_presig_index,
            timestamp,
        } => {
            assert!(matches!(
                reason,
                NullificationReason::ReconciliationAnomaly { .. }
            ));
            assert_eq!(*last_valid_presig_index, last_valid_index);
            assert!(*timestamp > 0);
        }
        _ => panic!("Expected nullified status"),
    }

    // Should not be able to reactivate a nullified child
    assert!(registry.reactivate_child(&child_id).is_err());
}

/// Test disk serialization preserves all data
#[test]
fn test_disk_serialization_integrity() {
    let current_time = 1700000000u64;
    let header = DiskHeader::new(
        ChildId::new([0x7a; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(42),
        50,
        current_time,
    );

    let presigs: Vec<PresigColdShare> = (0..50)
        .map(|i| {
            let mut share = PresigColdShare::new(
                [0x02 + (i as u8 % 2); 33], // Alternate prefix
                [(i * 2) as u8; 32],
                [(i * 3) as u8; 32],
            );
            if i < 10 {
                share.mark_used();
            }
            share
        })
        .collect();

    let mut disk = DiskFormat::new(header, presigs);
    disk.header.presig_used = 10;

    // Add some usage log entries
    for i in 0..10 {
        disk.usage_log
            .push(UsageLogEntry::new(
                i,
                current_time + (i as u64 * 100),
                MessageHash::new([i as u8; 32]),
                Signature::new([i as u8; 64]),
                ChainId::ETHEREUM,
                TxHash::new([i as u8; 32]),
                ZkProofHash::new([i as u8; 32]),
                format!("Tx #{}", i),
            ))
            .unwrap();
    }

    // Serialize and deserialize
    let bytes = disk.to_bytes();
    let recovered = DiskFormat::from_bytes(&bytes).unwrap();

    // Verify header
    assert_eq!(disk.header.child_id, recovered.header.child_id);
    assert_eq!(disk.header.presig_total, recovered.header.presig_total);
    assert_eq!(disk.header.presig_used, recovered.header.presig_used);
    assert_eq!(disk.header.created_at, recovered.header.created_at);

    // Verify presigs
    assert_eq!(disk.presigs.len(), recovered.presigs.len());
    for (orig, rec) in disk.presigs.iter().zip(recovered.presigs.iter()) {
        assert_eq!(orig.r_point, rec.r_point);
        assert_eq!(orig.k_cold, rec.k_cold);
        assert_eq!(orig.chi_cold, rec.chi_cold);
        assert_eq!(orig.status, rec.status);
    }

    // Verify usage log
    assert_eq!(disk.usage_log.len(), recovered.usage_log.len());
    for (orig, rec) in disk
        .usage_log
        .entries
        .iter()
        .zip(recovered.usage_log.entries.iter())
    {
        assert_eq!(orig.presig_index, rec.presig_index);
        assert_eq!(orig.timestamp, rec.timestamp);
        assert_eq!(orig.description, rec.description);
    }
}

/// Test concurrent usage patterns (simulating multiple rapid signatures)
#[test]
fn test_rapid_signature_sequence() {
    let current_time = 1700000000u64;
    let header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        1000,
        current_time,
    );

    let presigs: Vec<PresigColdShare> = (0..1000)
        .map(|i| PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]))
        .collect();

    let mut disk = DiskFormat::new(header, presigs);

    // Simulate rapid sequence of 100 signatures
    for i in 0..100 {
        let (index, _) = disk.get_next_presig().unwrap();
        assert_eq!(index, i);

        disk.mark_presig_used(index).unwrap();

        disk.usage_log
            .push(UsageLogEntry::new(
                index,
                current_time + i as u64, // 1 second apart
                MessageHash::new([i as u8; 32]),
                Signature::new([i as u8; 64]),
                ChainId::ETHEREUM,
                TxHash::new([i as u8; 32]),
                ZkProofHash::new([i as u8; 32]),
                format!("Rapid tx {}", i),
            ))
            .unwrap();
    }

    // Verify state
    assert_eq!(disk.header.presig_used, 100);
    assert_eq!(disk.usage_log.len(), 100);

    // Validate log integrity
    assert!(disk.usage_log.validate().is_ok());

    // Reconciliation should be clean
    let analysis = analyze_disk(&disk);
    assert!(analysis.anomalies.is_empty());
}
