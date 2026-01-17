//! Integration tests for sigil-mother ceremonies

use sigil_core::{
    crypto::{DerivationPath, PublicKey},
    disk::{DiskFormat, DiskHeader},
    presig::PresigColdShare,
    types::ChildId,
    ChildStatus, NullificationReason,
};

use sigil_mother::{
    keygen::MasterKeyGenerator,
    presig_gen::PresigGenerator,
    reconciliation::{analyze_disk, ReconciliationRecommendation},
    registry::ChildRegistry,
};

use std::collections::HashMap;

#[test]
fn test_master_key_generation() {
    let output = MasterKeyGenerator::generate().unwrap();

    // Verify shards are different
    assert_ne!(output.cold_master_shard, output.agent_master_shard);

    // Verify master pubkey is valid (compressed format starts with 02 or 03)
    let prefix = output.master_pubkey.as_bytes()[0];
    assert!(prefix == 0x02 || prefix == 0x03);

    // Verify shards are non-zero
    assert!(output.cold_master_shard.iter().any(|&b| b != 0));
    assert!(output.agent_master_shard.iter().any(|&b| b != 0));
}

#[test]
fn test_presig_generation_batch() {
    let cold_shard = [0x11; 32];
    let agent_shard = [0x22; 32];

    let pairs = PresigGenerator::generate_batch(&cold_shard, &agent_shard, 100).unwrap();

    assert_eq!(pairs.len(), 100);

    // Verify all pairs have matching R points
    for pair in &pairs {
        assert_eq!(pair.cold_share.r_point, pair.agent_share.r_point);
    }

    // Verify R points are unique (with overwhelming probability)
    let r_points: std::collections::HashSet<_> =
        pairs.iter().map(|p| p.cold_share.r_point).collect();
    assert_eq!(r_points.len(), 100);
}

#[test]
fn test_presig_generation_deterministic_r_point() {
    let cold_shard = [0x11; 32];
    let agent_shard = [0x22; 32];

    // Generate two separate pairs
    let pairs1 = PresigGenerator::generate_batch(&cold_shard, &agent_shard, 1).unwrap();
    let pairs2 = PresigGenerator::generate_batch(&cold_shard, &agent_shard, 1).unwrap();

    // R points should be different (randomness in nonce generation)
    assert_ne!(pairs1[0].cold_share.r_point, pairs2[0].cold_share.r_point);
}

#[test]
fn test_child_registry_operations() {
    let mut registry = ChildRegistry::new();

    // Register a child
    let child_id = ChildId::new([0x01; 32]);
    let pubkey = PublicKey::new([0x02; 33]);
    let path = DerivationPath::ethereum_hardened(0);

    registry.register_child(child_id, pubkey, path);

    // Verify registration
    let child = registry.get_child(&child_id).unwrap();
    assert_eq!(child.child_id, child_id);
    assert!(matches!(child.status, ChildStatus::Active));
    assert_eq!(child.refill_count, 0);
    assert_eq!(child.total_signatures, 0);

    // Update signature count
    registry.record_signatures(&child_id, 50).unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert_eq!(child.total_signatures, 50);

    // Record refill
    registry.record_refill(&child_id).unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert_eq!(child.refill_count, 1);
}

#[test]
fn test_child_registry_nullification() {
    let mut registry = ChildRegistry::new();

    let child_id = ChildId::new([0x01; 32]);
    registry.register_child(
        child_id,
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
    );

    // Nullify the child
    registry
        .nullify_child(&child_id, NullificationReason::ManualRevocation, 42)
        .unwrap();

    let child = registry.get_child(&child_id).unwrap();
    match &child.status {
        ChildStatus::Nullified {
            reason,
            last_valid_presig_index,
            ..
        } => {
            assert!(matches!(reason, NullificationReason::ManualRevocation));
            assert_eq!(*last_valid_presig_index, 42);
        }
        _ => panic!("Expected nullified status"),
    }
}

#[test]
fn test_child_registry_suspend_resume() {
    let mut registry = ChildRegistry::new();

    let child_id = ChildId::new([0x01; 32]);
    registry.register_child(
        child_id,
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
    );

    // Suspend
    registry.suspend_child(&child_id).unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert!(matches!(child.status, ChildStatus::Suspended));

    // Resume
    registry.resume_child(&child_id).unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert!(matches!(child.status, ChildStatus::Active));
}

#[test]
fn test_child_registry_count_by_status() {
    let mut registry = ChildRegistry::new();

    // Add multiple children with different statuses
    for i in 0..5 {
        let child_id = ChildId::new([i as u8; 32]);
        registry.register_child(
            child_id,
            PublicKey::new([0x02; 33]),
            DerivationPath::ethereum_hardened(i),
        );
    }

    // Suspend some
    registry.suspend_child(&ChildId::new([1; 32])).unwrap();
    registry.suspend_child(&ChildId::new([2; 32])).unwrap();

    // Nullify one
    registry
        .nullify_child(
            &ChildId::new([3; 32]),
            NullificationReason::LostOrStolen,
            0,
        )
        .unwrap();

    let (active, suspended, nullified) = registry.count_by_status();
    assert_eq!(active, 2);
    assert_eq!(suspended, 2);
    assert_eq!(nullified, 1);
}

#[test]
fn test_reconciliation_analysis_clean_disk() {
    // Create a clean disk with sequential usage
    let header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        100,
        1700000000,
    );

    let presigs: Vec<PresigColdShare> = (0..100)
        .map(|i| {
            let mut share = PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]);
            // Mark first 10 as used
            if i < 10 {
                share.mark_used();
            }
            share
        })
        .collect();

    let mut disk = DiskFormat::new(header, presigs);
    disk.header.presig_used = 10;

    let analysis = analyze_disk(&disk);

    assert!(analysis.anomalies.is_empty());
    assert_eq!(analysis.presigs_used, 10);
    assert_eq!(analysis.presigs_remaining, 90);
}

#[test]
fn test_reconciliation_analysis_count_mismatch() {
    let mut header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        100,
        1700000000,
    );
    header.presig_used = 20; // Header says 20 used

    let presigs: Vec<PresigColdShare> = (0..100)
        .map(|i| {
            let mut share = PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]);
            // But only 10 are actually marked used
            if i < 10 {
                share.mark_used();
            }
            share
        })
        .collect();

    let disk = DiskFormat::new(header, presigs);
    let analysis = analyze_disk(&disk);

    // Should detect the mismatch
    assert!(!analysis.anomalies.is_empty());
    assert!(analysis
        .anomalies
        .iter()
        .any(|a| a.contains("mismatch") || a.contains("Mismatch")));
}

#[test]
fn test_reconciliation_recommendation_clean() {
    let header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        100,
        1700000000,
    );

    let presigs: Vec<PresigColdShare> = (0..100)
        .map(|i| PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]))
        .collect();

    let disk = DiskFormat::new(header, presigs);
    let analysis = analyze_disk(&disk);

    assert!(matches!(
        analysis.recommendation,
        ReconciliationRecommendation::Refill
    ));
}
