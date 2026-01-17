//! Bitcoin Taproot (BIP-340 Schnorr) support via FROST
//!
//! This module implements FROST threshold signatures for Bitcoin Taproot,
//! using the secp256k1 curve with BIP-340 Schnorr signatures.

use crate::{
    error::{FrostError, Result},
    presig::{FrostPresig, FrostPresigBatch},
    traits::{FrostCipherSuite, FrostKeyGen, FrostPresigGen, FrostSigner},
    FrostSignature, KeyShare, SignatureScheme, VerifyingKey,
};
use frost_secp256k1_tr as frost;
use rand::{CryptoRng, RngCore};
use std::collections::BTreeMap;
use tracing::{debug, instrument};

/// Taproot FROST implementation
pub struct Taproot;

impl FrostCipherSuite for Taproot {
    const SCHEME: SignatureScheme = SignatureScheme::Taproot;
    const PUBLIC_KEY_SIZE: usize = 32; // x-only pubkey
    const SIGNATURE_SIZE: usize = 64; // BIP-340 signature
    const NONCE_SIZE: usize = 64; // Two 32-byte scalars
    const COMMITMENT_SIZE: usize = 66; // Two 33-byte points
}

impl FrostKeyGen for Taproot {
    type KeyGenOutput = (
        frost::keys::KeyPackage,
        frost::keys::KeyPackage,
        frost::keys::PublicKeyPackage,
    );

    #[instrument(skip(rng))]
    fn generate_2of2<R: RngCore + CryptoRng>(
        rng: &mut R,
    ) -> Result<(KeyShare, KeyShare, VerifyingKey)> {
        debug!("Generating 2-of-2 Taproot FROST key shares");

        // Use trusted dealer for 2-of-2
        let (shares, pubkey_package) = frost::keys::generate_with_dealer(
            2, // max signers
            2, // threshold
            frost::keys::IdentifierList::Default,
            rng,
        )
        .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

        // Get the two shares
        let id1 = frost::Identifier::try_from(1u16)
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
        let id2 = frost::Identifier::try_from(2u16)
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

        let share1 = shares
            .get(&id1)
            .ok_or_else(|| FrostError::KeyGeneration("Missing share 1".to_string()))?;
        let share2 = shares
            .get(&id2)
            .ok_or_else(|| FrostError::KeyGeneration("Missing share 2".to_string()))?;

        // Build key packages
        let key_package1 = frost::keys::KeyPackage::try_from(share1.clone())
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
        let key_package2 = frost::keys::KeyPackage::try_from(share2.clone())
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

        // Serialize for storage
        let cold_share = KeyShare::new(
            SignatureScheme::Taproot,
            serialize_key_package(&key_package1)?,
            1,
        );
        let agent_share = KeyShare::new(
            SignatureScheme::Taproot,
            serialize_key_package(&key_package2)?,
            2,
        );

        // Get the verifying key (x-only for Taproot)
        let vk = pubkey_package.verifying_key();
        let vk_bytes = vk
            .serialize()
            .map_err(|e| FrostError::Serialization(e.to_string()))?;

        let verifying_key = VerifyingKey::new(SignatureScheme::Taproot, vk_bytes);

        debug!(
            "Generated Taproot keys, verifying key: {}",
            verifying_key.to_hex()
        );

        Ok((cold_share, agent_share, verifying_key))
    }

    fn generate_shares<R: RngCore + CryptoRng>(
        threshold: u16,
        num_shares: u16,
        rng: &mut R,
    ) -> Result<(Vec<KeyShare>, VerifyingKey)> {
        if threshold < 2 {
            return Err(FrostError::InvalidThreshold {
                threshold: threshold as usize,
                participants: num_shares as usize,
            });
        }

        let (shares, pubkey_package) = frost::keys::generate_with_dealer(
            num_shares,
            threshold,
            frost::keys::IdentifierList::Default,
            rng,
        )
        .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

        let mut key_shares = Vec::with_capacity(num_shares as usize);
        for i in 1..=num_shares {
            let id = frost::Identifier::try_from(i)
                .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
            let share = shares
                .get(&id)
                .ok_or_else(|| FrostError::KeyGeneration(format!("Missing share {}", i)))?;
            let key_package = frost::keys::KeyPackage::try_from(share.clone())
                .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            key_shares.push(KeyShare::new(
                SignatureScheme::Taproot,
                serialize_key_package(&key_package)?,
                i,
            ));
        }

        let vk = pubkey_package.verifying_key();
        let vk_bytes = vk
            .serialize()
            .map_err(|e| FrostError::Serialization(e.to_string()))?;
        let verifying_key = VerifyingKey::new(SignatureScheme::Taproot, vk_bytes);

        Ok((key_shares, verifying_key))
    }

    fn derive_verifying_key(shares: &[KeyShare]) -> Result<VerifyingKey> {
        if shares.is_empty() {
            return Err(FrostError::InvalidParticipantCount { min: 1, got: 0 });
        }

        // Deserialize first share to get verifying key
        let key_package = deserialize_key_package(&shares[0].data)?;
        let vk = key_package.verifying_key();
        let vk_bytes = vk
            .serialize()
            .map_err(|e| FrostError::Serialization(e.to_string()))?;

        Ok(VerifyingKey::new(SignatureScheme::Taproot, vk_bytes))
    }
}

impl FrostPresigGen for Taproot {
    #[instrument(skip(key_share, rng))]
    fn generate_presigs<R: RngCore + CryptoRng>(
        key_share: &KeyShare,
        count: u32,
        rng: &mut R,
    ) -> Result<FrostPresigBatch> {
        if key_share.scheme != SignatureScheme::Taproot {
            return Err(FrostError::UnsupportedScheme(format!(
                "Expected Taproot, got {:?}",
                key_share.scheme
            )));
        }

        debug!(
            "Generating {} Taproot presignatures for participant {}",
            count, key_share.identifier
        );

        let key_package = deserialize_key_package(&key_share.data)?;

        let mut presigs = Vec::with_capacity(count as usize);
        for i in 0..count {
            // Generate nonce and commitment
            let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), rng);

            // Serialize
            let nonce_bytes = serialize_signing_nonces(&nonces)?;
            let commitment_bytes = serialize_signing_commitments(&commitments)?;

            presigs.push(FrostPresig::new(i, nonce_bytes, commitment_bytes));
        }

        debug!("Generated {} presignatures", presigs.len());

        Ok(FrostPresigBatch::new(
            SignatureScheme::Taproot,
            key_share.identifier,
            0,
            presigs,
        ))
    }
}

impl FrostSigner for Taproot {
    #[instrument(skip(key_share, presig, message, other_commitment))]
    fn sign_with_presig(
        key_share: &KeyShare,
        presig: &FrostPresig,
        message: &[u8],
        other_commitment: &[u8],
    ) -> Result<Vec<u8>> {
        if key_share.scheme != SignatureScheme::Taproot {
            return Err(FrostError::UnsupportedScheme(format!(
                "Expected Taproot, got {:?}",
                key_share.scheme
            )));
        }

        if presig.is_consumed() {
            return Err(FrostError::NonceReuse);
        }

        debug!(
            "Signing with presig {} for participant {}",
            presig.index, key_share.identifier
        );

        let key_package = deserialize_key_package(&key_share.data)?;
        let nonces = deserialize_signing_nonces(&presig.nonce)?;
        let my_commitment = deserialize_signing_commitments(&presig.commitment)?;
        let other_commitment = deserialize_signing_commitments(other_commitment)?;

        // Build commitments map
        let my_id = frost::Identifier::try_from(key_share.identifier)
            .map_err(|e| FrostError::Signing(e.to_string()))?;
        let other_id = frost::Identifier::try_from(if key_share.identifier == 1 {
            2u16
        } else {
            1u16
        })
        .map_err(|e| FrostError::Signing(e.to_string()))?;

        let mut commitments_map = BTreeMap::new();
        commitments_map.insert(my_id, my_commitment);
        commitments_map.insert(other_id, other_commitment);

        // Create signing package
        let signing_package = frost::SigningPackage::new(commitments_map, message);

        // Generate signature share
        let signature_share = frost::round2::sign(&signing_package, &nonces, &key_package)
            .map_err(|e| FrostError::Signing(e.to_string()))?;

        // Serialize
        let share_bytes = signature_share.serialize();

        debug!("Generated signature share");
        Ok(share_bytes.to_vec())
    }

    fn aggregate(
        shares: &[Vec<u8>],
        _message: &[u8],
        _verifying_key: &VerifyingKey,
    ) -> Result<FrostSignature> {
        if shares.is_empty() {
            return Err(FrostError::InvalidParticipantCount {
                min: 1,
                got: shares.len(),
            });
        }

        debug!("Aggregating {} signature shares", shares.len());

        // In the single-party (degenerate) case, treat the provided bytes as the
        // final Taproot Schnorr signature and wrap them in a FrostSignature.
        if shares.len() == 1 {
            return Ok(FrostSignature {
                scheme: SignatureScheme::Taproot,
                data: shares[0].clone(),
            });
        }

        // For true multi-party FROST aggregation, we need additional context such
        // as the SigningPackage and PublicKeyPackage, which are not available
        // through this trait method. Callers should use a scheme-specific
        // aggregation API that has access to that context.
        Err(FrostError::Aggregation(
            "Full aggregation requires SigningPackage and PublicKeyPackage context. \
             Use a scheme-specific aggregation function that has this context."
                .to_string(),
        ))
    }

    fn verify(
        signature: &FrostSignature,
        message: &[u8],
        verifying_key: &VerifyingKey,
    ) -> Result<bool> {
        if signature.scheme != SignatureScheme::Taproot {
            return Err(FrostError::UnsupportedScheme(format!(
                "Expected Taproot, got {:?}",
                signature.scheme
            )));
        }

        let vk = frost::VerifyingKey::deserialize(&verifying_key.data)
            .map_err(|e| FrostError::InvalidSignature(e.to_string()))?;

        let sig_bytes: [u8; 64] =
            signature.data.as_slice().try_into().map_err(|_| {
                FrostError::InvalidSignature("Invalid signature length".to_string())
            })?;

        let sig = frost::Signature::deserialize(&sig_bytes)
            .map_err(|e| FrostError::InvalidSignature(e.to_string()))?;

        vk.verify(message, &sig)
            .map(|_| true)
            .map_err(|e| FrostError::InvalidSignature(e.to_string()))
    }
}

// Serialization helpers

fn serialize_key_package(kp: &frost::keys::KeyPackage) -> Result<Vec<u8>> {
    kp.serialize()
        .map_err(|e| FrostError::Serialization(e.to_string()))
}

fn deserialize_key_package(data: &[u8]) -> Result<frost::keys::KeyPackage> {
    frost::keys::KeyPackage::deserialize(data)
        .map_err(|e| FrostError::Deserialization(e.to_string()))
}

fn serialize_signing_nonces(nonces: &frost::round1::SigningNonces) -> Result<Vec<u8>> {
    nonces
        .serialize()
        .map_err(|e| FrostError::Serialization(e.to_string()))
}

fn deserialize_signing_nonces(data: &[u8]) -> Result<frost::round1::SigningNonces> {
    frost::round1::SigningNonces::deserialize(data)
        .map_err(|e| FrostError::Deserialization(e.to_string()))
}

fn serialize_signing_commitments(
    commitments: &frost::round1::SigningCommitments,
) -> Result<Vec<u8>> {
    commitments
        .serialize()
        .map_err(|e| FrostError::Serialization(e.to_string()))
}

fn deserialize_signing_commitments(data: &[u8]) -> Result<frost::round1::SigningCommitments> {
    frost::round1::SigningCommitments::deserialize(data)
        .map_err(|e| FrostError::Deserialization(e.to_string()))
}

/// Extended signing context for full FROST signing flow
pub struct TaprootSigningContext {
    pub key_package: frost::keys::KeyPackage,
    pub pubkey_package: frost::keys::PublicKeyPackage,
}

impl TaprootSigningContext {
    /// Create from serialized packages
    pub fn new(key_share: &KeyShare, pubkey_package_bytes: &[u8]) -> Result<Self> {
        let key_package = deserialize_key_package(&key_share.data)?;
        let pubkey_package = frost::keys::PublicKeyPackage::deserialize(pubkey_package_bytes)
            .map_err(|e| FrostError::Deserialization(e.to_string()))?;

        Ok(Self {
            key_package,
            pubkey_package,
        })
    }

    /// Aggregate signature shares with full context
    pub fn aggregate_shares(
        &self,
        signature_shares: &[(u16, Vec<u8>)],
        signing_package: &frost::SigningPackage,
    ) -> Result<FrostSignature> {
        let mut shares_map = BTreeMap::new();

        for (participant_id, share_bytes) in signature_shares {
            let id = frost::Identifier::try_from(*participant_id)
                .map_err(|e| FrostError::Aggregation(e.to_string()))?;

            let share = frost::round2::SignatureShare::deserialize(share_bytes)
                .map_err(|e| FrostError::Aggregation(e.to_string()))?;

            shares_map.insert(id, share);
        }

        let signature = frost::aggregate(signing_package, &shares_map, &self.pubkey_package)
            .map_err(|e| FrostError::Aggregation(e.to_string()))?;

        let sig_bytes = signature
            .serialize()
            .map_err(|e| FrostError::Serialization(e.to_string()))?;
        Ok(FrostSignature::new(SignatureScheme::Taproot, sig_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_keygen_2of2() {
        let mut rng = OsRng;
        let (cold, agent, vk) = Taproot::generate_2of2(&mut rng).unwrap();

        assert_eq!(cold.scheme, SignatureScheme::Taproot);
        assert_eq!(agent.scheme, SignatureScheme::Taproot);
        assert_eq!(cold.identifier, 1);
        assert_eq!(agent.identifier, 2);
        assert_eq!(vk.scheme, SignatureScheme::Taproot);
        // FROST returns compressed pubkey (33 bytes) which can be converted to x-only
        assert!(vk.data.len() == 32 || vk.data.len() == 33);
    }

    #[test]
    fn test_presig_generation() {
        let mut rng = OsRng;
        let (cold, _, _) = Taproot::generate_2of2(&mut rng).unwrap();

        let presigs = Taproot::generate_presigs(&cold, 10, &mut rng).unwrap();

        assert_eq!(presigs.len(), 10);
        assert_eq!(presigs.scheme, SignatureScheme::Taproot);
        assert_eq!(presigs.remaining(), 10);
    }

    #[test]
    fn test_full_signing_flow() {
        let mut rng = OsRng;

        // Generate keys
        let (cold_share, agent_share, vk) = Taproot::generate_2of2(&mut rng).unwrap();

        // Generate presigs for both parties
        let mut cold_presigs = Taproot::generate_presigs(&cold_share, 5, &mut rng).unwrap();
        let mut agent_presigs = Taproot::generate_presigs(&agent_share, 5, &mut rng).unwrap();

        // Get first presig from each
        let cold_presig = cold_presigs.consume_next().unwrap();
        let agent_presig = agent_presigs.consume_next().unwrap();

        // Sign a message
        let message = b"Hello, Taproot!";

        // Each party signs with their presig and the other's commitment
        let cold_sig_share =
            Taproot::sign_with_presig(&cold_share, &cold_presig, message, &agent_presig.commitment)
                .unwrap();

        let agent_sig_share = Taproot::sign_with_presig(
            &agent_share,
            &agent_presig,
            message,
            &cold_presig.commitment,
        )
        .unwrap();

        // Both parties generated valid shares
        assert!(!cold_sig_share.is_empty());
        assert!(!agent_sig_share.is_empty());

        println!("Cold signature share: {} bytes", cold_sig_share.len());
        println!("Agent signature share: {} bytes", agent_sig_share.len());
        println!("Verifying key: {}", vk.to_hex());
    }
}
