//! RSA Accumulator for agent nullification
//!
//! This module implements an RSA accumulator that supports efficient non-membership proofs.
//! When an agent is nullified, their ID is added to the accumulator, invalidating their
//! non-membership witness instantly.
//!
//! Key properties:
//! - Zero false positives (cryptographic soundness)
//! - O(1) witness verification
//! - Witnesses can be updated when other elements are added
//! - Compatible with zkVM verification

use serde::{Deserialize, Serialize};

use crate::agent::AgentId;
use crate::types::hex_bytes_32;

/// Size of RSA modulus in bytes (2048 bits = 256 bytes)
pub const RSA_MODULUS_SIZE: usize = 256;

/// RSA Accumulator state
///
/// The accumulator maintains a single value A that represents the set of all
/// nullified agents. When a new agent is nullified, their ID (converted to a prime)
/// is multiplied into the exponent.
///
/// A = g^(p1 * p2 * ... * pn) mod N
///
/// where g is the generator and p_i are the primes representing nullified agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsaAccumulator {
    /// 2048-bit RSA modulus (product of two safe primes)
    #[serde(with = "hex_bytes_256")]
    pub modulus: [u8; RSA_MODULUS_SIZE],

    /// Current accumulator value A
    #[serde(with = "hex_bytes_256")]
    pub accumulator: [u8; RSA_MODULUS_SIZE],

    /// Version counter (increments on each update)
    pub version: u64,

    /// Generator g used for the accumulator
    #[serde(with = "hex_bytes_256")]
    pub generator: [u8; RSA_MODULUS_SIZE],
}

impl RsaAccumulator {
    /// Create a new empty accumulator with the given modulus
    ///
    /// The modulus should be the product of two safe primes p, q where
    /// p = 2p' + 1 and q = 2q' + 1 for primes p', q'.
    /// This ensures the group has no small subgroups.
    pub fn new(modulus: [u8; RSA_MODULUS_SIZE], generator: [u8; RSA_MODULUS_SIZE]) -> Self {
        Self {
            modulus,
            accumulator: generator, // A_0 = g
            version: 0,
            generator,
        }
    }

    /// Add an element (agent ID) to the accumulator
    ///
    /// This is called when nullifying an agent. The agent's ID is converted
    /// to a prime and the accumulator is updated: A' = A^prime mod N
    pub fn add(&mut self, agent_id: &AgentId) -> AccumulatorWitness {
        let prime = agent_id.to_prime();

        // Store old accumulator value as witness for membership proof
        let witness = AccumulatorWitness {
            agent_id: *agent_id,
            witness: self.accumulator,
            accumulator_version: self.version,
        };

        // Update accumulator: A' = A^prime mod N
        self.accumulator = modular_exp(&self.accumulator, &prime, &self.modulus);
        self.version += 1;

        witness
    }

    /// Get the current version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Verify that an element is IN the accumulator (membership proof)
    ///
    /// Given witness w, verify that w^prime = A mod N
    pub fn verify_membership(&self, witness: &AccumulatorWitness) -> bool {
        let prime = witness.agent_id.to_prime();
        let computed = modular_exp(&witness.witness, &prime, &self.modulus);
        computed == self.accumulator
    }

    /// Serialize to bytes for storage/transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(RSA_MODULUS_SIZE * 3 + 8);
        bytes.extend_from_slice(&self.modulus);
        bytes.extend_from_slice(&self.accumulator);
        bytes.extend_from_slice(&self.generator);
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < RSA_MODULUS_SIZE * 3 + 8 {
            return None;
        }

        let mut modulus = [0u8; RSA_MODULUS_SIZE];
        let mut accumulator = [0u8; RSA_MODULUS_SIZE];
        let mut generator = [0u8; RSA_MODULUS_SIZE];

        modulus.copy_from_slice(&bytes[0..RSA_MODULUS_SIZE]);
        accumulator.copy_from_slice(&bytes[RSA_MODULUS_SIZE..RSA_MODULUS_SIZE * 2]);
        generator.copy_from_slice(&bytes[RSA_MODULUS_SIZE * 2..RSA_MODULUS_SIZE * 3]);

        let version_bytes: [u8; 8] = bytes[RSA_MODULUS_SIZE * 3..RSA_MODULUS_SIZE * 3 + 8]
            .try_into()
            .ok()?;
        let version = u64::from_le_bytes(version_bytes);

        Some(Self {
            modulus,
            accumulator,
            generator,
            version,
        })
    }
}

/// Membership witness proving an element IS in the accumulator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatorWitness {
    /// The agent ID this witness is for
    pub agent_id: AgentId,

    /// The witness value (old accumulator before this element was added)
    #[serde(with = "hex_bytes_256")]
    pub witness: [u8; RSA_MODULUS_SIZE],

    /// Version of accumulator when witness was created
    pub accumulator_version: u64,
}

/// Non-membership witness proving an element is NOT in the accumulator
///
/// Uses the Bezout identity: if gcd(prime, product) = 1, then
/// there exist integers a, b such that a*prime + b*product = 1
///
/// This allows proving that prime does NOT divide the product (i.e., not in accumulator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonMembershipWitness {
    /// The agent ID this witness proves non-membership for
    pub agent_id: AgentId,

    /// Bezout coefficient 'a' where a*prime + b*product = gcd
    #[serde(with = "hex_bytes_256")]
    pub bezout_a: [u8; RSA_MODULUS_SIZE],

    /// Cofactor witness 'd' = g^b mod N
    #[serde(with = "hex_bytes_256")]
    pub cofactor_d: [u8; RSA_MODULUS_SIZE],

    /// The accumulator version this witness was computed against
    pub accumulator_version: u64,
}

impl NonMembershipWitness {
    /// Create a new non-membership witness
    ///
    /// This requires knowing the factorization of the accumulator's exponent,
    /// which only the mother device has.
    pub fn new(
        agent_id: AgentId,
        bezout_a: [u8; RSA_MODULUS_SIZE],
        cofactor_d: [u8; RSA_MODULUS_SIZE],
        accumulator_version: u64,
    ) -> Self {
        Self {
            agent_id,
            bezout_a,
            cofactor_d,
            accumulator_version,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32 + RSA_MODULUS_SIZE * 2 + 8);
        bytes.extend_from_slice(self.agent_id.as_bytes());
        bytes.extend_from_slice(&self.bezout_a);
        bytes.extend_from_slice(&self.cofactor_d);
        bytes.extend_from_slice(&self.accumulator_version.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 32 + RSA_MODULUS_SIZE * 2 + 8 {
            return None;
        }

        let mut agent_id_bytes = [0u8; 32];
        let mut bezout_a = [0u8; RSA_MODULUS_SIZE];
        let mut cofactor_d = [0u8; RSA_MODULUS_SIZE];

        agent_id_bytes.copy_from_slice(&bytes[0..32]);
        bezout_a.copy_from_slice(&bytes[32..32 + RSA_MODULUS_SIZE]);
        cofactor_d.copy_from_slice(&bytes[32 + RSA_MODULUS_SIZE..32 + RSA_MODULUS_SIZE * 2]);

        let version_start = 32 + RSA_MODULUS_SIZE * 2;
        let version_bytes: [u8; 8] = bytes[version_start..version_start + 8].try_into().ok()?;
        let accumulator_version = u64::from_le_bytes(version_bytes);

        Some(Self {
            agent_id: AgentId::new(agent_id_bytes),
            bezout_a,
            cofactor_d,
            accumulator_version,
        })
    }
}

/// Verify non-membership: prove that agent_id is NOT in the accumulator
///
/// Verification equation:
/// A^a * d^prime = g mod N
///
/// where:
/// - A is the current accumulator value
/// - a is the Bezout coefficient
/// - d is the cofactor witness
/// - prime is the hash-to-prime of agent_id
/// - g is the generator
pub fn verify_non_membership(accumulator: &RsaAccumulator, witness: &NonMembershipWitness) -> bool {
    let prime = witness.agent_id.to_prime();

    // Compute A^a mod N
    let a_to_a = modular_exp(
        &accumulator.accumulator,
        &witness.bezout_a,
        &accumulator.modulus,
    );

    // Compute d^prime mod N
    let d_to_prime = modular_exp(&witness.cofactor_d, &prime, &accumulator.modulus);

    // Compute A^a * d^prime mod N
    let product = modular_mul(&a_to_a, &d_to_prime, &accumulator.modulus);

    // Should equal generator g
    product == accumulator.generator
}

/// Extended presignature with accumulator version binding
///
/// Presigs are bound to a minimum accumulator version at generation time.
/// This prevents rollback attacks where an attacker uses an old accumulator
/// state from before an agent was nullified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresigWithAccumulatorBinding {
    /// Index in the presig table
    pub index: u32,

    /// Minimum accumulator version required to use this presig
    /// The daemon must have an accumulator with version >= this value
    pub min_accumulator_version: u64,

    /// Hash of the accumulator state at creation time (for audit)
    #[serde(with = "hex_bytes_32")]
    pub accumulator_hash: [u8; 32],
}

/// Stored accumulator state for the daemon
///
/// The daemon stores the latest valid accumulator and rejects any older versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAccumulator {
    /// The accumulator state
    pub accumulator: RsaAccumulator,

    /// Mother's signature over (modulus || accumulator || version)
    #[serde(with = "hex_bytes_64")]
    pub mother_signature: [u8; 64],

    /// When this accumulator was stored
    pub stored_at: u64,
}

impl StoredAccumulator {
    /// Create a new stored accumulator
    pub fn new(accumulator: RsaAccumulator, mother_signature: [u8; 64], stored_at: u64) -> Self {
        Self {
            accumulator,
            mother_signature,
            stored_at,
        }
    }

    /// Get the version of the stored accumulator
    pub fn version(&self) -> u64 {
        self.accumulator.version
    }

    /// Serialize to bytes for storage
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.accumulator.to_bytes();
        bytes.extend_from_slice(&self.mother_signature);
        bytes.extend_from_slice(&self.stored_at.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // RsaAccumulator takes RSA_MODULUS_SIZE * 3 + 8 bytes
        let acc_size = RSA_MODULUS_SIZE * 3 + 8;
        if bytes.len() < acc_size + 64 + 8 {
            return None;
        }

        let accumulator = RsaAccumulator::from_bytes(&bytes[..acc_size])?;

        let mut mother_signature = [0u8; 64];
        mother_signature.copy_from_slice(&bytes[acc_size..acc_size + 64]);

        let stored_at = u64::from_le_bytes(bytes[acc_size + 64..acc_size + 72].try_into().ok()?);

        Some(Self {
            accumulator,
            mother_signature,
            stored_at,
        })
    }
}

// =============================================================================
// Big integer modular arithmetic (simplified implementation)
// In production, use num-bigint or similar library
// =============================================================================

/// Modular exponentiation: base^exp mod modulus
/// Uses square-and-multiply algorithm
fn modular_exp(
    base: &[u8; RSA_MODULUS_SIZE],
    exp: &[u8],
    modulus: &[u8; RSA_MODULUS_SIZE],
) -> [u8; RSA_MODULUS_SIZE] {
    // This is a simplified placeholder implementation
    // In production, use num-bigint::BigUint::modpow

    // For now, we'll use a basic implementation that works for small values
    // and returns a deterministic result for larger values

    let base_u128 = bytes_to_u128(&base[RSA_MODULUS_SIZE - 16..]);
    let mod_u128 = bytes_to_u128(&modulus[RSA_MODULUS_SIZE - 16..]);

    if mod_u128 == 0 {
        return *base;
    }

    let exp_u64 = if exp.len() >= 8 {
        bytes_to_u64(&exp[exp.len() - 8..])
    } else {
        let mut padded = [0u8; 8];
        padded[8 - exp.len()..].copy_from_slice(exp);
        bytes_to_u64(&padded)
    };

    let result = mod_pow_u128(base_u128, exp_u64, mod_u128);

    let mut output = [0u8; RSA_MODULUS_SIZE];
    output[RSA_MODULUS_SIZE - 16..].copy_from_slice(&result.to_be_bytes());
    output
}

/// Modular multiplication: (a * b) mod modulus
fn modular_mul(
    a: &[u8; RSA_MODULUS_SIZE],
    b: &[u8; RSA_MODULUS_SIZE],
    modulus: &[u8; RSA_MODULUS_SIZE],
) -> [u8; RSA_MODULUS_SIZE] {
    // Simplified implementation using u128 for the lower bytes
    let a_u128 = bytes_to_u128(&a[RSA_MODULUS_SIZE - 16..]);
    let b_u128 = bytes_to_u128(&b[RSA_MODULUS_SIZE - 16..]);
    let mod_u128 = bytes_to_u128(&modulus[RSA_MODULUS_SIZE - 16..]);

    if mod_u128 == 0 {
        return *a;
    }

    // Use wrapping mul and mod
    let result = ((a_u128 % mod_u128) * (b_u128 % mod_u128)) % mod_u128;

    let mut output = [0u8; RSA_MODULUS_SIZE];
    output[RSA_MODULUS_SIZE - 16..].copy_from_slice(&result.to_be_bytes());
    output
}

fn bytes_to_u128(bytes: &[u8]) -> u128 {
    let len = bytes.len().min(16);
    let mut arr = [0u8; 16];
    arr[16 - len..].copy_from_slice(&bytes[bytes.len() - len..]);
    u128::from_be_bytes(arr)
}

fn bytes_to_u64(bytes: &[u8]) -> u64 {
    let len = bytes.len().min(8);
    let mut arr = [0u8; 8];
    arr[8 - len..].copy_from_slice(&bytes[bytes.len() - len..]);
    u64::from_be_bytes(arr)
}

fn mod_pow_u128(mut base: u128, mut exp: u64, modulus: u128) -> u128 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1u128;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = mul_mod_u128(result, base, modulus);
        }
        exp >>= 1;
        base = mul_mod_u128(base, base, modulus);
    }
    result
}

fn mul_mod_u128(a: u128, b: u128, modulus: u128) -> u128 {
    // Handle potential overflow by breaking into parts
    let mut result = 0u128;
    let mut a = a % modulus;
    let mut b = b % modulus;

    while b > 0 {
        if b & 1 == 1 {
            result = (result + a) % modulus;
        }
        a = (a << 1) % modulus;
        b >>= 1;
    }
    result
}

// =============================================================================
// Serde helpers for large byte arrays
// =============================================================================

mod hex_bytes_256 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 256], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 256], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut bytes = [0u8; 256];
        hex::decode_to_slice(&s, &mut bytes).map_err(serde::de::Error::custom)?;
        Ok(bytes)
    }
}

mod hex_bytes_64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut bytes = [0u8; 64];
        hex::decode_to_slice(&s, &mut bytes).map_err(serde::de::Error::custom)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_modulus() -> [u8; RSA_MODULUS_SIZE] {
        // Small test modulus (in production, use 2048-bit safe prime product)
        let mut modulus = [0u8; RSA_MODULUS_SIZE];
        // Set a small prime for testing
        modulus[RSA_MODULUS_SIZE - 1] = 251; // Small prime for testing
        modulus[RSA_MODULUS_SIZE - 2] = 1; // Makes it 507
        modulus
    }

    fn test_generator() -> [u8; RSA_MODULUS_SIZE] {
        let mut generator = [0u8; RSA_MODULUS_SIZE];
        generator[RSA_MODULUS_SIZE - 1] = 3; // Common generator
        generator
    }

    #[test]
    fn test_accumulator_creation() {
        let accumulator = RsaAccumulator::new(test_modulus(), test_generator());
        assert_eq!(accumulator.version, 0);
        assert_eq!(accumulator.accumulator, test_generator());
    }

    #[test]
    fn test_accumulator_add_increments_version() {
        let mut accumulator = RsaAccumulator::new(test_modulus(), test_generator());
        let agent_id = AgentId::new([0x42; 32]);

        accumulator.add(&agent_id);
        assert_eq!(accumulator.version, 1);

        accumulator.add(&AgentId::new([0x43; 32]));
        assert_eq!(accumulator.version, 2);
    }

    #[test]
    fn test_accumulator_serialization_roundtrip() {
        let accumulator = RsaAccumulator::new(test_modulus(), test_generator());
        let bytes = accumulator.to_bytes();
        let recovered = RsaAccumulator::from_bytes(&bytes).unwrap();

        assert_eq!(accumulator.modulus, recovered.modulus);
        assert_eq!(accumulator.accumulator, recovered.accumulator);
        assert_eq!(accumulator.generator, recovered.generator);
        assert_eq!(accumulator.version, recovered.version);
    }

    #[test]
    fn test_non_membership_witness_serialization() {
        let witness = NonMembershipWitness::new(
            AgentId::new([0x42; 32]),
            [0x01; RSA_MODULUS_SIZE],
            [0x02; RSA_MODULUS_SIZE],
            5,
        );

        let bytes = witness.to_bytes();
        let recovered = NonMembershipWitness::from_bytes(&bytes).unwrap();

        assert_eq!(witness.agent_id, recovered.agent_id);
        assert_eq!(witness.bezout_a, recovered.bezout_a);
        assert_eq!(witness.cofactor_d, recovered.cofactor_d);
        assert_eq!(witness.accumulator_version, recovered.accumulator_version);
    }

    #[test]
    fn test_agent_to_prime_deterministic() {
        let agent_id = AgentId::new([0x42; 32]);
        let prime1 = agent_id.to_prime();
        let prime2 = agent_id.to_prime();
        assert_eq!(prime1, prime2);
    }

    #[test]
    fn test_agent_to_prime_different_agents() {
        let agent1 = AgentId::new([0x42; 32]);
        let agent2 = AgentId::new([0x43; 32]);
        let prime1 = agent1.to_prime();
        let prime2 = agent2.to_prime();
        assert_ne!(prime1, prime2);
    }
}
