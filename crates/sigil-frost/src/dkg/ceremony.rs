//! DKG ceremony implementation for each FROST cipher suite

use std::collections::BTreeMap;
use std::marker::PhantomData;

use rand::rngs::OsRng;

use crate::error::FrostError;
use crate::{KeyShare, SignatureScheme, VerifyingKey};

use super::types::{
    compute_round1_hash, DkgConfig, DkgRound1Package, DkgRound2Package, DkgState, ParticipantId,
};
use super::DkgResult;

/// Output of a successful DKG ceremony
#[derive(Debug, Clone)]
pub struct DkgOutput<S> {
    /// The participant's secret key share
    pub key_share: KeyShare,

    /// The group verifying key (public key)
    pub verifying_key: VerifyingKey,

    /// Verification hash (should match across all participants)
    pub verification_hash: [u8; 32],

    /// Phantom data for the scheme type
    _marker: PhantomData<S>,
}

impl<S> DkgOutput<S> {
    /// Create a new DKG output
    pub fn new(key_share: KeyShare, verifying_key: VerifyingKey) -> Self {
        // Compute verification hash
        let verification_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(verifying_key.data.as_slice());
            hasher.finalize().into()
        };

        Self {
            key_share,
            verifying_key,
            verification_hash,
            _marker: PhantomData,
        }
    }
}

/// Key package from DKG (scheme-agnostic wrapper)
#[derive(Debug, Clone)]
pub struct DkgKeyPackage {
    /// The key share (secret)
    pub key_share: KeyShare,

    /// The verifying key (public)
    pub verifying_key: VerifyingKey,

    /// Participant ID
    pub participant_id: ParticipantId,

    /// Verification hash
    pub verification_hash: [u8; 32],
}

/// DKG ceremony state machine
pub struct DkgCeremony<S> {
    /// Configuration
    pub config: DkgConfig,

    /// Current state
    pub state: DkgState,

    /// Our Round 1 secret state (scheme-specific, serialized)
    round1_secret: Option<Vec<u8>>,

    /// Our Round 1 package
    our_round1: Option<DkgRound1Package>,

    /// Received Round 1 packages (by sender ID)
    received_round1: BTreeMap<ParticipantId, DkgRound1Package>,

    /// Our Round 2 packages (by recipient ID)
    our_round2: BTreeMap<ParticipantId, DkgRound2Package>,

    /// Received Round 2 packages (by sender ID)
    received_round2: BTreeMap<ParticipantId, DkgRound2Package>,

    /// Phantom for scheme type
    _marker: PhantomData<S>,
}

impl<S> DkgCeremony<S> {
    /// Create a new DKG ceremony
    pub fn new(config: DkgConfig) -> DkgResult<Self> {
        config
            .validate()
            .map_err(|e| FrostError::InvalidParameters(e.to_string()))?;

        Ok(Self {
            config,
            state: DkgState::Initialized,
            round1_secret: None,
            our_round1: None,
            received_round1: BTreeMap::new(),
            our_round2: BTreeMap::new(),
            received_round2: BTreeMap::new(),
            _marker: PhantomData,
        })
    }

    /// Get our participant ID
    pub fn participant_id(&self) -> ParticipantId {
        self.config.participant_id
    }

    /// Check if we have all required Round 1 packages
    pub fn has_all_round1(&self) -> bool {
        // We need packages from all other participants
        let expected = self.config.max_signers as usize - 1;
        self.received_round1.len() >= expected
    }

    /// Check if we have all required Round 2 packages
    pub fn has_all_round2(&self) -> bool {
        // We need packages from all other participants
        let expected = self.config.max_signers as usize - 1;
        self.received_round2.len() >= expected
    }

    /// Add a received Round 1 package
    pub fn add_round1(&mut self, package: DkgRound1Package) -> DkgResult<()> {
        // Validate package
        if package.sender_id == self.config.participant_id {
            return Err(FrostError::InvalidParameters(
                "Cannot add our own Round 1 package".to_string(),
            ));
        }

        if package.scheme != self.config.scheme {
            return Err(FrostError::InvalidParameters(format!(
                "Scheme mismatch: expected {:?}, got {:?}",
                self.config.scheme, package.scheme
            )));
        }

        if package.min_signers != self.config.min_signers
            || package.max_signers != self.config.max_signers
        {
            return Err(FrostError::InvalidParameters(
                "Threshold parameters mismatch".to_string(),
            ));
        }

        self.received_round1.insert(package.sender_id, package);
        Ok(())
    }

    /// Add a received Round 2 package
    pub fn add_round2(&mut self, package: DkgRound2Package) -> DkgResult<()> {
        // Validate package
        if package.recipient_id != self.config.participant_id {
            return Err(FrostError::InvalidParameters(
                "Round 2 package not for us".to_string(),
            ));
        }

        if package.scheme != self.config.scheme {
            return Err(FrostError::InvalidParameters(format!(
                "Scheme mismatch: expected {:?}, got {:?}",
                self.config.scheme, package.scheme
            )));
        }

        // Verify Round 1 hash
        let our_r1 = self
            .our_round1
            .as_ref()
            .ok_or_else(|| FrostError::InvalidState("Round 1 not generated".to_string()))?;

        let all_r1: Vec<_> = std::iter::once(our_r1.clone())
            .chain(self.received_round1.values().cloned())
            .collect();

        let expected_hash = compute_round1_hash(&all_r1);
        if package.round1_hash != expected_hash {
            return Err(FrostError::InvalidParameters(
                "Round 1 hash mismatch - ceremony state diverged".to_string(),
            ));
        }

        self.received_round2.insert(package.sender_id, package);
        Ok(())
    }

    /// Get our Round 1 package (after generation)
    pub fn our_round1_package(&self) -> Option<&DkgRound1Package> {
        self.our_round1.as_ref()
    }

    /// Get our Round 2 packages (after generation)
    pub fn our_round2_packages(&self) -> impl Iterator<Item = &DkgRound2Package> {
        self.our_round2.values()
    }
}

// ============================================================================
// Taproot DKG Implementation
// ============================================================================

#[cfg(feature = "taproot")]
pub mod taproot {
    use super::*;
    use frost_secp256k1_tr::keys::dkg as frost_dkg;
    use frost_secp256k1_tr::Identifier;

    /// Convert FROST Identifier to u16
    ///
    /// This relies on the assumption that `Identifier::serialize()` returns a
    /// 32-byte big-endian scalar encoding of the participant ID. For small
    /// integer IDs (u16), the value is expected to appear in the last 2 bytes.
    /// In debug builds we verify this assumption by reconstructing an
    /// `Identifier` from the derived `u16` and checking that it matches `id`.
    fn identifier_to_u16(id: &Identifier) -> u16 {
        // Serialize the identifier - expected to be a 32-byte scalar in
        // big-endian format.
        let bytes = id.serialize();

        debug_assert!(
            bytes.len() == 32,
            "Identifier serialization must be 32 bytes, got {}",
            bytes.len()
        );

        // The scalar is big-endian, so for small values (like participant IDs)
        // the u16 value resides in the last 2 bytes.
        let value = u16::from_be_bytes([bytes[30], bytes[31]]);

        // In debug builds, verify that this conversion is consistent with how
        // the library constructs an Identifier from a u16.
        #[cfg(debug_assertions)]
        {
            if let Ok(roundtrip_id) = Identifier::try_from(value) {
                debug_assert_eq!(
                    &roundtrip_id, id,
                    "Identifier serialization mismatch for participant id {}",
                    value
                );
            }
        }

        value
    }

    /// Taproot DKG marker type
    pub struct TaprootDkg;

    impl DkgCeremony<TaprootDkg> {
        /// Generate Round 1 package for Taproot
        pub fn generate_round1(&mut self) -> DkgResult<DkgRound1Package> {
            if self.state != DkgState::Initialized {
                return Err(FrostError::InvalidState(
                    "Round 1 already generated".to_string(),
                ));
            }

            let identifier = Identifier::try_from(self.config.participant_id)
                .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let (round1_secret, round1_package) = frost_dkg::part1(
                identifier,
                self.config.max_signers,
                self.config.min_signers,
                &mut OsRng,
            )
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            // Serialize the secret state for later use
            let secret_bytes = round1_secret
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            // Extract commitments from the package
            let pkg_bytes = round1_package
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            // Create our wrapper package
            let dkg_package = DkgRound1Package::new(
                self.config.participant_id,
                self.config.scheme,
                self.config.min_signers,
                self.config.max_signers,
                vec![], // Commitments extracted from frost_package
                vec![], // PoK extracted from frost_package
                pkg_bytes,
            );

            self.round1_secret = Some(secret_bytes);
            self.our_round1 = Some(dkg_package.clone());
            self.state = DkgState::Round1Generated;

            Ok(dkg_package)
        }

        /// Generate Round 2 packages for all other participants
        pub fn generate_round2(&mut self) -> DkgResult<Vec<DkgRound2Package>> {
            if self.state != DkgState::Round1Generated {
                return Err(FrostError::InvalidState(
                    "Round 1 not generated or Round 2 already generated".to_string(),
                ));
            }

            if !self.has_all_round1() {
                return Err(FrostError::InvalidState(
                    "Missing Round 1 packages from other participants".to_string(),
                ));
            }

            // Deserialize our Round 1 secret
            let secret_bytes = self
                .round1_secret
                .as_ref()
                .ok_or_else(|| FrostError::InvalidState("Round 1 secret missing".to_string()))?;

            let round1_secret = frost_dkg::round1::SecretPackage::deserialize(secret_bytes)
                .map_err(|e| FrostError::Deserialization(e.to_string()))?;

            // Deserialize received Round 1 packages
            let mut frost_round1_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round1 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

                let frost_pkg = frost_dkg::round1::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;

                frost_round1_packages.insert(identifier, frost_pkg);
            }

            // Generate Round 2
            let (round2_secret, round2_packages) =
                frost_dkg::part2(round1_secret, &frost_round1_packages)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            // Compute Round 1 transcript hash
            let our_r1 = self.our_round1.as_ref().unwrap();
            let all_r1: Vec<_> = std::iter::once(our_r1.clone())
                .chain(self.received_round1.values().cloned())
                .collect();
            let round1_hash = compute_round1_hash(&all_r1);

            // Store Round 2 secret (serialized)
            let secret_bytes = round2_secret
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;
            self.round1_secret = Some(secret_bytes);

            // Create wrapper packages for each recipient
            let mut result = Vec::new();
            for (recipient_id, frost_pkg) in round2_packages {
                let recipient_u16 = identifier_to_u16(&recipient_id);
                let pkg_bytes = frost_pkg
                    .serialize()
                    .map_err(|e| FrostError::Serialization(e.to_string()))?;

                let dkg_package = DkgRound2Package::new(
                    self.config.participant_id,
                    recipient_u16,
                    self.config.scheme,
                    round1_hash,
                    pkg_bytes,
                );

                self.our_round2.insert(recipient_u16, dkg_package.clone());
                result.push(dkg_package);
            }

            self.state = DkgState::Round2Generated;
            Ok(result)
        }

        /// Finalize the DKG ceremony
        pub fn finalize(mut self) -> DkgResult<DkgOutput<TaprootDkg>> {
            if self.state != DkgState::Round2Generated {
                return Err(FrostError::InvalidState(
                    "Round 2 not generated".to_string(),
                ));
            }

            if !self.has_all_round2() {
                return Err(FrostError::InvalidState(
                    "Missing Round 2 packages from other participants".to_string(),
                ));
            }

            // Deserialize Round 2 secret
            let secret_bytes = self
                .round1_secret
                .take()
                .ok_or_else(|| FrostError::InvalidState("Round 2 secret missing".to_string()))?;

            let round2_secret = frost_dkg::round2::SecretPackage::deserialize(&secret_bytes)
                .map_err(|e| FrostError::Deserialization(e.to_string()))?;

            // Deserialize Round 1 packages (needed for finalization)
            let mut frost_round1_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round1 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

                let frost_pkg = frost_dkg::round1::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;

                frost_round1_packages.insert(identifier, frost_pkg);
            }

            // Deserialize Round 2 packages
            let mut frost_round2_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round2 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

                let frost_pkg = frost_dkg::round2::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;

                frost_round2_packages.insert(identifier, frost_pkg);
            }

            // Finalize
            let (key_package, pubkey_package) = frost_dkg::part3(
                &round2_secret,
                &frost_round1_packages,
                &frost_round2_packages,
            )
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            // Convert to our types
            // SigningShare::serialize() returns Vec<u8> directly in FROST 2.x
            let key_share_bytes = key_package.signing_share().serialize();

            let verifying_key_bytes = pubkey_package
                .verifying_key()
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            let key_share = KeyShare {
                scheme: SignatureScheme::Taproot,
                data: key_share_bytes,
                identifier: self.config.participant_id,
            };

            let verifying_key = VerifyingKey {
                scheme: SignatureScheme::Taproot,
                data: verifying_key_bytes.to_vec(),
            };

            self.state = DkgState::Completed;

            Ok(DkgOutput::new(key_share, verifying_key))
        }
    }
}

// ============================================================================
// Ed25519 DKG Implementation
// ============================================================================

#[cfg(feature = "ed25519")]
pub mod ed25519 {
    use super::*;
    use frost_ed25519::keys::dkg as frost_dkg;
    use frost_ed25519::Identifier;

    /// Convert FROST Identifier to u16
    fn identifier_to_u16(id: &Identifier) -> u16 {
        let bytes = id.serialize();
        u16::from_le_bytes([bytes[0], bytes[1]])
    }

    /// Ed25519 DKG marker type
    pub struct Ed25519Dkg;

    impl DkgCeremony<Ed25519Dkg> {
        /// Generate Round 1 package for Ed25519
        pub fn generate_round1(&mut self) -> DkgResult<DkgRound1Package> {
            if self.state != DkgState::Initialized {
                return Err(FrostError::InvalidState(
                    "Round 1 already generated".to_string(),
                ));
            }

            let identifier = Identifier::try_from(self.config.participant_id)
                .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let (round1_secret, round1_package) = frost_dkg::part1(
                identifier,
                self.config.max_signers,
                self.config.min_signers,
                &mut OsRng,
            )
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let secret_bytes = round1_secret
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            let pkg_bytes = round1_package
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            let dkg_package = DkgRound1Package::new(
                self.config.participant_id,
                self.config.scheme,
                self.config.min_signers,
                self.config.max_signers,
                vec![],
                vec![],
                pkg_bytes,
            );

            self.round1_secret = Some(secret_bytes);
            self.our_round1 = Some(dkg_package.clone());
            self.state = DkgState::Round1Generated;

            Ok(dkg_package)
        }

        /// Generate Round 2 packages
        pub fn generate_round2(&mut self) -> DkgResult<Vec<DkgRound2Package>> {
            if self.state != DkgState::Round1Generated {
                return Err(FrostError::InvalidState(
                    "Round 1 not generated".to_string(),
                ));
            }

            if !self.has_all_round1() {
                return Err(FrostError::InvalidState(
                    "Missing Round 1 packages".to_string(),
                ));
            }

            let secret_bytes = self.round1_secret.as_ref().unwrap();
            let round1_secret = frost_dkg::round1::SecretPackage::deserialize(secret_bytes)
                .map_err(|e| FrostError::Deserialization(e.to_string()))?;

            let mut frost_round1_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round1 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
                let frost_pkg = frost_dkg::round1::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;
                frost_round1_packages.insert(identifier, frost_pkg);
            }

            let (round2_secret, round2_packages) =
                frost_dkg::part2(round1_secret, &frost_round1_packages)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let our_r1 = self.our_round1.as_ref().unwrap();
            let all_r1: Vec<_> = std::iter::once(our_r1.clone())
                .chain(self.received_round1.values().cloned())
                .collect();
            let round1_hash = compute_round1_hash(&all_r1);

            let secret_bytes = round2_secret
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;
            self.round1_secret = Some(secret_bytes);

            let mut result = Vec::new();
            for (recipient_id, frost_pkg) in round2_packages {
                let recipient_u16 = identifier_to_u16(&recipient_id);
                let pkg_bytes = frost_pkg
                    .serialize()
                    .map_err(|e| FrostError::Serialization(e.to_string()))?;

                let dkg_package = DkgRound2Package::new(
                    self.config.participant_id,
                    recipient_u16,
                    self.config.scheme,
                    round1_hash,
                    pkg_bytes,
                );

                self.our_round2.insert(recipient_u16, dkg_package.clone());
                result.push(dkg_package);
            }

            self.state = DkgState::Round2Generated;
            Ok(result)
        }

        /// Finalize the ceremony
        pub fn finalize(mut self) -> DkgResult<DkgOutput<Ed25519Dkg>> {
            if self.state != DkgState::Round2Generated {
                return Err(FrostError::InvalidState(
                    "Round 2 not generated".to_string(),
                ));
            }

            if !self.has_all_round2() {
                return Err(FrostError::InvalidState(
                    "Missing Round 2 packages".to_string(),
                ));
            }

            let secret_bytes = self.round1_secret.take().unwrap();
            let round2_secret = frost_dkg::round2::SecretPackage::deserialize(&secret_bytes)
                .map_err(|e| FrostError::Deserialization(e.to_string()))?;

            let mut frost_round1_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round1 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
                let frost_pkg = frost_dkg::round1::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;
                frost_round1_packages.insert(identifier, frost_pkg);
            }

            let mut frost_round2_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round2 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
                let frost_pkg = frost_dkg::round2::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;
                frost_round2_packages.insert(identifier, frost_pkg);
            }

            let (key_package, pubkey_package) = frost_dkg::part3(
                &round2_secret,
                &frost_round1_packages,
                &frost_round2_packages,
            )
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let key_share_bytes = key_package.signing_share().serialize();

            let verifying_key_bytes = pubkey_package
                .verifying_key()
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            let key_share = KeyShare {
                scheme: SignatureScheme::Ed25519,
                data: key_share_bytes,
                identifier: self.config.participant_id,
            };

            let verifying_key = VerifyingKey {
                scheme: SignatureScheme::Ed25519,
                data: verifying_key_bytes.to_vec(),
            };

            self.state = DkgState::Completed;

            Ok(DkgOutput::new(key_share, verifying_key))
        }
    }
}

// ============================================================================
// Ristretto255 DKG Implementation
// ============================================================================

#[cfg(feature = "ristretto255")]
pub mod ristretto255 {
    use super::*;
    use frost_ristretto255::keys::dkg as frost_dkg;
    use frost_ristretto255::Identifier;

    /// Convert FROST Identifier to u16
    fn identifier_to_u16(id: &Identifier) -> u16 {
        let bytes = id.serialize();
        u16::from_le_bytes([bytes[0], bytes[1]])
    }

    /// Ristretto255 DKG marker type
    pub struct Ristretto255Dkg;

    impl DkgCeremony<Ristretto255Dkg> {
        /// Generate Round 1 package
        pub fn generate_round1(&mut self) -> DkgResult<DkgRound1Package> {
            if self.state != DkgState::Initialized {
                return Err(FrostError::InvalidState(
                    "Round 1 already generated".to_string(),
                ));
            }

            let identifier = Identifier::try_from(self.config.participant_id)
                .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let (round1_secret, round1_package) = frost_dkg::part1(
                identifier,
                self.config.max_signers,
                self.config.min_signers,
                &mut OsRng,
            )
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let secret_bytes = round1_secret
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            let pkg_bytes = round1_package
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            let dkg_package = DkgRound1Package::new(
                self.config.participant_id,
                self.config.scheme,
                self.config.min_signers,
                self.config.max_signers,
                vec![],
                vec![],
                pkg_bytes,
            );

            self.round1_secret = Some(secret_bytes);
            self.our_round1 = Some(dkg_package.clone());
            self.state = DkgState::Round1Generated;

            Ok(dkg_package)
        }

        /// Generate Round 2 packages
        pub fn generate_round2(&mut self) -> DkgResult<Vec<DkgRound2Package>> {
            if self.state != DkgState::Round1Generated {
                return Err(FrostError::InvalidState(
                    "Round 1 not generated".to_string(),
                ));
            }

            if !self.has_all_round1() {
                return Err(FrostError::InvalidState(
                    "Missing Round 1 packages".to_string(),
                ));
            }

            let secret_bytes = self.round1_secret.as_ref().unwrap();
            let round1_secret = frost_dkg::round1::SecretPackage::deserialize(secret_bytes)
                .map_err(|e| FrostError::Deserialization(e.to_string()))?;

            let mut frost_round1_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round1 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
                let frost_pkg = frost_dkg::round1::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;
                frost_round1_packages.insert(identifier, frost_pkg);
            }

            let (round2_secret, round2_packages) =
                frost_dkg::part2(round1_secret, &frost_round1_packages)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let our_r1 = self.our_round1.as_ref().unwrap();
            let all_r1: Vec<_> = std::iter::once(our_r1.clone())
                .chain(self.received_round1.values().cloned())
                .collect();
            let round1_hash = compute_round1_hash(&all_r1);

            let secret_bytes = round2_secret
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;
            self.round1_secret = Some(secret_bytes);

            let mut result = Vec::new();
            for (recipient_id, frost_pkg) in round2_packages {
                let recipient_u16 = identifier_to_u16(&recipient_id);
                let pkg_bytes = frost_pkg
                    .serialize()
                    .map_err(|e| FrostError::Serialization(e.to_string()))?;

                let dkg_package = DkgRound2Package::new(
                    self.config.participant_id,
                    recipient_u16,
                    self.config.scheme,
                    round1_hash,
                    pkg_bytes,
                );

                self.our_round2.insert(recipient_u16, dkg_package.clone());
                result.push(dkg_package);
            }

            self.state = DkgState::Round2Generated;
            Ok(result)
        }

        /// Finalize the ceremony
        pub fn finalize(mut self) -> DkgResult<DkgOutput<Ristretto255Dkg>> {
            if self.state != DkgState::Round2Generated {
                return Err(FrostError::InvalidState(
                    "Round 2 not generated".to_string(),
                ));
            }

            if !self.has_all_round2() {
                return Err(FrostError::InvalidState(
                    "Missing Round 2 packages".to_string(),
                ));
            }

            let secret_bytes = self.round1_secret.take().unwrap();
            let round2_secret = frost_dkg::round2::SecretPackage::deserialize(&secret_bytes)
                .map_err(|e| FrostError::Deserialization(e.to_string()))?;

            let mut frost_round1_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round1 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
                let frost_pkg = frost_dkg::round1::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;
                frost_round1_packages.insert(identifier, frost_pkg);
            }

            let mut frost_round2_packages = BTreeMap::new();
            for (sender_id, pkg) in &self.received_round2 {
                let identifier = Identifier::try_from(*sender_id)
                    .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;
                let frost_pkg = frost_dkg::round2::Package::deserialize(&pkg.frost_package)
                    .map_err(|e| FrostError::Deserialization(e.to_string()))?;
                frost_round2_packages.insert(identifier, frost_pkg);
            }

            let (key_package, pubkey_package) = frost_dkg::part3(
                &round2_secret,
                &frost_round1_packages,
                &frost_round2_packages,
            )
            .map_err(|e| FrostError::KeyGeneration(e.to_string()))?;

            let key_share_bytes = key_package.signing_share().serialize();

            let verifying_key_bytes = pubkey_package
                .verifying_key()
                .serialize()
                .map_err(|e| FrostError::Serialization(e.to_string()))?;

            let key_share = KeyShare {
                scheme: SignatureScheme::Ristretto255,
                data: key_share_bytes,
                identifier: self.config.participant_id,
            };

            let verifying_key = VerifyingKey {
                scheme: SignatureScheme::Ristretto255,
                data: verifying_key_bytes.to_vec(),
            };

            self.state = DkgState::Completed;

            Ok(DkgOutput::new(key_share, verifying_key))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "taproot")]
    #[test]
    fn test_taproot_dkg_2of2() {
        use taproot::TaprootDkg;

        // Create ceremonies for both participants
        let config1 = DkgConfig::mother_2of2(SignatureScheme::Taproot);
        let config2 = DkgConfig::agent_2of2(SignatureScheme::Taproot);

        let mut ceremony1: DkgCeremony<TaprootDkg> = DkgCeremony::new(config1).unwrap();
        let mut ceremony2: DkgCeremony<TaprootDkg> = DkgCeremony::new(config2).unwrap();

        // Round 1
        let r1_pkg1 = ceremony1.generate_round1().unwrap();
        let r1_pkg2 = ceremony2.generate_round1().unwrap();

        // Exchange Round 1
        ceremony1.add_round1(r1_pkg2).unwrap();
        ceremony2.add_round1(r1_pkg1).unwrap();

        // Round 2
        let r2_pkgs1 = ceremony1.generate_round2().unwrap();
        let r2_pkgs2 = ceremony2.generate_round2().unwrap();

        // In 2-of-2, ceremony1 (participant 1) generates one package for participant 2
        // and ceremony2 (participant 2) generates one package for participant 1
        assert_eq!(
            r2_pkgs1.len(),
            1,
            "Ceremony1 should generate 1 Round 2 package"
        );
        assert_eq!(
            r2_pkgs2.len(),
            1,
            "Ceremony2 should generate 1 Round 2 package"
        );

        // Exchange Round 2 - ceremony2's package goes to ceremony1
        // The package from ceremony2 is for ceremony1 (recipient = 1)
        let pkg_for_1 = r2_pkgs2.into_iter().next().unwrap();
        assert_eq!(
            pkg_for_1.sender_id, 2,
            "Package should be from participant 2"
        );
        ceremony1.add_round2(pkg_for_1).unwrap();

        // The package from ceremony1 is for ceremony2 (recipient = 2)
        let pkg_for_2 = r2_pkgs1.into_iter().next().unwrap();
        assert_eq!(
            pkg_for_2.sender_id, 1,
            "Package should be from participant 1"
        );
        ceremony2.add_round2(pkg_for_2).unwrap();

        // Finalize
        let output1 = ceremony1.finalize().unwrap();
        let output2 = ceremony2.finalize().unwrap();

        // Both should have the same verifying key
        assert_eq!(output1.verifying_key.data, output2.verifying_key.data);
        assert_eq!(output1.verification_hash, output2.verification_hash);

        // But different key shares
        assert_ne!(output1.key_share.data, output2.key_share.data);
    }

    #[cfg(feature = "ed25519")]
    #[test]
    fn test_ed25519_dkg_2of2() {
        use ed25519::Ed25519Dkg;

        let config1 = DkgConfig::mother_2of2(SignatureScheme::Ed25519);
        let config2 = DkgConfig::agent_2of2(SignatureScheme::Ed25519);

        let mut ceremony1: DkgCeremony<Ed25519Dkg> = DkgCeremony::new(config1).unwrap();
        let mut ceremony2: DkgCeremony<Ed25519Dkg> = DkgCeremony::new(config2).unwrap();

        let r1_pkg1 = ceremony1.generate_round1().unwrap();
        let r1_pkg2 = ceremony2.generate_round1().unwrap();

        ceremony1.add_round1(r1_pkg2).unwrap();
        ceremony2.add_round1(r1_pkg1).unwrap();

        let r2_pkgs1 = ceremony1.generate_round2().unwrap();
        let r2_pkgs2 = ceremony2.generate_round2().unwrap();

        for pkg in r2_pkgs2 {
            if pkg.recipient_id == 1 {
                ceremony1.add_round2(pkg).unwrap();
            }
        }
        for pkg in r2_pkgs1 {
            if pkg.recipient_id == 2 {
                ceremony2.add_round2(pkg).unwrap();
            }
        }

        let output1 = ceremony1.finalize().unwrap();
        let output2 = ceremony2.finalize().unwrap();

        assert_eq!(output1.verifying_key.data, output2.verifying_key.data);
        assert_eq!(output1.verification_hash, output2.verification_hash);
    }

    #[cfg(feature = "ristretto255")]
    #[test]
    fn test_ristretto255_dkg_2of2() {
        use ristretto255::Ristretto255Dkg;

        let config1 = DkgConfig::mother_2of2(SignatureScheme::Ristretto255);
        let config2 = DkgConfig::agent_2of2(SignatureScheme::Ristretto255);

        let mut ceremony1: DkgCeremony<Ristretto255Dkg> = DkgCeremony::new(config1).unwrap();
        let mut ceremony2: DkgCeremony<Ristretto255Dkg> = DkgCeremony::new(config2).unwrap();

        let r1_pkg1 = ceremony1.generate_round1().unwrap();
        let r1_pkg2 = ceremony2.generate_round1().unwrap();

        ceremony1.add_round1(r1_pkg2).unwrap();
        ceremony2.add_round1(r1_pkg1).unwrap();

        let r2_pkgs1 = ceremony1.generate_round2().unwrap();
        let r2_pkgs2 = ceremony2.generate_round2().unwrap();

        for pkg in r2_pkgs2 {
            if pkg.recipient_id == 1 {
                ceremony1.add_round2(pkg).unwrap();
            }
        }
        for pkg in r2_pkgs1 {
            if pkg.recipient_id == 2 {
                ceremony2.add_round2(pkg).unwrap();
            }
        }

        let output1 = ceremony1.finalize().unwrap();
        let output2 = ceremony2.finalize().unwrap();

        assert_eq!(output1.verifying_key.data, output2.verifying_key.data);
        assert_eq!(output1.verification_hash, output2.verification_hash);
    }
}
