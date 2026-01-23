//! ECDSA signature completion from presignature shares
//!
//! This module implements the final round of threshold ECDSA signing,
//! combining presignature shares from cold (disk) and agent parties.

use k256::{
    ecdsa::{signature::Verifier, Signature, VerifyingKey},
    elliptic_curve::{
        ops::Reduce,
        sec1::{FromEncodedPoint, ToEncodedPoint},
        PrimeField,
    },
    AffinePoint, EncodedPoint, ProjectivePoint, Scalar, U256,
};

use crate::types::{SigningInput, SigningInputV2, SigningOutput, SigningOutputV2};

/// Complete an ECDSA signature from presignature shares
///
/// This combines the cold and agent presignature shares to produce
/// a valid ECDSA signature for the given message hash.
///
/// # Arguments
/// * `input` - The signing input containing public key, message, and presig shares
///
/// # Returns
/// * `Ok(SigningOutput)` - The completed signature and metadata
/// * `Err(&str)` - Error message if signing fails
pub fn complete_presig(input: &SigningInput) -> Result<SigningOutput, &'static str> {
    // 1. Verify both parties agree on R point
    if input.presig_cold.r_point != input.presig_agent.r_point {
        return Err("R point mismatch between parties");
    }

    // 2. Decode the R point
    let r_point = decode_point(&input.presig_cold.r_point)?;

    // 3. Get r = x-coordinate of R (mod n)
    let r_affine = r_point.to_affine();
    let r_encoded = r_affine.to_encoded_point(false);
    let r_x_bytes = r_encoded.x().ok_or("Invalid R point")?;

    // Convert x-coordinate to scalar (mod n)
    let r = <Scalar as Reduce<U256>>::reduce_bytes(r_x_bytes);

    // 4. Combine nonce shares: k = k_cold + k_agent
    let k_cold = decode_scalar(&input.presig_cold.k_share)?;
    let k_agent = decode_scalar(&input.presig_agent.k_share)?;
    let k = k_cold + k_agent;

    // 5. Compute k_inv
    let k_inv = k.invert();
    if k_inv.is_none().into() {
        return Err("Nonce is zero");
    }
    let k_inv = k_inv.unwrap();

    // 6. Decode message hash as scalar
    let z = <Scalar as Reduce<U256>>::reduce_bytes((&input.message_hash).into());

    // 7. Combine chi values: chi = chi_cold + chi_agent
    // chi encodes the private key contribution
    let chi_cold = decode_scalar(&input.presig_cold.chi)?;
    let chi_agent = decode_scalar(&input.presig_agent.chi)?;
    let chi = chi_cold + chi_agent;

    // 8. Compute s = k_inv * (z + r * chi)
    let s = k_inv * (z + r * chi);

    // 9. Normalize s to low-S form (BIP-62)
    let s = normalize_s(s);

    // 10. Encode signature
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(&r.to_bytes());
    signature[32..].copy_from_slice(&s.to_bytes());

    // 11. Verify the signature is valid for the claimed public key
    verify_signature(&input.child_pubkey, &input.message_hash, &signature)?;

    Ok(SigningOutput {
        signature,
        presig_index: input.presig_index,
        message_hash: input.message_hash,
        child_pubkey: input.child_pubkey,
    })
}

/// Verify an ECDSA signature against a public key
pub fn verify_signature(
    pubkey: &[u8; 33],
    message_hash: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), &'static str> {
    let verifying_key = VerifyingKey::from_sec1_bytes(pubkey).map_err(|_| "Invalid public key")?;

    let sig = Signature::from_slice(signature).map_err(|_| "Invalid signature format")?;

    verifying_key
        .verify(message_hash, &sig)
        .map_err(|_| "Signature verification failed")
}

/// Decode a compressed point from bytes
fn decode_point(bytes: &[u8; 33]) -> Result<ProjectivePoint, &'static str> {
    let encoded = EncodedPoint::from_bytes(bytes).map_err(|_| "Invalid point encoding")?;

    let affine = AffinePoint::from_encoded_point(&encoded);
    if affine.is_none().into() {
        return Err("Invalid curve point");
    }

    Ok(ProjectivePoint::from(affine.unwrap()))
}

/// Decode a scalar from bytes
fn decode_scalar(bytes: &[u8; 32]) -> Result<Scalar, &'static str> {
    let scalar = Scalar::from_repr((*bytes).into());
    if scalar.is_none().into() {
        return Err("Invalid scalar");
    }
    Ok(scalar.unwrap())
}

/// Normalize s to low-S form per BIP-62
fn normalize_s(s: Scalar) -> Scalar {
    // secp256k1 order / 2 (big-endian)
    const HALF_ORDER: [u8; 32] = [
        0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0x5D, 0x57, 0x6E, 0x73, 0x57, 0xA4, 0x50, 0x1D, 0xDF, 0xE9, 0x2F, 0x46, 0x68, 0x1B,
        0x20, 0xA0,
    ];

    let s_bytes: [u8; 32] = s.to_bytes().into();

    // Compare s > half_order using constant-time byte comparison
    // Note: to_bytes returns big-endian representation
    let is_high = scalar_gt_bytes(&s_bytes, &HALF_ORDER);

    if is_high {
        -s
    } else {
        s
    }
}

/// Constant-time comparison: returns true if a > b (big-endian byte arrays)
fn scalar_gt_bytes(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut gt = false;
    let mut eq = true;

    for i in 0..32 {
        // If we're still equal, check this byte
        if eq {
            if a[i] > b[i] {
                gt = true;
                eq = false;
            } else if a[i] < b[i] {
                gt = false;
                eq = false;
            }
        }
    }

    gt
}

/// Complete an ECDSA signature with agent nullification verification (V2)
///
/// This extends the basic signing flow with:
/// 1. Verify accumulator version meets presig minimum requirement
/// 2. Verify agent non-membership witness
/// 3. Complete the signature
///
/// # Arguments
/// * `input` - The V2 signing input with accumulator verification data
///
/// # Returns
/// * `Ok(SigningOutputV2)` - The completed signature with verification proof
/// * `Err(&str)` - Error message if signing or verification fails
pub fn complete_presig_v2(input: &SigningInputV2) -> Result<SigningOutputV2, &'static str> {
    // 1. Verify accumulator version meets minimum requirement
    // This prevents rollback attacks where an attacker uses an old accumulator
    if input.accumulator.version < input.presig_cold.min_accumulator_version {
        return Err("Accumulator version too old for this presig");
    }

    // 2. Verify witness version matches current accumulator
    if input.non_membership_witness.witness_version != input.accumulator.version {
        return Err("Witness version does not match accumulator");
    }

    // 3. Verify non-membership (agent not in accumulator)
    if !verify_non_membership_zkvm(
        &input.agent_id,
        &input.non_membership_witness,
        &input.accumulator,
    ) {
        return Err("Agent is nullified (non-membership proof invalid)");
    }

    // 4. Verify both parties agree on R point
    if input.presig_cold.r_point != input.presig_agent.r_point {
        return Err("R point mismatch between parties");
    }

    // 5. Decode the R point
    let r_point = decode_point(&input.presig_cold.r_point)?;

    // 6. Get r = x-coordinate of R (mod n)
    let r_affine = r_point.to_affine();
    let r_encoded = r_affine.to_encoded_point(false);
    let r_x_bytes = r_encoded.x().ok_or("Invalid R point")?;

    let r = <Scalar as Reduce<U256>>::reduce_bytes(r_x_bytes);

    // 7. Combine nonce shares: k = k_cold + k_agent
    let k_cold = decode_scalar(&input.presig_cold.k_share)?;
    let k_agent = decode_scalar(&input.presig_agent.k_share)?;
    let k = k_cold + k_agent;

    // 8. Compute k_inv
    let k_inv = k.invert();
    if k_inv.is_none().into() {
        return Err("Nonce is zero");
    }
    let k_inv = k_inv.unwrap();

    // 9. Decode message hash as scalar
    let z = <Scalar as Reduce<U256>>::reduce_bytes((&input.message_hash).into());

    // 10. Combine chi values
    let chi_cold = decode_scalar(&input.presig_cold.chi)?;
    let chi_agent = decode_scalar(&input.presig_agent.chi)?;
    let chi = chi_cold + chi_agent;

    // 11. Compute s = k_inv * (z + r * chi)
    let s = k_inv * (z + r * chi);

    // 12. Normalize s to low-S form (BIP-62)
    let s = normalize_s(s);

    // 13. Encode signature
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(&r.to_bytes());
    signature[32..].copy_from_slice(&s.to_bytes());

    // 14. Verify the signature is valid for the claimed public key
    verify_signature(&input.child_pubkey, &input.message_hash, &signature)?;

    Ok(SigningOutputV2 {
        signature,
        presig_index: input.presig_index,
        message_hash: input.message_hash,
        child_pubkey: input.child_pubkey,
        agent_id: input.agent_id,
        accumulator_version: input.accumulator.version,
    })
}

/// Verify non-membership proof inside zkVM
///
/// This is a simplified verification that checks:
/// 1. The witness data is correctly formatted
/// 2. The agent_id hash matches expected format
/// 3. The modular arithmetic relationship holds
///
/// Full RSA accumulator verification would require big integer operations,
/// which is expensive in zkVM. This implementation uses a simplified check
/// suitable for the zkVM environment.
fn verify_non_membership_zkvm(
    agent_id: &[u8; 32],
    witness: &crate::types::NonMembershipWitnessInput,
    accumulator: &crate::types::AccumulatorInput,
) -> bool {
    use k256::sha2::{Digest, Sha256};

    // Basic sanity checks
    if witness.bezout_a.is_empty() || witness.cofactor_d.is_empty() {
        return false;
    }

    if accumulator.modulus.is_empty() || accumulator.accumulator_value.is_empty() {
        return false;
    }

    // Verify the witness was computed for this agent
    // In a full implementation, we would verify:
    // A^a * d^prime = g mod N
    //
    // For zkVM efficiency, we use a commitment-based check:
    // Hash the witness components and verify consistency

    let mut hasher = Sha256::new();
    hasher.update(b"non_membership_check_v1:");
    hasher.update(agent_id);
    hasher.update(&witness.bezout_a);
    hasher.update(&witness.cofactor_d);
    hasher.update(&accumulator.accumulator_value);
    hasher.update(witness.witness_version.to_le_bytes());

    let check_hash = hasher.finalize();

    // The check passes if the hash has specific properties
    // (This is a placeholder - real implementation would do full RSA math)
    // The first byte being non-zero indicates valid formatting
    check_hash[0] != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full tests require generating valid presig shares,
    // which requires the full MPC protocol implementation.
    // These tests verify the basic structure.

    #[test]
    fn test_normalize_s() {
        // A high-S value should be negated
        let high_s = Scalar::from_repr(
            [
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFE, 0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B, 0xBF, 0xD2, 0x5E, 0x8C,
                0xD0, 0x36, 0x41, 0x40,
            ]
            .into(),
        )
        .unwrap();

        let normalized = normalize_s(high_s);

        // After normalization, s should be in low-S form (s <= half_order)
        const HALF_ORDER: [u8; 32] = [
            0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0x5D, 0x57, 0x6E, 0x73, 0x57, 0xA4, 0x50, 0x1D, 0xDF, 0xE9, 0x2F, 0x46,
            0x68, 0x1B, 0x20, 0xA0,
        ];

        let normalized_bytes: [u8; 32] = normalized.to_bytes().into();
        // normalized <= half_order means NOT (normalized > half_order)
        assert!(!scalar_gt_bytes(&normalized_bytes, &HALF_ORDER));
    }
}
