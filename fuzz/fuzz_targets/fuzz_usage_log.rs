#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil_core::usage::{UsageLog, UsageLogEntry};

fuzz_target!(|data: &[u8]| {
    // Try parsing as usage log
    if let Some(log) = UsageLog::from_bytes(data) {
        // Validation should not panic
        let _ = log.validate();

        // Round-trip
        let reserialized = log.to_bytes();
        if let Some(log2) = UsageLog::from_bytes(&reserialized) {
            assert_eq!(log.len(), log2.len());
        }
    }

    // Try parsing as single entry (minimum 178 bytes)
    if data.len() >= 178 {
        if let Some(entry) = UsageLogEntry::from_bytes(data) {
            let reserialized = entry.to_bytes();
            if let Some(entry2) = UsageLogEntry::from_bytes(&reserialized) {
                assert_eq!(entry.presig_index, entry2.presig_index);
                assert_eq!(entry.timestamp, entry2.timestamp);
            }
        }
    }
});
