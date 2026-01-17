#![no_main]

use libfuzzer_sys::fuzz_target;
use sigil_core::crypto::DerivationPath;

fuzz_target!(|data: &[u8]| {
    // Try parsing 32 bytes as derivation path
    if data.len() >= 32 {
        let mut path_bytes = [0u8; 32];
        path_bytes.copy_from_slice(&data[..32]);

        if let Ok(path) = DerivationPath::from_bytes(&path_bytes) {
            // Depth should be valid
            assert!(path.depth <= 5);

            // Round-trip
            let reserialized = path.to_bytes();
            let path2 = DerivationPath::from_bytes(&reserialized).unwrap();

            assert_eq!(path.depth, path2.depth);
            assert_eq!(path.components, path2.components);

            // String conversion should not panic
            let _ = path.to_string_path();
        }
    }

    // Try creating from components
    if data.len() >= 4 {
        let num_components = (data[0] % 6) as usize; // 0-5 components
        if data.len() >= 1 + num_components * 4 {
            let mut components = Vec::with_capacity(num_components);
            for i in 0..num_components {
                let offset = 1 + i * 4;
                let component = u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                components.push(component);
            }

            if let Ok(path) = DerivationPath::new(&components) {
                assert_eq!(path.depth as usize, num_components);
            }
        }
    }
});
