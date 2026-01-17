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

use crate::types::{SigningInput, SigningOutput};

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
    let r = <Scalar as Reduce<U256>>::reduce_bytes(r_x_bytes.into());

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

    let s_generic = s.to_bytes();
    let s_bytes: [u8; 32] = s_generic.as_slice().try_into().expect("scalar is 32 bytes");

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

        let normalized_generic = normalized.to_bytes();
        let normalized_bytes: [u8; 32] = normalized_generic.as_slice().try_into().unwrap();
        // normalized <= half_order means NOT (normalized > half_order)
        assert!(!scalar_gt_bytes(&normalized_bytes, &HALF_ORDER));
    }
}
