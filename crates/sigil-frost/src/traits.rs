//! Traits for FROST cipher suite implementations

use crate::{FrostPresigBatch, FrostSignature, KeyShare, Result, SignatureScheme, VerifyingKey};

/// Trait for FROST cipher suite implementations
pub trait FrostCipherSuite: Sized {
    /// The signature scheme identifier
    const SCHEME: SignatureScheme;

    /// Size of a serialized public key in bytes
    const PUBLIC_KEY_SIZE: usize;

    /// Size of a serialized signature in bytes
    const SIGNATURE_SIZE: usize;

    /// Size of a serialized nonce in bytes
    const NONCE_SIZE: usize;

    /// Size of a serialized commitment in bytes
    const COMMITMENT_SIZE: usize;
}

/// Trait for FROST key generation
pub trait FrostKeyGen: FrostCipherSuite {
    /// Output of key generation
    type KeyGenOutput;

    /// Generate key shares for a 2-of-2 threshold scheme
    ///
    /// Returns (cold_share, agent_share, verifying_key)
    fn generate_2of2<R: rand::RngCore + rand::CryptoRng>(
        rng: &mut R,
    ) -> Result<(KeyShare, KeyShare, VerifyingKey)>;

    /// Generate key shares for a t-of-n threshold scheme
    ///
    /// # Arguments
    /// * `threshold` - Minimum signers required (t)
    /// * `num_shares` - Total number of shares to generate (n)
    /// * `rng` - Cryptographic random number generator
    ///
    /// Returns (shares, verifying_key)
    fn generate_shares<R: rand::RngCore + rand::CryptoRng>(
        threshold: u16,
        num_shares: u16,
        rng: &mut R,
    ) -> Result<(Vec<KeyShare>, VerifyingKey)>;

    /// Derive the verifying key from key shares
    fn derive_verifying_key(shares: &[KeyShare]) -> Result<VerifyingKey>;
}

/// Trait for FROST presignature (nonce) generation
pub trait FrostPresigGen: FrostCipherSuite {
    /// Generate a batch of presignatures (nonces) for offline storage
    ///
    /// # Arguments
    /// * `key_share` - The participant's key share
    /// * `count` - Number of presignatures to generate
    /// * `rng` - Cryptographic random number generator
    ///
    /// Returns a batch of presignatures that can be stored on disk
    fn generate_presigs<R: rand::RngCore + rand::CryptoRng>(
        key_share: &KeyShare,
        count: u32,
        rng: &mut R,
    ) -> Result<FrostPresigBatch>;
}

/// Trait for FROST signing operations
pub trait FrostSigner: FrostCipherSuite {
    /// Generate a signature share using a presignature
    ///
    /// # Arguments
    /// * `key_share` - The participant's key share
    /// * `presig` - The presignature (nonce) to use
    /// * `message` - The message to sign
    /// * `other_commitment` - The other participant's commitment for this presig
    ///
    /// Returns a signature share
    fn sign_with_presig(
        key_share: &KeyShare,
        presig: &crate::FrostPresig,
        message: &[u8],
        other_commitment: &[u8],
    ) -> Result<Vec<u8>>;

    /// Aggregate signature shares into a complete signature
    ///
    /// # Arguments
    /// * `shares` - The signature shares from all participants
    /// * `message` - The message that was signed
    /// * `verifying_key` - The group's verifying key
    ///
    /// Returns the complete signature
    fn aggregate(
        shares: &[Vec<u8>],
        message: &[u8],
        verifying_key: &VerifyingKey,
    ) -> Result<FrostSignature>;

    /// Verify a signature
    ///
    /// # Arguments
    /// * `signature` - The signature to verify
    /// * `message` - The message that was signed
    /// * `verifying_key` - The public key to verify against
    fn verify(
        signature: &FrostSignature,
        message: &[u8],
        verifying_key: &VerifyingKey,
    ) -> Result<bool>;
}

/// Combined trait for full FROST functionality
pub trait Frost: FrostKeyGen + FrostPresigGen + FrostSigner {}

// Blanket implementation
impl<T: FrostKeyGen + FrostPresigGen + FrostSigner> Frost for T {}
