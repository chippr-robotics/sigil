#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil_core::disk::{DiskHeader, HEADER_SIZE};

fuzz_target!(|data: &[u8]| {
    // Ensure we have enough data for a header
    if data.len() >= HEADER_SIZE {
        let mut header_bytes = [0u8; HEADER_SIZE];
        header_bytes.copy_from_slice(&data[..HEADER_SIZE]);

        // Try to parse - should not panic
        if let Ok(header) = DiskHeader::from_bytes(&header_bytes) {
            // If parsing succeeded, serialization should round-trip
            let reserialized = header.to_bytes();

            // Deserialize again
            if let Ok(header2) = DiskHeader::from_bytes(&reserialized) {
                // Key fields should match
                assert_eq!(header.child_id, header2.child_id);
                assert_eq!(header.presig_total, header2.presig_total);
                assert_eq!(header.presig_used, header2.presig_used);
            }
        }
    }
});
