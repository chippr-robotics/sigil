//! Ed25519 support via FROST
//!
//! This module implements FROST threshold signatures for Ed25519,
//! enabling threshold signing for Solana, Cosmos, Near, Polkadot, and other
//! Ed25519-based blockchains.

use crate::{
    error::{FrostError, Result},
    presig::{FrostPresig, FrostPresigBatch},
    traits::{FrostCipherSuite, FrostKeyGen, FrostPresigGen, FrostSigner},
    FrostSignature, KeyShare, SignatureScheme, VerifyingKey,
};
use frost_ed25519 as frost;
use rand::{CryptoRng, RngCore};
use std::collections::BTreeMap;
use tracing::{debug, instrument};

/// Ed25519 FROST implementation
pub struct Ed25519;

impl FrostCipherSuite for Ed25519 {
    const SCHEME: SignatureScheme = SignatureScheme::Ed25519;
    const PUBLIC_KEY_SIZE: usize = 32;
    const SIGNATURE_SIZE: usize = 64;
    const NONCE_SIZE: usize = 64;
    const COMMITMENT_SIZE: usize = 64;
}

impl FrostKeyGen for Ed25519 {
    type KeyGenOutput = (
        frost::keys::KeyPackage,
        frost::keys::KeyPackage,
        frost::keys::PublicKeyPackage,
    );

    #[instrument(skip(rng))]
    fn generate_2of2<R: RngCore + CryptoRng>(
        rng: &mut R,
    ) -> Result<(KeyShare, KeyShare, VerifyingKey)> {
        debug!("Generating 2-of-2 Ed25519 FROST key shares");

        let (shares, pubkey_package) =
            frost::keys::generate_with_dealer(2, 2, frost::keys::IdentifierList::Default, rng)
                .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

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

        let key_package1 = frost::keys::KeyPackage::try_from(share1.clone())
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
        let key_package2 = frost::keys::KeyPackage::try_from(share2.clone())
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

        let cold_share = KeyShare::new(
            SignatureScheme::Ed25519,
            serialize_key_package(&key_package1)?,
            1,
        );
        let agent_share = KeyShare::new(
            SignatureScheme::Ed25519,
            serialize_key_package(&key_package2)?,
            2,
        );

        let vk = pubkey_package.verifying_key();
        let vk_bytes = vk
            .serialize()
            .map_err(|e| FrostError::Serialization(e.to_string()))?;
        let verifying_key = VerifyingKey::new(SignatureScheme::Ed25519, vk_bytes);

        debug!(
            "Generated Ed25519 keys, verifying key: {}",
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
                SignatureScheme::Ed25519,
                serialize_key_package(&key_package)?,
                i,
            ));
        }

        let vk = pubkey_package.verifying_key();
        let vk_bytes = vk
            .serialize()
            .map_err(|e| FrostError::Serialization(e.to_string()))?;
        let verifying_key = VerifyingKey::new(SignatureScheme::Ed25519, vk_bytes);

        Ok((key_shares, verifying_key))
    }

    fn derive_verifying_key(shares: &[KeyShare]) -> Result<VerifyingKey> {
        if shares.is_empty() {
            return Err(FrostError::InvalidParticipantCount { min: 1, got: 0 });
        }

        let key_package = deserialize_key_package(&shares[0].data)?;
        let vk = key_package.verifying_key();
        let vk_bytes = vk
            .serialize()
            .map_err(|e| FrostError::Serialization(e.to_string()))?;

        Ok(VerifyingKey::new(SignatureScheme::Ed25519, vk_bytes))
    }
}

impl FrostPresigGen for Ed25519 {
    #[instrument(skip(key_share, rng))]
    fn generate_presigs<R: RngCore + CryptoRng>(
        key_share: &KeyShare,
        count: u32,
        rng: &mut R,
    ) -> Result<FrostPresigBatch> {
        if key_share.scheme != SignatureScheme::Ed25519 {
            return Err(FrostError::UnsupportedScheme(format!(
                "Expected Ed25519, got {:?}",
                key_share.scheme
            )));
        }

        debug!(
            "Generating {} Ed25519 presignatures for participant {}",
            count, key_share.identifier
        );

        let key_package = deserialize_key_package(&key_share.data)?;

        let mut presigs = Vec::with_capacity(count as usize);
        for i in 0..count {
            let (nonces, commitments) = frost::round1::commit(key_package.signing_share(), rng);

            let nonce_bytes = serialize_signing_nonces(&nonces)?;
            let commitment_bytes = serialize_signing_commitments(&commitments)?;

            presigs.push(FrostPresig::new(i, nonce_bytes, commitment_bytes));
        }

        debug!("Generated {} presignatures", presigs.len());

        Ok(FrostPresigBatch::new(
            SignatureScheme::Ed25519,
            key_share.identifier,
            0,
            presigs,
        ))
    }
}

impl FrostSigner for Ed25519 {
    #[instrument(skip(key_share, presig, message, other_commitment))]
    fn sign_with_presig(
        key_share: &KeyShare,
        presig: &FrostPresig,
        message: &[u8],
        other_commitment: &[u8],
    ) -> Result<Vec<u8>> {
        if key_share.scheme != SignatureScheme::Ed25519 {
            return Err(FrostError::UnsupportedScheme(format!(
                "Expected Ed25519, got {:?}",
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

        let signing_package = frost::SigningPackage::new(commitments_map, message);

        let signature_share = frost::round2::sign(&signing_package, &nonces, &key_package)
            .map_err(|e| FrostError::Signing(e.to_string()))?;

        let share_bytes = signature_share.serialize();

        debug!("Generated signature share");
        Ok(share_bytes.to_vec())
    }

    fn aggregate(
        shares: &[Vec<u8>],
        _message: &[u8],
        _verifying_key: &VerifyingKey,
    ) -> Result<FrostSignature> {
        if shares.len() < 2 {
            return Err(FrostError::InvalidParticipantCount {
                min: 2,
                got: shares.len(),
            });
        }

        Err(FrostError::Aggregation(
            "Full aggregation requires SigningPackage and PublicKeyPackage context. \
             Use Ed25519SigningContext::aggregate_shares instead."
                .to_string(),
        ))
    }

    fn verify(
        signature: &FrostSignature,
        message: &[u8],
        verifying_key: &VerifyingKey,
    ) -> Result<bool> {
        if signature.scheme != SignatureScheme::Ed25519 {
            return Err(FrostError::UnsupportedScheme(format!(
                "Expected Ed25519, got {:?}",
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

/// Extended signing context for full Ed25519 FROST signing flow
pub struct Ed25519SigningContext {
    pub key_package: frost::keys::KeyPackage,
    pub pubkey_package: frost::keys::PublicKeyPackage,
}

impl Ed25519SigningContext {
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
        Ok(FrostSignature::new(SignatureScheme::Ed25519, sig_bytes))
    }
}

/// Utility functions for Solana-specific operations
pub mod solana {
    use super::*;

    /// Convert Ed25519 verifying key to Solana public key format (base58)
    pub fn to_solana_pubkey(vk: &VerifyingKey) -> Result<String> {
        if vk.scheme != SignatureScheme::Ed25519 {
            return Err(FrostError::UnsupportedScheme(
                "Expected Ed25519 for Solana".to_string(),
            ));
        }

        // Solana uses base58 for public keys
        Ok(bs58_encode(&vk.data))
    }

    /// Convert signature to Solana format (base58)
    pub fn to_solana_signature(sig: &FrostSignature) -> Result<String> {
        if sig.scheme != SignatureScheme::Ed25519 {
            return Err(FrostError::UnsupportedScheme(
                "Expected Ed25519 for Solana".to_string(),
            ));
        }

        Ok(bs58_encode(&sig.data))
    }

    fn bs58_encode(data: &[u8]) -> String {
        const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

        if data.is_empty() {
            return String::new();
        }

        // Count leading zeros
        let zeros = data.iter().take_while(|&&x| x == 0).count();

        // Convert to base58
        let mut digits: Vec<u8> = Vec::with_capacity(data.len() * 138 / 100 + 1);

        for &byte in data.iter() {
            let mut carry = byte as u32;
            for digit in digits.iter_mut() {
                carry += (*digit as u32) * 256;
                *digit = (carry % 58) as u8;
                carry /= 58;
            }
            while carry > 0 {
                digits.push((carry % 58) as u8);
                carry /= 58;
            }
        }

        // Add leading '1's for leading zeros
        let mut result = String::with_capacity(zeros + digits.len());
        for _ in 0..zeros {
            result.push('1');
        }

        // Convert digits to characters (reverse order)
        for &digit in digits.iter().rev() {
            result.push(ALPHABET[digit as usize] as char);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_keygen_2of2() {
        let mut rng = OsRng;
        let (cold, agent, vk) = Ed25519::generate_2of2(&mut rng).unwrap();

        assert_eq!(cold.scheme, SignatureScheme::Ed25519);
        assert_eq!(agent.scheme, SignatureScheme::Ed25519);
        assert_eq!(cold.identifier, 1);
        assert_eq!(agent.identifier, 2);
        assert_eq!(vk.scheme, SignatureScheme::Ed25519);
        assert_eq!(vk.data.len(), 32);
    }

    #[test]
    fn test_presig_generation() {
        let mut rng = OsRng;
        let (cold, _, _) = Ed25519::generate_2of2(&mut rng).unwrap();

        let presigs = Ed25519::generate_presigs(&cold, 10, &mut rng).unwrap();

        assert_eq!(presigs.len(), 10);
        assert_eq!(presigs.scheme, SignatureScheme::Ed25519);
        assert_eq!(presigs.remaining(), 10);
    }

    #[test]
    fn test_full_signing_flow() {
        let mut rng = OsRng;

        let (cold_share, agent_share, vk) = Ed25519::generate_2of2(&mut rng).unwrap();

        let mut cold_presigs = Ed25519::generate_presigs(&cold_share, 5, &mut rng).unwrap();
        let mut agent_presigs = Ed25519::generate_presigs(&agent_share, 5, &mut rng).unwrap();

        let cold_presig = cold_presigs.consume_next().unwrap();
        let agent_presig = agent_presigs.consume_next().unwrap();

        let message = b"Hello, Solana!";

        let cold_sig_share =
            Ed25519::sign_with_presig(&cold_share, &cold_presig, message, &agent_presig.commitment)
                .unwrap();

        let agent_sig_share = Ed25519::sign_with_presig(
            &agent_share,
            &agent_presig,
            message,
            &cold_presig.commitment,
        )
        .unwrap();

        assert!(!cold_sig_share.is_empty());
        assert!(!agent_sig_share.is_empty());

        println!(
            "Ed25519 cold signature share: {} bytes",
            cold_sig_share.len()
        );
        println!(
            "Ed25519 agent signature share: {} bytes",
            agent_sig_share.len()
        );
        println!("Ed25519 verifying key: {}", vk.to_hex());

        // Test Solana format
        let solana_pubkey = solana::to_solana_pubkey(&vk).unwrap();
        println!("Solana public key (base58): {}", solana_pubkey);
    }
}
