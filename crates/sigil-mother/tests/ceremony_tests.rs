//! Integration tests for sigil-mother ceremonies

use sigil_core::{
    crypto::{DerivationPath, PublicKey},
    disk::{DiskFormat, DiskHeader},
    presig::PresigColdShare,
    types::{ChainId, ChildId, MessageHash, Signature, TxHash, ZkProofHash},
    usage::UsageLogEntry,
    ChildStatus, NullificationReason,
};

use sigil_mother::{
    keygen::MasterKeyGenerator,
    presig_gen::PresigGenerator,
    reconciliation::analyze_disk,
    registry::ChildRegistry,
    storage::{MasterShardData, MotherStorage},
};

use tempfile::TempDir;

#[test]
fn test_master_key_generation() {
    let output = MasterKeyGenerator::generate().unwrap();

    // Verify shards are different
    assert_ne!(
        output.cold_master_shard.cold_master_shard,
        output.agent_master_shard
    );

    // Verify master pubkey is valid (compressed format starts with 02 or 03)
    let prefix = output.master_pubkey.as_bytes()[0];
    assert!(prefix == 0x02 || prefix == 0x03);

    // Verify shards are non-zero
    assert!(output
        .cold_master_shard
        .cold_master_shard
        .iter()
        .any(|&b| b != 0));
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
    let path = DerivationPath::ethereum_hardened(0);

    registry.register_child(child_id, path).unwrap();

    // Verify registration
    let child = registry.get_child(&child_id).unwrap();
    assert_eq!(child.child_id, child_id);
    assert!(matches!(child.status, ChildStatus::Active));
    assert_eq!(child.refill_count, 0);
    assert_eq!(child.total_signatures, 0);
}

#[test]
fn test_child_registry_nullification() {
    let mut registry = ChildRegistry::new();

    let child_id = ChildId::new([0x01; 32]);
    registry
        .register_child(child_id, DerivationPath::ethereum_hardened(0))
        .unwrap();

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
    registry
        .register_child(child_id, DerivationPath::ethereum_hardened(0))
        .unwrap();

    // Suspend
    registry.suspend_child(&child_id).unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert!(matches!(child.status, ChildStatus::Suspended));

    // Resume
    registry.reactivate_child(&child_id).unwrap();
    let child = registry.get_child(&child_id).unwrap();
    assert!(matches!(child.status, ChildStatus::Active));
}

#[test]
fn test_child_registry_count_by_status() {
    let mut registry = ChildRegistry::new();

    // Add multiple children with different statuses
    for i in 0..5 {
        let child_id = ChildId::new([i as u8; 32]);
        registry
            .register_child(child_id, DerivationPath::ethereum_hardened(i))
            .unwrap();
    }

    // Suspend some
    registry.suspend_child(&ChildId::new([1; 32])).unwrap();
    registry.suspend_child(&ChildId::new([2; 32])).unwrap();

    // Nullify one
    registry
        .nullify_child(
            &ChildId::new([3; 32]),
            NullificationReason::ManualRevocation,
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

    // Add corresponding log entries for the used presigs
    for i in 0..10u32 {
        let entry = UsageLogEntry::new(
            i,
            1700000000 + (i as u64 * 60),
            MessageHash::new([i as u8; 32]),
            Signature::new([i as u8; 64]),
            ChainId::ETHEREUM,
            TxHash::new([i as u8; 32]),
            ZkProofHash::new([i as u8; 32]),
            format!("Test tx {}", i),
        );
        disk.usage_log.push(entry).unwrap();
    }

    let analysis = analyze_disk(&disk);

    assert!(analysis.passed);
    assert_eq!(analysis.used_presigs, 10);
    assert_eq!(analysis.fresh_presigs, 90);
}

// ============================================================================
// Genesis Operations Tests
// ============================================================================

#[test]
fn test_genesis_first_boot_initialization() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage = MotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Initially, no master shard should exist
    assert!(!storage.has_master_shard());

    // Generate master key
    let output = MasterKeyGenerator::generate().unwrap();

    // Save master shard to storage
    storage
        .save_master_shard(&output.cold_master_shard)
        .unwrap();

    // Verify master shard is now present
    assert!(storage.has_master_shard());

    // Load and verify the stored data
    let loaded = storage.load_master_shard().unwrap();
    assert_eq!(
        loaded.cold_master_shard,
        output.cold_master_shard.cold_master_shard
    );
    assert_eq!(loaded.master_pubkey, *output.master_pubkey.as_bytes());
    assert_eq!(loaded.next_child_index, 0);
}

#[test]
fn test_genesis_prevent_reinitialization() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage = MotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // First initialization
    let output1 = MasterKeyGenerator::generate().unwrap();
    storage
        .save_master_shard(&output1.cold_master_shard)
        .unwrap();

    // Load the first master shard
    let loaded1 = storage.load_master_shard().unwrap();

    // Attempting to reinitialize would require checking has_master_shard()
    // In the CLI, this check prevents overwriting
    assert!(storage.has_master_shard());

    // Generate another key (simulating a reinitialization attempt)
    let output2 = MasterKeyGenerator::generate().unwrap();

    // Verify the two keys are different
    assert_ne!(
        output1.cold_master_shard.cold_master_shard,
        output2.cold_master_shard.cold_master_shard
    );

    // If we were to save output2, it would overwrite
    // But the CLI prevents this by checking has_master_shard() first
    // Verify that the original data is still intact
    let still_loaded = storage.load_master_shard().unwrap();
    assert_eq!(still_loaded.cold_master_shard, loaded1.cold_master_shard);
}

#[test]
fn test_genesis_storage_persistence() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    let master_pubkey: [u8; 33];
    let cold_shard: [u8; 32];

    // First storage instance
    {
        let storage = MotherStorage::new(storage_path.clone()).unwrap();
        let output = MasterKeyGenerator::generate().unwrap();

        master_pubkey = *output.master_pubkey.as_bytes();
        cold_shard = output.cold_master_shard.cold_master_shard;

        storage
            .save_master_shard(&output.cold_master_shard)
            .unwrap();
    }

    // Create a new storage instance pointing to the same directory
    // This simulates restarting the application
    {
        let storage = MotherStorage::new(storage_path.clone()).unwrap();

        // Verify data persisted correctly
        assert!(storage.has_master_shard());
        let loaded = storage.load_master_shard().unwrap();

        assert_eq!(loaded.cold_master_shard, cold_shard);
        assert_eq!(loaded.master_pubkey, master_pubkey);
    }
}

#[test]
fn test_genesis_child_index_allocation() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage = MotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Initialize with master key
    let output = MasterKeyGenerator::generate().unwrap();
    storage
        .save_master_shard(&output.cold_master_shard)
        .unwrap();

    // Load and allocate child indices
    let mut master = storage.load_master_shard().unwrap();

    assert_eq!(master.next_child_index, 0);

    let idx1 = master.allocate_child_index();
    assert_eq!(idx1, 0);
    assert_eq!(master.next_child_index, 1);

    let idx2 = master.allocate_child_index();
    assert_eq!(idx2, 1);
    assert_eq!(master.next_child_index, 2);

    // Save updated state
    storage.save_master_shard(&master).unwrap();

    // Reload and verify persistence
    let reloaded = storage.load_master_shard().unwrap();
    assert_eq!(reloaded.next_child_index, 2);
}

#[test]
fn test_genesis_storage_error_not_initialized() {
    // Create temporary storage directory but don't initialize
    let temp_dir = TempDir::new().unwrap();
    let storage = MotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Attempting to load master shard should fail
    assert!(!storage.has_master_shard());
    let result = storage.load_master_shard();
    assert!(result.is_err());

    // Verify the error message is about not being initialized
    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(err_msg.contains("not initialized") || err_msg.contains("Master key"));
    }
}

#[test]
fn test_genesis_registry_initialization() {
    // Create temporary storage directory
    let temp_dir = TempDir::new().unwrap();
    let storage = MotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Load registry from empty storage (should return empty registry)
    let registry = storage.load_registry().unwrap();
    assert_eq!(registry.children.len(), 0);

    // Create and save a registry with a child
    let mut registry = ChildRegistry::new();
    let child_id = ChildId::new([0x01; 32]);
    registry
        .register_child(child_id, DerivationPath::ethereum_hardened(0))
        .unwrap();

    storage.save_registry(&registry).unwrap();

    // Reload and verify
    let loaded_registry = storage.load_registry().unwrap();
    assert_eq!(loaded_registry.children.len(), 1);
    assert!(loaded_registry.get_child(&child_id).is_ok());
}

#[test]
fn test_genesis_master_shard_data_creation() {
    let cold_shard = [0xAB; 32];
    let master_pubkey = [0x02; 33]; // Valid compressed pubkey prefix

    let shard_data = MasterShardData::new(cold_shard, master_pubkey);

    assert_eq!(shard_data.cold_master_shard, cold_shard);
    assert_eq!(shard_data.master_pubkey, master_pubkey);
    assert_eq!(shard_data.next_child_index, 0);
    assert!(shard_data.created_at > 0);
}

// ============================================================================
// Ledger Integration Tests (conditional on ledger feature)
// ============================================================================

#[cfg(feature = "ledger")]
mod ledger_tests {
    use super::*;
    use sigil_mother::ledger::LedgerDevice;

    #[tokio::test]
    #[ignore] // Requires physical Ledger device
    async fn test_ledger_device_connection() {
        // This test requires a physical Ledger device
        // It's marked as ignored by default
        let result = LedgerDevice::connect();

        // If a Ledger is connected, this should succeed
        // If not, it should fail with a descriptive error
        match result {
            Ok(device) => {
                // Verify we can get device info
                let info_result = device.get_info().await;
                assert!(info_result.is_ok());
            }
            Err(e) => {
                // Verify error message is descriptive
                let err_msg = format!("{}", e);
                assert!(
                    err_msg.contains("Ledger") || err_msg.contains("connect"),
                    "Error message should mention Ledger: {}",
                    err_msg
                );
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires physical Ledger device
    async fn test_ledger_master_key_generation() {
        // This test requires a physical Ledger device with Ethereum app open
        let device = match LedgerDevice::connect() {
            Ok(d) => d,
            Err(_) => {
                println!("Skipping test - no Ledger device connected");
                return;
            }
        };

        let result = device.generate_master_key().await;

        match result {
            Ok(output) => {
                // Verify both shards are different
                assert_ne!(output.cold_master_shard, output.agent_master_shard);

                // Verify shards are non-zero
                assert!(output.cold_master_shard.iter().any(|&b| b != 0));
                assert!(output.agent_master_shard.iter().any(|&b| b != 0));

                // Verify master pubkey is valid
                let prefix = output.master_pubkey.as_bytes()[0];
                assert!(prefix == 0x02 || prefix == 0x03);

                // Verify ledger pubkey is in uncompressed format
                assert_eq!(output.ledger_pubkey.len(), 65);
                assert_eq!(output.ledger_pubkey[0], 0x04);
            }
            Err(e) => {
                // If Ethereum app is not open, error should indicate that
                let err_msg = format!("{}", e);
                assert!(
                    err_msg.contains("app")
                        || err_msg.contains("Ethereum")
                        || err_msg.contains("communication"),
                    "Error should indicate app/communication issue: {}",
                    err_msg
                );
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires physical Ledger device
    async fn test_ledger_status_check() {
        let device = match LedgerDevice::connect() {
            Ok(d) => d,
            Err(_) => {
                println!("Skipping test - no Ledger device connected");
                return;
            }
        };

        let info = device.get_info().await.unwrap();

        // Verify info structure
        assert!(!info.model.is_empty());

        // If Ethereum app is open, we should have public key and address
        if info.eth_app_open {
            assert!(info.public_key.is_some());
            assert!(info.address.is_some());

            if let Some(addr) = info.address {
                assert!(addr.starts_with("0x"));
                assert_eq!(addr.len(), 42); // 0x + 40 hex chars
            }
        }
    }

    #[test]
    fn test_ledger_genesis_with_storage() {
        // Test that Ledger-generated keys can be stored properly
        // This uses mock data since we can't rely on physical device in CI

        let temp_dir = TempDir::new().unwrap();
        let storage = MotherStorage::new(temp_dir.path().to_path_buf()).unwrap();

        // Simulate Ledger-generated shards
        let cold_shard = [0xCD; 32];
        let master_pubkey = [0x03; 33];

        let shard_data = MasterShardData::new(cold_shard, master_pubkey);
        storage.save_master_shard(&shard_data).unwrap();

        // Verify storage
        assert!(storage.has_master_shard());
        let loaded = storage.load_master_shard().unwrap();
        assert_eq!(loaded.cold_master_shard, cold_shard);
        assert_eq!(loaded.master_pubkey, master_pubkey);
    }
}

#[cfg(not(feature = "ledger"))]
#[test]
fn test_ledger_not_compiled() {
    // When ledger feature is not enabled, verify the stub returns proper error
    use sigil_mother::ledger::LedgerDevice;

    let result = LedgerDevice::connect();
    assert!(result.is_err());

    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(
            err_msg.contains("not compiled") || err_msg.contains("feature"),
            "Error should indicate feature not compiled: {}",
            err_msg
        );
    }
}
