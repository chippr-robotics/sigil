//! Property-based tests for sigil-core using proptest
//!
//! These tests verify invariants that should hold for all valid inputs.

use proptest::prelude::*;
use sigil_core::{
    crypto::{DerivationPath, PublicKey},
    disk::{DiskFormat, DiskHeader},
    presig::{PresigColdShare, PresigStatus},
    types::{ChainId, ChildId, MessageHash, Signature, TxHash, ZkProofHash},
    usage::{UsageLog, UsageLogEntry},
    PRESIG_ENTRY_SIZE,
};

// ============================================
// Arbitrary Implementations
// ============================================

fn arb_child_id() -> impl Strategy<Value = ChildId> {
    any::<[u8; 32]>().prop_map(ChildId::new)
}

fn arb_public_key() -> impl Strategy<Value = PublicKey> {
    // Valid compressed public key prefixes are 0x02 and 0x03
    (prop::bool::ANY, any::<[u8; 32]>()).prop_map(|(high_y, x)| {
        let mut bytes = [0u8; 33];
        bytes[0] = if high_y { 0x03 } else { 0x02 };
        bytes[1..].copy_from_slice(&x);
        PublicKey::new(bytes)
    })
}

fn arb_derivation_path() -> impl Strategy<Value = DerivationPath> {
    (0u8..=5, any::<[u32; 5]>())
        .prop_map(|(depth, components)| DerivationPath { depth, components })
}

fn arb_presig_cold_share() -> impl Strategy<Value = PresigColdShare> {
    (any::<[u8; 33]>(), any::<[u8; 32]>(), any::<[u8; 32]>())
        .prop_map(|(r, k, chi)| PresigColdShare::new(r, k, chi))
}

fn arb_presig_status() -> impl Strategy<Value = PresigStatus> {
    prop_oneof![
        Just(PresigStatus::Fresh),
        Just(PresigStatus::Used),
        Just(PresigStatus::Voided),
    ]
}

fn arb_message_hash() -> impl Strategy<Value = MessageHash> {
    any::<[u8; 32]>().prop_map(MessageHash::new)
}

fn arb_signature() -> impl Strategy<Value = Signature> {
    any::<[u8; 64]>().prop_map(Signature::new)
}

fn arb_tx_hash() -> impl Strategy<Value = TxHash> {
    any::<[u8; 32]>().prop_map(TxHash::new)
}

fn arb_zkproof_hash() -> impl Strategy<Value = ZkProofHash> {
    any::<[u8; 32]>().prop_map(ZkProofHash::new)
}

fn arb_chain_id() -> impl Strategy<Value = ChainId> {
    any::<u32>().prop_map(ChainId::new)
}

fn arb_usage_log_entry() -> impl Strategy<Value = UsageLogEntry> {
    (
        any::<u32>(),
        any::<u64>(),
        arb_message_hash(),
        arb_signature(),
        arb_chain_id(),
        arb_tx_hash(),
        arb_zkproof_hash(),
        "[a-zA-Z0-9 ]{0,100}",
    )
        .prop_map(
            |(
                presig_index,
                timestamp,
                message_hash,
                signature,
                chain_id,
                tx_hash,
                zkproof_hash,
                description,
            )| {
                UsageLogEntry::new(
                    presig_index,
                    timestamp,
                    message_hash,
                    signature,
                    chain_id,
                    tx_hash,
                    zkproof_hash,
                    description,
                )
            },
        )
}

fn arb_disk_header() -> impl Strategy<Value = DiskHeader> {
    (
        arb_child_id(),
        arb_public_key(),
        arb_derivation_path(),
        1u32..=1000,
        any::<u64>(),
    )
        .prop_map(|(child_id, pubkey, path, presig_total, created_at)| {
            DiskHeader::new(child_id, pubkey, path, presig_total, created_at)
        })
}

// ============================================
// Property Tests
// ============================================

proptest! {
    // ----------------------------------------
    // ChildId Properties
    // ----------------------------------------

    #[test]
    fn child_id_hex_roundtrip(id in arb_child_id()) {
        let hex = id.to_hex();
        let recovered = ChildId::from_hex(&hex).unwrap();
        prop_assert_eq!(id, recovered);
    }

    #[test]
    fn child_id_short_is_prefix(id in arb_child_id()) {
        let hex = id.to_hex();
        let short = id.short();
        prop_assert!(hex.starts_with(&short));
        prop_assert_eq!(short.len(), 8); // 4 bytes = 8 hex chars
    }

    // ----------------------------------------
    // Derivation Path Properties
    // ----------------------------------------

    #[test]
    fn derivation_path_serialization_roundtrip(path in arb_derivation_path()) {
        let bytes = path.to_bytes();
        let recovered = DerivationPath::from_bytes(&bytes).unwrap();
        prop_assert_eq!(path.depth, recovered.depth);
        for i in 0..path.depth as usize {
            prop_assert_eq!(path.components[i], recovered.components[i]);
        }
    }

    #[test]
    fn derivation_path_depth_bounded(path in arb_derivation_path()) {
        prop_assert!(path.depth <= 5);
    }

    // ----------------------------------------
    // PresigColdShare Properties
    // ----------------------------------------

    #[test]
    fn presig_serialization_roundtrip(presig in arb_presig_cold_share()) {
        let bytes = presig.to_bytes();
        prop_assert_eq!(bytes.len(), PRESIG_ENTRY_SIZE);

        let recovered = PresigColdShare::from_bytes(&bytes);
        prop_assert_eq!(presig.r_point, recovered.r_point);
        prop_assert_eq!(presig.k_cold, recovered.k_cold);
        prop_assert_eq!(presig.chi_cold, recovered.chi_cold);
        prop_assert_eq!(presig.status, recovered.status);
    }

    #[test]
    fn presig_fresh_by_default(presig in arb_presig_cold_share()) {
        prop_assert_eq!(presig.status, PresigStatus::Fresh);
        prop_assert!(presig.is_fresh());
    }

    #[test]
    fn presig_status_transitions_are_monotonic(mut presig in arb_presig_cold_share()) {
        // Fresh presig
        prop_assert!(presig.is_fresh());

        // Mark used
        presig.mark_used();
        prop_assert!(!presig.is_fresh());
        prop_assert_eq!(presig.status, PresigStatus::Used);

        // Cannot go back to fresh (no API for it)
    }

    // ----------------------------------------
    // UsageLogEntry Properties
    // ----------------------------------------

    #[test]
    fn usage_log_entry_roundtrip(entry in arb_usage_log_entry()) {
        let bytes = entry.to_bytes();
        let recovered = UsageLogEntry::from_bytes(&bytes).unwrap();

        prop_assert_eq!(entry.presig_index, recovered.presig_index);
        prop_assert_eq!(entry.timestamp, recovered.timestamp);
        prop_assert_eq!(entry.message_hash.0, recovered.message_hash.0);
        prop_assert_eq!(entry.signature.0, recovered.signature.0);
        prop_assert_eq!(entry.chain_id.0, recovered.chain_id.0);
    }

    #[test]
    fn usage_log_entry_description_truncated(
        presig_index in any::<u32>(),
        timestamp in any::<u64>(),
        long_desc in "[a-z]{300}"
    ) {
        let entry = UsageLogEntry::new(
            presig_index,
            timestamp,
            MessageHash::new([0; 32]),
            Signature::new([0; 64]),
            ChainId::ETHEREUM,
            TxHash::new([0; 32]),
            ZkProofHash::new([0; 32]),
            long_desc,
        );

        prop_assert!(entry.description.len() <= UsageLogEntry::MAX_DESCRIPTION_LEN);
    }

    // ----------------------------------------
    // UsageLog Properties
    // ----------------------------------------

    #[test]
    fn usage_log_ordered_entries_validate(entries in prop::collection::vec(any::<u64>(), 0..10)) {
        let mut log = UsageLog::new();
        let mut last_index = 0u32;
        let mut base_timestamp = 1700000000u64;

        for (i, _) in entries.iter().enumerate() {
            let entry = UsageLogEntry::new(
                last_index + 1,
                base_timestamp + (i as u64 * 1000),
                MessageHash::new([i as u8; 32]),
                Signature::new([i as u8; 64]),
                ChainId::ETHEREUM,
                TxHash::new([i as u8; 32]),
                ZkProofHash::new([i as u8; 32]),
                format!("Entry {}", i),
            );
            log.push(entry).unwrap();
            last_index += 1;
            base_timestamp += 1000;
        }

        prop_assert!(log.validate().is_ok());
    }

    // ----------------------------------------
    // DiskHeader Properties
    // ----------------------------------------

    #[test]
    fn disk_header_serialization_roundtrip(header in arb_disk_header()) {
        let bytes = header.to_bytes();
        prop_assert_eq!(bytes.len(), 256);

        let recovered = DiskHeader::from_bytes(&bytes).unwrap();
        prop_assert_eq!(header.child_id, recovered.child_id);
        prop_assert_eq!(header.presig_total, recovered.presig_total);
        prop_assert_eq!(header.presig_used, recovered.presig_used);
        prop_assert_eq!(header.created_at, recovered.created_at);
    }

    #[test]
    fn disk_header_presigs_remaining_invariant(
        child_id in arb_child_id(),
        pubkey in arb_public_key(),
        path in arb_derivation_path(),
        presig_total in 1u32..=1000,
        presig_used in 0u32..=1000,
        created_at in any::<u64>()
    ) {
        let mut header = DiskHeader::new(child_id, pubkey, path, presig_total, created_at);
        header.presig_used = presig_used.min(presig_total);

        let remaining = header.presigs_remaining();
        prop_assert!(remaining <= presig_total);
        prop_assert_eq!(remaining, presig_total.saturating_sub(header.presig_used));
    }

    #[test]
    fn disk_header_has_presigs_consistent(header in arb_disk_header()) {
        let has = header.has_presigs();
        let remaining = header.presigs_remaining();

        if has {
            prop_assert!(remaining > 0);
        } else {
            prop_assert_eq!(remaining, 0);
        }
    }

    // ----------------------------------------
    // Signature Properties
    // ----------------------------------------

    #[test]
    fn signature_components(sig in arb_signature()) {
        let r = sig.r();
        let s = sig.s();

        // r and s should be the two halves
        prop_assert_eq!(r, &sig.0[..32]);
        prop_assert_eq!(s, &sig.0[32..]);
    }

    #[test]
    fn signature_hex_roundtrip(sig in arb_signature()) {
        let hex = sig.to_hex();
        let recovered = Signature::from_hex(&hex).unwrap();
        prop_assert_eq!(sig.0, recovered.0);
    }

    // ----------------------------------------
    // MessageHash Properties
    // ----------------------------------------

    #[test]
    fn message_hash_hex_roundtrip(hash in arb_message_hash()) {
        let hex = hash.to_hex();
        let recovered = MessageHash::from_hex(&hex).unwrap();
        prop_assert_eq!(hash.0, recovered.0);
    }

    // ----------------------------------------
    // ChainId Properties
    // ----------------------------------------

    #[test]
    fn chain_id_roundtrip(id in any::<u32>()) {
        let chain_id = ChainId::new(id);
        prop_assert_eq!(chain_id.as_u32(), id);
    }
}

// ============================================
// Invariant Tests (non-proptest)
// ============================================

#[test]
fn presig_status_byte_values() {
    // Ensure status byte values are stable
    assert_eq!(PresigStatus::Fresh as u8, 0);
    assert_eq!(PresigStatus::Used as u8, 1);
    assert_eq!(PresigStatus::Voided as u8, 2);

    // Unknown values should map to Voided
    assert_eq!(PresigStatus::from(255), PresigStatus::Voided);
}

#[test]
fn chain_id_constants_stable() {
    assert_eq!(ChainId::ETHEREUM.0, 1);
    assert_eq!(ChainId::SEPOLIA.0, 11155111);
    assert_eq!(ChainId::ARBITRUM.0, 42161);
    assert_eq!(ChainId::OPTIMISM.0, 10);
    assert_eq!(ChainId::BASE.0, 8453);
    assert_eq!(ChainId::POLYGON.0, 137);
}
