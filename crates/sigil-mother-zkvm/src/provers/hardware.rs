//! Hardware wallet derivation prover
//!
//! Proves:
//! 1. Signature is valid for the device's public key
//! 2. Shard is correctly derived as `SHA256(domain || signature)`
//! 3. Public key is correctly computed from shard

use k256::{
    ecdsa::{signature::Verifier, Signature, VerifyingKey},
    elliptic_curve::sec1::ToEncodedPoint,
    SecretKey,
};
use sha2::{Digest, Sha256};

#[cfg(feature = "sp1-prover")]
use super::HardwareProverTrait;
use crate::error::{Result, ZkvmError};
use crate::types::{HardwareInput, HardwareOutput};

/// Hardware derivation prover that can use mock or SP1 backend
pub struct HardwareProver;

impl HardwareProver {
    /// Execute the hardware derivation computation (used by both mock and SP1)
    ///
    /// This is the core computation that proves:
    /// 1. The signature is valid for the message and device public key
    /// 2. The shard is correctly derived as SHA256(domain || signature)
    /// 3. The shard's public key is correctly computed
    pub fn compute(input: &HardwareInput) -> Result<HardwareOutput> {
        // 1. Verify the signature
        Self::verify_signature(&input.device_pubkey, &input.message, &input.signature)?;

        // 2. Verify shard derivation: shard = SHA256(domain || signature)
        let expected_shard = {
            let mut hasher = Sha256::new();
            hasher.update(&input.domain);
            hasher.update(input.signature);
            let result: [u8; 32] = hasher.finalize().into();
            result
        };

        if expected_shard != input.derived_shard {
            return Err(ZkvmError::Crypto(
                "Shard derivation does not match expected".into(),
            ));
        }

        // 3. Compute shard's public key
        let shard_pubkey = derive_public_key(&input.derived_shard)?;

        // 4. Compute message hash for output
        let message_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&input.message);
            hasher.finalize().into()
        };

        Ok(HardwareOutput {
            shard_pubkey,
            device_pubkey: input.device_pubkey,
            message_hash,
            is_cold_shard: input.is_cold_shard,
        })
    }

    /// Verify an ECDSA signature from a hardware wallet
    ///
    /// The signature format is 65 bytes: r (32) || s (32) || v (1)
    pub fn verify_signature(pubkey: &[u8; 65], message: &[u8], signature: &[u8; 65]) -> Result<()> {
        // Parse the uncompressed public key
        let verifying_key = VerifyingKey::from_sec1_bytes(pubkey)
            .map_err(|e| ZkvmError::Crypto(format!("Invalid public key: {}", e)))?;

        // Parse signature (r || s)
        let sig = Signature::from_slice(&signature[..64])
            .map_err(|e| ZkvmError::Crypto(format!("Invalid signature: {}", e)))?;

        // Hash the message (Ethereum personal_sign style)
        let message_hash = {
            let mut hasher = Sha256::new();
            hasher.update(message);
            hasher.finalize()
        };

        // Verify
        verifying_key
            .verify(&message_hash, &sig)
            .map_err(|e| ZkvmError::Crypto(format!("Signature verification failed: {}", e)))?;

        Ok(())
    }

    /// Derive a shard from a signature (same as hardware/mod.rs)
    pub fn derive_shard(signature: &[u8; 65], domain: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update(signature);
        hasher.finalize().into()
    }
}

/// Derive a public key from a 32-byte secret
fn derive_public_key(secret: &[u8; 32]) -> Result<[u8; 33]> {
    let secret_key = SecretKey::from_bytes(secret.into())
        .map_err(|e| ZkvmError::Crypto(format!("Invalid secret key: {}", e)))?;

    let public_key = secret_key.public_key();
    let encoded = public_key.to_encoded_point(true);

    let mut result = [0u8; 33];
    result.copy_from_slice(encoded.as_bytes());
    Ok(result)
}

/// SP1 hardware derivation prover (requires sp1-prover feature)
#[cfg(feature = "sp1-prover")]
pub struct Sp1HardwareProver {
    prover: sp1_sdk::EnvProver,
    pk: sp1_sdk::SP1ProvingKey,
    vk: sp1_sdk::SP1VerifyingKey,
}

#[cfg(feature = "sp1-prover")]
impl Sp1HardwareProver {
    /// Create a new SP1 hardware prover
    pub fn new() -> Result<Self> {
        use sp1_sdk::ProverClient;

        let client = ProverClient::from_env();

        // Load the ELF from the built program
        let elf = include_bytes!("../../programs/hardware/elf/riscv32im-succinct-zkvm-elf");

        let (pk, vk) = client.setup(elf);

        Ok(Self {
            prover: client,
            pk,
            vk,
        })
    }

    /// Get the verification key
    pub fn vkey(&self) -> &sp1_sdk::SP1VerifyingKey {
        &self.vk
    }
}

#[cfg(feature = "sp1-prover")]
impl HardwareProverTrait for Sp1HardwareProver {
    fn prove(&self, input: HardwareInput) -> Result<(HardwareOutput, Vec<u8>)> {
        use sp1_sdk::SP1Stdin;

        // Write input to SP1 stdin
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);

        // Generate proof
        let mut proof = self
            .prover
            .prove(&self.pk, &stdin)
            .run()
            .map_err(|e| ZkvmError::Sp1Error(e.to_string()))?;

        // Decode output
        let output: HardwareOutput = proof.public_values.read();

        // Serialize proof
        let proof_bytes =
            bincode::serialize(&proof).map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        Ok((output, proof_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_signature() -> ([u8; 65], [u8; 65], Vec<u8>) {
        use k256::ecdsa::signature::Signer;

        // Generate a test key
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Get uncompressed public key
        let pubkey_point = verifying_key.to_encoded_point(false);
        let mut pubkey = [0u8; 65];
        pubkey.copy_from_slice(pubkey_point.as_bytes());

        // Create message
        let message = b"Sigil MPC Cold Master Shard Derivation v1";

        // Hash and sign
        let message_hash: [u8; 32] = {
            let mut hasher = Sha256::new();
            hasher.update(message);
            hasher.finalize().into()
        };

        let sig: Signature = signing_key.sign(&message_hash);

        // Create 65-byte signature (r || s || v)
        let mut signature = [0u8; 65];
        signature[..64].copy_from_slice(&sig.to_bytes()[..]);
        signature[64] = 0; // Recovery ID placeholder

        (pubkey, signature, message.to_vec())
    }

    #[test]
    fn test_hardware_compute() {
        let (device_pubkey, signature, message) = create_test_signature();

        // Derive shard
        let domain = b"cold_master_shard";
        let derived_shard = HardwareProver::derive_shard(&signature, domain);

        let input = HardwareInput {
            signature,
            derived_shard,
            device_pubkey,
            message,
            domain: domain.to_vec(),
            is_cold_shard: true,
        };

        let output = HardwareProver::compute(&input).unwrap();

        assert_eq!(output.device_pubkey, device_pubkey);
        assert!(output.is_cold_shard);
        assert_eq!(output.shard_pubkey.len(), 33);
    }

    #[test]
    fn test_derive_shard_deterministic() {
        let signature = [42u8; 65];
        let domain = b"test_domain";

        let shard1 = HardwareProver::derive_shard(&signature, domain);
        let shard2 = HardwareProver::derive_shard(&signature, domain);

        assert_eq!(shard1, shard2);
    }

    #[test]
    fn test_derive_shard_different_domains() {
        let signature = [42u8; 65];

        let shard1 = HardwareProver::derive_shard(&signature, b"domain1");
        let shard2 = HardwareProver::derive_shard(&signature, b"domain2");

        assert_ne!(shard1, shard2);
    }
}
