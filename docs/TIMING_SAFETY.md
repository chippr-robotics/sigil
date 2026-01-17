# Timing Safety Analysis

This document analyzes the timing safety of cryptographic operations in Sigil and documents mitigations against timing side-channel attacks.

## 1. Critical Operations

### 1.1 Operations That MUST Be Constant-Time

| Operation | Location | Current Implementation |
|-----------|----------|------------------------|
| Scalar multiplication | sigil-zkvm/signing | k256 `Scalar::mul` |
| Scalar inversion | sigil-zkvm/signing | k256 `Scalar::invert` |
| Scalar addition | sigil-zkvm/signing | k256 `Scalar::add` |
| Point addition | sigil-core/crypto | k256 `ProjectivePoint::add` |
| Low-S normalization | sigil-zkvm/signing | k256 `Scalar::ct_gt` |
| Secret comparison | Various | `subtle::ConstantTimeEq` |

### 1.2 Operations That Are Timing-Insensitive

| Operation | Reason |
|-----------|--------|
| Disk header parsing | Public data |
| Presig status check | Public information |
| Usage log parsing | Public audit data |
| Error messages | Do not depend on secrets |

## 2. k256 Crate Analysis

The `k256` crate (v0.13) provides constant-time implementations for secp256k1 operations:

### 2.1 Verified Constant-Time Features

```rust
// Using the "arithmetic" feature enables:
// - Constant-time scalar multiplication
// - Constant-time scalar inversion (via Fermat's little theorem)
// - Constant-time point operations

// In Cargo.toml:
k256 = { version = "0.13", features = ["ecdsa", "arithmetic"] }
```

### 2.2 k256 Security Guarantees

From k256 documentation:
- "This crate provides a constant-time, heap-less implementation of secp256k1"
- Uses the `subtle` crate for constant-time comparisons
- Scalar operations avoid branching on secret values

## 3. Code Review Checklist

### 3.1 sigil-zkvm/src/signing.rs

```rust
// ✓ SAFE: k256 Scalar operations are constant-time
let k_cold = decode_scalar(&input.presig_cold.k_share)?;
let k_agent = decode_scalar(&input.presig_agent.k_share)?;
let k = k_cold + k_agent;

// ✓ SAFE: k256 inversion is constant-time (Fermat's method)
let k_inv = k.invert();

// ✓ SAFE: constant-time comparison using ct_gt
let s = normalize_s(s);
```

### 3.2 sigil-core/src/crypto.rs

```rust
// ✓ SAFE: k256 point operations are constant-time
let sum = ProjectivePoint::from(point1) + ProjectivePoint::from(point2);
```

### 3.3 Potential Issues

```rust
// ⚠️ REVIEW: Early return on R-point mismatch
// This is timing-safe because R-points are public values
if input.presig_cold.r_point != input.presig_agent.r_point {
    return Err("R point mismatch between parties");
}
```

## 4. Memory Safety

### 4.1 Zeroization

The `zeroize` crate is used to clear sensitive data from memory:

```rust
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PresigColdShare {
    pub r_point: [u8; 33],
    #[zeroize(skip)]  // Intentionally not zeroized (less sensitive)
    pub k_cold: [u8; 32],
    // ...
}
```

**Note:** Some fields marked `#[zeroize(skip)]` should be reviewed:
- `k_cold`, `chi_cold` - nonce shares, single-use
- Consider if these should be zeroized after signing

### 4.2 Memory Allocation

- Core crypto uses stack allocation (no heap)
- Large structures (disk format) use heap but don't contain secrets
- zkVM execution is sandboxed

## 5. Recommendations

### 5.1 High Priority

1. **Add `subtle` crate** for explicit constant-time comparisons:
   ```rust
   use subtle::ConstantTimeEq;

   // For comparing secret data
   if r_point_cold.ct_eq(&r_point_agent).into() {
       // ...
   }
   ```

2. **Audit zeroization coverage**:
   - Add `#[zeroize(drop)]` to more sensitive structures
   - Consider `zeroize_on_drop` for all presig shares

3. **Add timing tests**:
   ```rust
   #[test]
   fn test_signing_time_independent_of_input() {
       // Measure signing time for various inputs
       // Verify variance is within acceptable bounds
   }
   ```

### 5.2 Medium Priority

4. **Document public vs secret data**:
   - Add comments marking which fields are public/secret
   - Use newtypes to enforce handling

5. **Consider blinding**:
   - Add scalar blinding for extra protection
   - `s = k_inv * (z + r * chi)` could use blinding factor

### 5.3 Future Work

6. **Formal verification**:
   - Use tools like `haybale` or `binsec` for timing analysis
   - Consider Frama-C for critical functions

7. **Hardware security module (HSM) support**:
   - Move signing to HSM for hardware-level protection
   - Use PKCS#11 or similar interface

## 6. Testing

### 6.1 Timing Test Framework

```rust
// Example timing test (not included in main test suite due to noise)
#[cfg(feature = "timing-tests")]
mod timing_tests {
    use std::time::Instant;

    #[test]
    fn signing_time_variance() {
        let iterations = 1000;
        let mut times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            // Perform signing operation
            let elapsed = start.elapsed();
            times.push(elapsed.as_nanos());
        }

        let mean = times.iter().sum::<u128>() / iterations as u128;
        let variance = times.iter()
            .map(|t| (*t as i128 - mean as i128).pow(2))
            .sum::<i128>() / iterations as i128;

        // Variance should be low and not correlated with input
        assert!(variance < ACCEPTABLE_VARIANCE);
    }
}
```

### 6.2 Running Timing Analysis

```bash
# Build with timing feature
cargo build --release --features timing-tests

# Run with CPU frequency pinning (Linux)
sudo cpupower frequency-set -g performance
cargo test --release timing_tests -- --nocapture
```

## 7. Changelog

- **v0.1.0** (2026-01-17): Initial timing safety analysis
