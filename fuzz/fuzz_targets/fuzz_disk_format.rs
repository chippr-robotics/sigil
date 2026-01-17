#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil_core::disk::DiskFormat;

fuzz_target!(|data: &[u8]| {
    // Try parsing as full disk format
    // This exercises header parsing, presig table parsing, and usage log parsing
    if let Ok(disk) = DiskFormat::from_bytes(data) {
        // Validation should not panic (even if it returns error)
        let current_time = 1700000000u64;
        let _ = disk.validate(current_time);

        // Status summary should not panic
        let _ = disk.status_summary(current_time);

        // Round-trip for valid disks
        let reserialized = disk.to_bytes();
        if let Ok(disk2) = DiskFormat::from_bytes(&reserialized) {
            assert_eq!(disk.header.child_id, disk2.header.child_id);
            assert_eq!(disk.header.presig_total, disk2.header.presig_total);
            assert_eq!(disk.presigs.len(), disk2.presigs.len());
        }
    }
});
