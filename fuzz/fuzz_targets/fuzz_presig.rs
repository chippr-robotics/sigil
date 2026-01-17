#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil_core::presig::{PresigColdShare, PresigStatus};
use sigil_core::PRESIG_ENTRY_SIZE;

fuzz_target!(|data: &[u8]| {
    if data.len() >= PRESIG_ENTRY_SIZE {
        let mut presig_bytes = [0u8; PRESIG_ENTRY_SIZE];
        presig_bytes.copy_from_slice(&data[..PRESIG_ENTRY_SIZE]);

        // Parse presig - should not panic
        let presig = PresigColdShare::from_bytes(&presig_bytes);

        // Verify status is valid
        assert!(matches!(
            presig.status,
            PresigStatus::Fresh | PresigStatus::Used | PresigStatus::Voided
        ));

        // Round-trip should preserve data
        let reserialized = presig.to_bytes();
        let presig2 = PresigColdShare::from_bytes(&reserialized);

        assert_eq!(presig.r_point, presig2.r_point);
        assert_eq!(presig.k_cold, presig2.k_cold);
        assert_eq!(presig.chi_cold, presig2.chi_cold);
        assert_eq!(presig.status, presig2.status);
    }
});
