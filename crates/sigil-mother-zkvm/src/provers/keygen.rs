//! Master key generation prover
//!
//! Proves: `master_pubkey = [cold_shard]*G + [agent_shard]*G`

use k256::{
    elliptic_curve::{sec1::ToEncodedPoint, PrimeField},
    ProjectivePoint, Scalar,
};

use crate::error::{Result, ZkvmError};
use crate::types::{KeygenInput, KeygenOutput};

/// Keygen prover that can use mock or SP1 backend
pub struct KeygenProver;

impl KeygenProver {
    /// Execute the keygen computation (used by both mock and SP1)
    ///
    /// This is the core computation that proves:
    /// `master_pubkey = [cold_shard]*G + [agent_shard]*G`
    pub fn compute(input: &KeygenInput) -> Result<KeygenOutput> {
        // Convert shards to scalars
        let cold_scalar = Scalar::from_repr(input.cold_shard.into())
            .into_option()
            .ok_or_else(|| ZkvmError::Crypto("Invalid cold shard scalar".into()))?;

        let agent_scalar = Scalar::from_repr(input.agent_shard.into())
            .into_option()
            .ok_or_else(|| ZkvmError::Crypto("Invalid agent shard scalar".into()))?;

        // Compute public key points
        let cold_point = ProjectivePoint::GENERATOR * cold_scalar;
        let agent_point = ProjectivePoint::GENERATOR * agent_scalar;

        // Combined public key = cold_point + agent_point
        let combined_point = cold_point + agent_point;

        // Encode as compressed public keys
        let cold_affine = cold_point.to_affine();
        let cold_pubkey: [u8; 33] = cold_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode cold pubkey".into()))?;

        let agent_affine = agent_point.to_affine();
        let agent_pubkey: [u8; 33] = agent_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode agent pubkey".into()))?;

        let combined_affine = combined_point.to_affine();
        let master_pubkey: [u8; 33] = combined_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode master pubkey".into()))?;

        Ok(KeygenOutput {
            master_pubkey,
            cold_pubkey,
            agent_pubkey,
            ceremony_nonce: input.ceremony_nonce,
        })
    }
}

/// SP1 keygen prover (requires sp1-prover feature)
#[cfg(feature = "sp1-prover")]
pub struct Sp1KeygenProver {
    prover: sp1_sdk::EnvProver,
    pk: sp1_sdk::SP1ProvingKey,
    vk: sp1_sdk::SP1VerifyingKey,
}

#[cfg(feature = "sp1-prover")]
impl Sp1KeygenProver {
    /// Create a new SP1 keygen prover
    pub fn new() -> Result<Self> {
        use sp1_sdk::ProverClient;

        let client = ProverClient::from_env();

        // Load the ELF from the built program
        // The ELF path is determined by the SP1 build system
        let elf = include_bytes!("../../programs/keygen/elf/riscv32im-succinct-zkvm-elf");

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
impl KeygenProverTrait for Sp1KeygenProver {
    fn prove(&self, input: KeygenInput) -> Result<(KeygenOutput, Vec<u8>)> {
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
        let output: KeygenOutput = proof.public_values.read();

        // Serialize proof
        let proof_bytes =
            bincode::serialize(&proof).map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        Ok((output, proof_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keygen_compute() {
        let input = KeygenInput {
            cold_shard: [1u8; 32],
            agent_shard: [2u8; 32],
            ceremony_nonce: [3u8; 32],
        };

        let output = KeygenProver::compute(&input).unwrap();

        // Verify output is valid
        assert_eq!(output.master_pubkey.len(), 33);
        assert_eq!(output.cold_pubkey.len(), 33);
        assert_eq!(output.agent_pubkey.len(), 33);
        assert_eq!(output.ceremony_nonce, input.ceremony_nonce);

        // Verify first byte is valid SEC1 prefix
        assert!(output.master_pubkey[0] == 0x02 || output.master_pubkey[0] == 0x03);
        assert!(output.cold_pubkey[0] == 0x02 || output.cold_pubkey[0] == 0x03);
        assert!(output.agent_pubkey[0] == 0x02 || output.agent_pubkey[0] == 0x03);
    }

    #[test]
    fn test_keygen_deterministic() {
        let input = KeygenInput {
            cold_shard: [42u8; 32],
            agent_shard: [43u8; 32],
            ceremony_nonce: [44u8; 32],
        };

        let output1 = KeygenProver::compute(&input).unwrap();
        let output2 = KeygenProver::compute(&input).unwrap();

        assert_eq!(output1, output2);
    }
}
