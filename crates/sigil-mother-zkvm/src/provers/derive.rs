//! Child key derivation prover
//!
//! Proves: `child = HKDF(master, path)`, `child_pubkey = [child]*G`

use k256::{
    elliptic_curve::{
        sec1::{FromEncodedPoint, ToEncodedPoint},
        PrimeField,
    },
    AffinePoint, EncodedPoint, ProjectivePoint, Scalar,
};
use sha2::{Digest, Sha256};

use crate::error::{Result, ZkvmError};
use crate::provers::DeriveProverTrait;
use crate::types::{DeriveInput, DeriveOutput};

/// Derive prover that can use mock or SP1 backend
pub struct DeriveProver;

impl DeriveProver {
    /// Execute the derivation computation (used by both mock and SP1)
    ///
    /// This is the core computation that proves:
    /// - `cold_child = SHA256(cold_master || path)`
    /// - `agent_child = SHA256(agent_master || path)`
    /// - `child_pubkey = [cold_child]*G + [agent_child]*G`
    pub fn compute(input: &DeriveInput) -> Result<DeriveOutput> {
        // First verify the master public key
        let cold_master_scalar = Scalar::from_repr(input.cold_master_shard.into())
            .into_option()
            .ok_or_else(|| ZkvmError::Crypto("Invalid cold master shard".into()))?;

        let agent_master_scalar = Scalar::from_repr(input.agent_master_shard.into())
            .into_option()
            .ok_or_else(|| ZkvmError::Crypto("Invalid agent master shard".into()))?;

        // Compute expected master public key
        let cold_master_point = ProjectivePoint::GENERATOR * cold_master_scalar;
        let agent_master_point = ProjectivePoint::GENERATOR * agent_master_scalar;
        let expected_master = cold_master_point + agent_master_point;
        let expected_master_affine = expected_master.to_affine();
        let expected_master_bytes: [u8; 33] = expected_master_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode master pubkey".into()))?;

        // Verify master public key matches
        if expected_master_bytes != input.master_pubkey {
            return Err(ZkvmError::Crypto(
                "Master public key does not match provided shards".into(),
            ));
        }

        // Derive child shards using SHA256 (simplified HKDF)
        let cold_child_shard = {
            let mut hasher = Sha256::new();
            hasher.update(&input.cold_master_shard);
            hasher.update(&input.derivation_path);
            let result: [u8; 32] = hasher.finalize().into();
            result
        };

        let agent_child_shard = {
            let mut hasher = Sha256::new();
            hasher.update(&input.agent_master_shard);
            hasher.update(&input.derivation_path);
            let result: [u8; 32] = hasher.finalize().into();
            result
        };

        // Convert to scalars
        let cold_child_scalar = Scalar::from_repr(cold_child_shard.into())
            .into_option()
            .ok_or_else(|| ZkvmError::Crypto("Invalid cold child scalar".into()))?;

        let agent_child_scalar = Scalar::from_repr(agent_child_shard.into())
            .into_option()
            .ok_or_else(|| ZkvmError::Crypto("Invalid agent child scalar".into()))?;

        // Compute child public keys
        let cold_child_point = ProjectivePoint::GENERATOR * cold_child_scalar;
        let agent_child_point = ProjectivePoint::GENERATOR * agent_child_scalar;
        let child_point = cold_child_point + agent_child_point;

        // Encode as compressed public keys
        let cold_child_affine = cold_child_point.to_affine();
        let cold_child_pubkey: [u8; 33] = cold_child_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode cold child pubkey".into()))?;

        let agent_child_affine = agent_child_point.to_affine();
        let agent_child_pubkey: [u8; 33] = agent_child_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode agent child pubkey".into()))?;

        let child_affine = child_point.to_affine();
        let child_pubkey: [u8; 33] = child_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode child pubkey".into()))?;

        Ok(DeriveOutput {
            child_pubkey,
            cold_child_pubkey,
            agent_child_pubkey,
            derivation_path: input.derivation_path.clone(),
            master_pubkey: input.master_pubkey,
        })
    }

    /// Verify that two child public keys combine to the expected combined key
    pub fn verify_child_pubkey_combination(
        cold_child_pubkey: &[u8; 33],
        agent_child_pubkey: &[u8; 33],
        expected_combined: &[u8; 33],
    ) -> Result<bool> {
        let cold_point = decode_point(cold_child_pubkey)?;
        let agent_point = decode_point(agent_child_pubkey)?;
        let combined = cold_point + agent_point;

        let combined_affine = combined.to_affine();
        let combined_bytes: [u8; 33] = combined_affine
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .map_err(|_| ZkvmError::Crypto("Failed to encode combined pubkey".into()))?;

        Ok(combined_bytes == *expected_combined)
    }
}

/// Decode a compressed point from bytes
fn decode_point(bytes: &[u8; 33]) -> Result<ProjectivePoint> {
    let encoded = EncodedPoint::from_bytes(bytes)
        .map_err(|_| ZkvmError::Crypto("Invalid point encoding".into()))?;

    let affine = AffinePoint::from_encoded_point(&encoded);
    if affine.is_none().into() {
        return Err(ZkvmError::Crypto("Invalid curve point".into()));
    }

    Ok(ProjectivePoint::from(affine.unwrap()))
}

/// SP1 derive prover (requires sp1-prover feature)
#[cfg(feature = "sp1-prover")]
pub struct Sp1DeriveProver {
    client: sp1_sdk::ProverClient,
    pk: sp1_sdk::SP1ProvingKey,
    vk: sp1_sdk::SP1VerifyingKey,
}

#[cfg(feature = "sp1-prover")]
impl Sp1DeriveProver {
    /// Create a new SP1 derive prover
    pub fn new() -> Result<Self> {
        use sp1_sdk::ProverClient;

        let client = ProverClient::new();

        // Load the ELF from the built program
        let elf = include_bytes!("../../programs/derive/elf/riscv32im-succinct-zkvm-elf");

        let (pk, vk) = client.setup(elf);

        Ok(Self { client, pk, vk })
    }

    /// Get the verification key
    pub fn vkey(&self) -> &sp1_sdk::SP1VerifyingKey {
        &self.vk
    }
}

#[cfg(feature = "sp1-prover")]
impl DeriveProverTrait for Sp1DeriveProver {
    fn prove(&self, input: DeriveInput) -> Result<(DeriveOutput, Vec<u8>)> {
        use sp1_sdk::SP1Stdin;

        // Write input to SP1 stdin
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);

        // Generate proof
        let proof = self
            .client
            .prove(&self.pk, stdin)
            .run()
            .map_err(|e| ZkvmError::Sp1Error(e.to_string()))?;

        // Decode output
        let output: DeriveOutput = proof.public_values.read();

        // Serialize proof
        let proof_bytes =
            bincode::serialize(&proof).map_err(|e| ZkvmError::Serialization(e.to_string()))?;

        Ok((output, proof_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_input() -> DeriveInput {
        // Create valid master shards and compute expected master pubkey
        let cold_master = [1u8; 32];
        let agent_master = [2u8; 32];

        let cold_scalar = Scalar::from_repr(cold_master.into()).unwrap();
        let agent_scalar = Scalar::from_repr(agent_master.into()).unwrap();

        let cold_point = ProjectivePoint::GENERATOR * cold_scalar;
        let agent_point = ProjectivePoint::GENERATOR * agent_scalar;
        let master_point = cold_point + agent_point;

        let master_pubkey: [u8; 33] = master_point
            .to_affine()
            .to_encoded_point(true)
            .as_bytes()
            .try_into()
            .unwrap();

        DeriveInput {
            cold_master_shard: cold_master,
            agent_master_shard: agent_master,
            derivation_path: vec![0x80, 0x00, 0x00, 0x2c], // m/44'
            master_pubkey,
        }
    }

    #[test]
    fn test_derive_compute() {
        let input = create_test_input();
        let output = DeriveProver::compute(&input).unwrap();

        // Verify output is valid
        assert_eq!(output.child_pubkey.len(), 33);
        assert_eq!(output.cold_child_pubkey.len(), 33);
        assert_eq!(output.agent_child_pubkey.len(), 33);
        assert_eq!(output.derivation_path, input.derivation_path);
        assert_eq!(output.master_pubkey, input.master_pubkey);

        // Verify child pubkeys combine correctly
        assert!(DeriveProver::verify_child_pubkey_combination(
            &output.cold_child_pubkey,
            &output.agent_child_pubkey,
            &output.child_pubkey
        )
        .unwrap());
    }

    #[test]
    fn test_derive_deterministic() {
        let input = create_test_input();

        let output1 = DeriveProver::compute(&input).unwrap();
        let output2 = DeriveProver::compute(&input).unwrap();

        assert_eq!(output1, output2);
    }

    #[test]
    fn test_derive_different_paths() {
        let mut input1 = create_test_input();
        let mut input2 = create_test_input();

        input1.derivation_path = vec![0x80, 0x00, 0x00, 0x2c, 0x00]; // m/44'/0
        input2.derivation_path = vec![0x80, 0x00, 0x00, 0x2c, 0x01]; // m/44'/1

        let output1 = DeriveProver::compute(&input1).unwrap();
        let output2 = DeriveProver::compute(&input2).unwrap();

        // Different paths should produce different child keys
        assert_ne!(output1.child_pubkey, output2.child_pubkey);
    }
}
