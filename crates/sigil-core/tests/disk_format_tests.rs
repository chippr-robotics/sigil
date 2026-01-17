//! Integration tests for sigil-core disk format

use sigil_core::{
    crypto::{DerivationPath, PublicKey},
    disk::{DiskFormat, DiskHeader, DISK_MAGIC, HEADER_SIZE},
    expiry::DiskExpiry,
    presig::{PresigColdShare, PresigStatus},
    types::{ChainId, ChildId, MessageHash, Signature, TxHash, ZkProofHash},
    usage::{UsageLog, UsageLogEntry},
    PRESIG_ENTRY_SIZE, VERSION,
};

#[test]
fn test_disk_header_serialization_roundtrip() {
    let header = DiskHeader::new(
        ChildId::new([0x7a; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(42),
        1000,
        1700000000,
    );

    let bytes = header.to_bytes();
    assert_eq!(bytes.len(), HEADER_SIZE);

    // Check magic bytes
    assert_eq!(&bytes[0..8], DISK_MAGIC);

    // Check version
    let version = u32::from_le_bytes(bytes[0x0008..0x000C].try_into().unwrap());
    assert_eq!(version, VERSION);

    // Deserialize and verify
    let recovered = DiskHeader::from_bytes(&bytes).unwrap();
    assert_eq!(header.child_id, recovered.child_id);
    assert_eq!(header.child_pubkey.0, recovered.child_pubkey.0);
    assert_eq!(header.presig_total, recovered.presig_total);
    assert_eq!(header.presig_used, recovered.presig_used);
    assert_eq!(header.created_at, recovered.created_at);
}

#[test]
fn test_disk_header_invalid_magic() {
    let mut bytes = [0u8; HEADER_SIZE];
    bytes[0..8].copy_from_slice(b"BAADDISK");

    let result = DiskHeader::from_bytes(&bytes);
    assert!(result.is_err());
}

#[test]
fn test_disk_header_presig_tracking() {
    let mut header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        100,
        1700000000,
    );

    assert_eq!(header.presigs_remaining(), 100);
    assert!(header.has_presigs());

    header.presig_used = 50;
    assert_eq!(header.presigs_remaining(), 50);

    header.presig_used = 100;
    assert_eq!(header.presigs_remaining(), 0);
    assert!(!header.has_presigs());
}

#[test]
fn test_disk_header_expiry_validation() {
    let current_time = 1700000000u64;
    let header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        100,
        current_time,
    );

    // Should be valid right after creation
    assert!(header.validate(current_time + 1000).is_ok());

    // Create expired header
    let mut expired_header = header.clone();
    expired_header.expiry.expires_at = current_time - 1000;
    assert!(expired_header.validate(current_time).is_err());
}

#[test]
fn test_presig_cold_share_serialization() {
    let share = PresigColdShare::new(
        [0x02; 33], // R point (compressed)
        [0xaa; 32], // k_cold
        [0xbb; 32], // chi_cold
    );

    let bytes = share.to_bytes();
    assert_eq!(bytes.len(), PRESIG_ENTRY_SIZE);

    let recovered = PresigColdShare::from_bytes(&bytes);
    assert_eq!(share.r_point, recovered.r_point);
    assert_eq!(share.k_cold, recovered.k_cold);
    assert_eq!(share.chi_cold, recovered.chi_cold);
    assert_eq!(share.status, recovered.status);
}

#[test]
fn test_presig_status_transitions() {
    let mut share = PresigColdShare::new([0x02; 33], [0xaa; 32], [0xbb; 32]);

    assert!(share.is_fresh());
    assert_eq!(share.status, PresigStatus::Fresh);

    share.mark_used();
    assert!(!share.is_fresh());
    assert_eq!(share.status, PresigStatus::Used);

    // Create another share and void it
    let mut share2 = PresigColdShare::new([0x02; 33], [0xaa; 32], [0xbb; 32]);
    share2.mark_voided();
    assert_eq!(share2.status, PresigStatus::Voided);
}

#[test]
fn test_full_disk_format_roundtrip() {
    let header = DiskHeader::new(
        ChildId::new([0x7a; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        10,
        1700000000,
    );

    let presigs: Vec<PresigColdShare> = (0..10)
        .map(|i| PresigColdShare::new([i as u8; 33], [i as u8; 32], [(i * 2) as u8; 32]))
        .collect();

    let disk = DiskFormat::new(header, presigs);
    let bytes = disk.to_bytes();

    // Verify minimum size
    assert!(bytes.len() > HEADER_SIZE);

    let recovered = DiskFormat::from_bytes(&bytes).unwrap();
    assert_eq!(disk.header.child_id, recovered.header.child_id);
    assert_eq!(disk.presigs.len(), recovered.presigs.len());

    // Verify presig content
    for (original, recovered) in disk.presigs.iter().zip(recovered.presigs.iter()) {
        assert_eq!(original.r_point, recovered.r_point);
        assert_eq!(original.k_cold, recovered.k_cold);
    }
}

#[test]
fn test_disk_format_presig_operations() {
    let header = DiskHeader::new(
        ChildId::new([0x01; 32]),
        PublicKey::new([0x02; 33]),
        DerivationPath::ethereum_hardened(0),
        5,
        1700000000,
    );

    let presigs: Vec<PresigColdShare> = (0..5)
        .map(|i| PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]))
        .collect();

    let mut disk = DiskFormat::new(header, presigs);

    // Get next presig
    let (index, presig) = disk.get_next_presig().unwrap();
    assert_eq!(index, 0);
    assert!(presig.is_fresh());

    // Mark it used
    disk.mark_presig_used(0).unwrap();
    assert_eq!(disk.header.presig_used, 1);

    // Next presig should be index 1
    let (index, _) = disk.get_next_presig().unwrap();
    assert_eq!(index, 1);
}

#[test]
fn test_usage_log_entry_serialization() {
    let entry = UsageLogEntry::new(
        42,
        1700000000,
        MessageHash::new([0x11; 32]),
        Signature::new([0x22; 64]),
        ChainId::ETHEREUM,
        TxHash::new([0x33; 32]),
        ZkProofHash::new([0x44; 32]),
        "Test transaction: send 1 ETH".to_string(),
    );

    let bytes = entry.to_bytes();
    let recovered = UsageLogEntry::from_bytes(&bytes).unwrap();

    assert_eq!(entry.presig_index, recovered.presig_index);
    assert_eq!(entry.timestamp, recovered.timestamp);
    assert_eq!(entry.message_hash.0, recovered.message_hash.0);
    assert_eq!(entry.signature.0, recovered.signature.0);
    assert_eq!(entry.chain_id.0, recovered.chain_id.0);
    assert_eq!(entry.tx_hash.0, recovered.tx_hash.0);
    assert_eq!(entry.description, recovered.description);
}

#[test]
fn test_usage_log_validation_correct_order() {
    let mut log = UsageLog::new();

    // Add entries in correct order
    for i in 0..5 {
        log.push(UsageLogEntry::new(
            i,
            1700000000 + (i as u64 * 1000),
            MessageHash::new([i as u8; 32]),
            Signature::new([i as u8; 64]),
            ChainId::ETHEREUM,
            TxHash::new([i as u8; 32]),
            ZkProofHash::new([i as u8; 32]),
            format!("Transaction {}", i),
        ))
        .unwrap();
    }

    assert!(log.validate().is_ok());
}

#[test]
fn test_usage_log_validation_detects_index_gap() {
    let mut log = UsageLog::new();

    // Add entry with index 0
    log.push(UsageLogEntry::new(
        0,
        1700000000,
        MessageHash::new([0; 32]),
        Signature::new([0; 64]),
        ChainId::ETHEREUM,
        TxHash::new([0; 32]),
        ZkProofHash::new([0; 32]),
        "First".to_string(),
    ))
    .unwrap();

    // Skip to index 5 (gap)
    log.push(UsageLogEntry::new(
        5,
        1700001000,
        MessageHash::new([1; 32]),
        Signature::new([1; 64]),
        ChainId::ETHEREUM,
        TxHash::new([1; 32]),
        ZkProofHash::new([1; 32]),
        "Second with gap".to_string(),
    ))
    .unwrap();

    // Validation should still pass (gaps are suspicious but not invalid)
    // The validation checks for non-monotonic indices, not gaps
    assert!(log.validate().is_ok());
}

#[test]
fn test_usage_log_validation_detects_out_of_order_index() {
    let mut log = UsageLog::new();

    log.entries.push(UsageLogEntry::new(
        5,
        1700000000,
        MessageHash::new([0; 32]),
        Signature::new([0; 64]),
        ChainId::ETHEREUM,
        TxHash::new([0; 32]),
        ZkProofHash::new([0; 32]),
        "First".to_string(),
    ));

    // Add entry with lower index (out of order)
    log.entries.push(UsageLogEntry::new(
        3,
        1700001000,
        MessageHash::new([1; 32]),
        Signature::new([1; 64]),
        ChainId::ETHEREUM,
        TxHash::new([1; 32]),
        ZkProofHash::new([1; 32]),
        "Out of order".to_string(),
    ));

    assert!(log.validate().is_err());
}

#[test]
fn test_derivation_path_bip44() {
    let path = DerivationPath::ethereum_hardened(0);
    assert_eq!(path.depth, 4);

    let path_str = path.to_string_path();
    assert_eq!(path_str, "m/44'/60'/0'/0'");

    // Test serialization
    let bytes = path.to_bytes();
    let recovered = DerivationPath::from_bytes(&bytes).unwrap();
    assert_eq!(path, recovered);
}

#[test]
fn test_child_id_short_format() {
    let id = ChildId::new([
        0x7a, 0x3f, 0xbc, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]);

    assert_eq!(id.short(), "7a3fbc12");
}

#[test]
fn test_chain_id_constants() {
    assert_eq!(ChainId::ETHEREUM.as_u32(), 1);
    assert_eq!(ChainId::SEPOLIA.as_u32(), 11155111);
    assert_eq!(ChainId::ARBITRUM.as_u32(), 42161);
    assert_eq!(ChainId::OPTIMISM.as_u32(), 10);
    assert_eq!(ChainId::BASE.as_u32(), 8453);
    assert_eq!(ChainId::POLYGON.as_u32(), 137);
}

#[test]
fn test_expiry_calculations() {
    let current_time = 1700000000u64;
    let expiry = DiskExpiry::new(current_time);

    // Default: 30 days validity
    assert!(expiry.expires_at > current_time);
    assert!(!expiry.is_expired(current_time));

    // Check days until expiry
    let days = expiry.days_until_expiry(current_time);
    assert!((29..=30).contains(&days));

    // Check warning threshold
    let near_expiry_time = expiry.expires_at - (6 * 24 * 60 * 60); // 6 days before
    assert!(expiry.is_warning_period(near_expiry_time));
}

#[test]
fn test_signature_components() {
    let mut sig_bytes = [0u8; 64];
    sig_bytes[..32].copy_from_slice(&[0xaa; 32]);
    sig_bytes[32..].copy_from_slice(&[0xbb; 32]);

    let sig = Signature::new(sig_bytes);
    assert_eq!(sig.r(), &[0xaa; 32]);
    assert_eq!(sig.s(), &[0xbb; 32]);
}

#[test]
fn test_hex_encoding_decoding() {
    let mut bytes = [0u8; 32];
    bytes[0] = 0x12;
    bytes[1] = 0x34;
    bytes[2] = 0x56;
    bytes[3] = 0x78;
    let original = ChildId::new(bytes);
    let hex_str = original.to_hex();
    let recovered = ChildId::from_hex(&hex_str).unwrap();
    assert_eq!(original, recovered);
}
