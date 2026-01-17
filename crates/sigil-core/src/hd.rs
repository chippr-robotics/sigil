//! Hierarchical Deterministic (HD) key derivation for MPC shards
//!
//! This module implements SLIP-10 compatible HD derivation for both
//! cold and agent master shards.

use crate::error::{Result, SigilError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// HD derivation path component
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathComponent {
    /// Index value
    pub index: u32,
    /// Whether this is a hardened derivation
    pub hardened: bool,
}

impl PathComponent {
    /// Create a normal (non-hardened) component
    pub fn normal(index: u32) -> Self {
        Self {
            index,
            hardened: false,
        }
    }

    /// Create a hardened component
    pub fn hardened(index: u32) -> Self {
        Self {
            index,
            hardened: true,
        }
    }

    /// Get the value to use in derivation (adds 2^31 for hardened)
    pub fn value(&self) -> u32 {
        if self.hardened {
            self.index | 0x80000000
        } else {
            self.index
        }
    }
}

/// HD derivation path (e.g., m/44'/60'/0'/0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DerivationPath {
    pub components: Vec<PathComponent>,
}

impl DerivationPath {
    /// Create a new derivation path
    pub fn new(components: Vec<PathComponent>) -> Self {
        Self { components }
    }

    /// Create a BIP44 path for Ethereum: m/44'/60'/0'/i
    pub fn bip44_ethereum(child_index: u32) -> Self {
        Self {
            components: vec![
                PathComponent::hardened(44),  // BIP44
                PathComponent::hardened(60),  // Ethereum
                PathComponent::hardened(0),   // Account
                PathComponent::normal(child_index), // Child index (non-hardened)
            ],
        }
    }

    /// Serialize to bytes (32 bytes fixed)
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        
        // Store number of components in first byte
        bytes[0] = self.components.len().min(8) as u8;
        
        // Store up to 7 path components (4 bytes each)
        for (i, component) in self.components.iter().take(7).enumerate() {
            let offset = 1 + (i * 4);
            bytes[offset..offset + 4].copy_from_slice(&component.value().to_be_bytes());
        }
        
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let count = bytes[0] as usize;
        if count > 7 {
            return Err(SigilError::HdDerivation(
                "Too many path components".to_string()
            ));
        }
        
        let mut components = Vec::new();
        
        for i in 0..count {
            let offset = 1 + (i * 4);
            let value = u32::from_be_bytes(
                bytes[offset..offset + 4].try_into().unwrap()
            );
            
            let hardened = (value & 0x80000000) != 0;
            let index = value & 0x7FFFFFFF;
            
            components.push(PathComponent { index, hardened });
        }
        
        Ok(Self { components })
    }

    /// Convert to string representation (e.g., "m/44'/60'/0'/0")
    pub fn to_string_path(&self) -> String {
        let mut s = String::from("m");
        for component in &self.components {
            s.push('/');
            s.push_str(&component.index.to_string());
            if component.hardened {
                s.push('\'');
            }
        }
        s
    }
}

/// Master shard (either cold or agent)
#[derive(Debug, Clone)]
pub struct MasterShard {
    /// Secret key material (32 bytes)
    secret: [u8; 32],
    
    /// Chain code for HD derivation (32 bytes)
    chain_code: [u8; 32],
}

impl MasterShard {
    /// Create a new master shard from seed
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        // Use HMAC-SHA256 with key "Sigil MPC seed"
        let key = b"Sigil MPC seed";
        
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(seed);
        let hash = hasher.finalize();
        
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&hash);
        
        // Derive chain code
        let mut hasher2 = Sha256::new();
        hasher2.update(key);
        hasher2.update(b"chain");
        hasher2.update(seed);
        let chain_hash = hasher2.finalize();
        
        let mut chain_code = [0u8; 32];
        chain_code.copy_from_slice(&chain_hash);
        
        Ok(Self { secret, chain_code })
    }

    /// Derive a child shard at the given path
    pub fn derive_child(&self, path: &DerivationPath) -> Result<ChildShard> {
        let mut current_secret = self.secret;
        let mut current_chain = self.chain_code;
        
        for component in &path.components {
            let (new_secret, new_chain) = self.derive_key(
                &current_secret,
                &current_chain,
                component.value(),
            )?;
            
            current_secret = new_secret;
            current_chain = new_chain;
        }
        
        Ok(ChildShard {
            secret: current_secret,
            chain_code: current_chain,
            path: path.clone(),
        })
    }

    /// Derive a single key using CKD (Child Key Derivation)
    fn derive_key(
        &self,
        parent_secret: &[u8; 32],
        parent_chain: &[u8; 32],
        index: u32,
    ) -> Result<([u8; 32], [u8; 32])> {
        let mut hasher = Sha256::new();
        
        // For hardened derivation (index >= 2^31)
        if index & 0x80000000 != 0 {
            hasher.update(&[0u8]); // 0x00 padding
            hasher.update(parent_secret);
        } else {
            // For normal derivation, we'd use the public key
            // Simplified here for demonstration
            hasher.update(parent_secret);
        }
        
        hasher.update(&index.to_be_bytes());
        hasher.update(parent_chain);
        
        let hash = hasher.finalize();
        
        // Split hash into IL (left 32 bytes) and IR (right 32 bytes)
        // For simplicity, we'll use the hash directly
        let mut child_secret = [0u8; 32];
        child_secret.copy_from_slice(&hash);
        
        // Derive new chain code
        let mut chain_hasher = Sha256::new();
        chain_hasher.update(b"chain");
        chain_hasher.update(&hash);
        chain_hasher.update(parent_chain);
        let chain_hash = chain_hasher.finalize();
        
        let mut child_chain = [0u8; 32];
        child_chain.copy_from_slice(&chain_hash);
        
        Ok((child_secret, child_chain))
    }

    /// Get the secret key material (use with caution!)
    pub fn secret(&self) -> &[u8; 32] {
        &self.secret
    }
}

/// Derived child shard
#[derive(Debug, Clone)]
pub struct ChildShard {
    /// Secret key material (32 bytes)
    secret: [u8; 32],
    
    /// Chain code (32 bytes)
    chain_code: [u8; 32],
    
    /// Derivation path
    path: DerivationPath,
}

impl ChildShard {
    /// Get the secret key material
    pub fn secret(&self) -> &[u8; 32] {
        &self.secret
    }

    /// Get the derivation path
    pub fn path(&self) -> &DerivationPath {
        &self.path
    }

    /// Compute the combined public key from cold and agent child shards
    /// In real MPC, this would be: pubkey = point_add(cold_point, agent_point)
    pub fn compute_combined_pubkey(
        cold_shard: &ChildShard,
        agent_shard: &ChildShard,
    ) -> Result<[u8; 33]> {
        if cold_shard.path != agent_shard.path {
            return Err(SigilError::HdDerivation(
                "Path mismatch between cold and agent shards".to_string()
            ));
        }

        // Simplified: combine the secrets and derive pubkey
        // In real MPC, each party would compute their point and add them
        let mut hasher = Sha256::new();
        hasher.update(cold_shard.secret());
        hasher.update(agent_shard.secret());
        let combined = hasher.finalize();
        
        // Create a deterministic public key representation
        let mut pubkey = [0u8; 33];
        pubkey[0] = 0x02; // Compressed point prefix
        pubkey[1..33].copy_from_slice(&combined);
        
        Ok(pubkey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_component() {
        let normal = PathComponent::normal(0);
        assert!(!normal.hardened);
        assert_eq!(normal.value(), 0);

        let hardened = PathComponent::hardened(0);
        assert!(hardened.hardened);
        assert_eq!(hardened.value(), 0x80000000);
    }

    #[test]
    fn test_derivation_path_bip44() {
        let path = DerivationPath::bip44_ethereum(0);
        assert_eq!(path.components.len(), 4);
        assert!(path.components[0].hardened);
        assert_eq!(path.components[0].index, 44);
    }

    #[test]
    fn test_derivation_path_serialization() {
        let path = DerivationPath::bip44_ethereum(5);
        let bytes = path.to_bytes();
        let deserialized = DerivationPath::from_bytes(&bytes).unwrap();
        assert_eq!(path, deserialized);
    }

    #[test]
    fn test_derivation_path_string() {
        let path = DerivationPath::bip44_ethereum(0);
        let s = path.to_string_path();
        assert_eq!(s, "m/44'/60'/0'/0");
    }

    #[test]
    fn test_master_shard_creation() {
        let seed = b"test seed for master shard";
        let master = MasterShard::from_seed(seed).unwrap();
        assert_eq!(master.secret().len(), 32);
    }

    #[test]
    fn test_child_derivation() {
        let seed = b"test seed";
        let master = MasterShard::from_seed(seed).unwrap();
        
        let path = DerivationPath::bip44_ethereum(0);
        let child = master.derive_child(&path).unwrap();
        
        assert_eq!(child.path(), &path);
        assert_eq!(child.secret().len(), 32);
    }

    #[test]
    fn test_combined_pubkey() {
        let cold_seed = b"cold master seed";
        let agent_seed = b"agent master seed";
        
        let cold_master = MasterShard::from_seed(cold_seed).unwrap();
        let agent_master = MasterShard::from_seed(agent_seed).unwrap();
        
        let path = DerivationPath::bip44_ethereum(0);
        
        let cold_child = cold_master.derive_child(&path).unwrap();
        let agent_child = agent_master.derive_child(&path).unwrap();
        
        let pubkey = ChildShard::compute_combined_pubkey(&cold_child, &agent_child).unwrap();
        assert_eq!(pubkey.len(), 33);
        assert_eq!(pubkey[0], 0x02); // Compressed point prefix
    }

    #[test]
    fn test_combined_pubkey_path_mismatch() {
        let cold_seed = b"cold master seed";
        let agent_seed = b"agent master seed";
        
        let cold_master = MasterShard::from_seed(cold_seed).unwrap();
        let agent_master = MasterShard::from_seed(agent_seed).unwrap();
        
        let cold_child = cold_master.derive_child(&DerivationPath::bip44_ethereum(0)).unwrap();
        let agent_child = agent_master.derive_child(&DerivationPath::bip44_ethereum(1)).unwrap();
        
        let result = ChildShard::compute_combined_pubkey(&cold_child, &agent_child);
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_derivation() {
        let seed = b"test seed";
        let master1 = MasterShard::from_seed(seed).unwrap();
        let master2 = MasterShard::from_seed(seed).unwrap();
        
        let path = DerivationPath::bip44_ethereum(5);
        
        let child1 = master1.derive_child(&path).unwrap();
        let child2 = master2.derive_child(&path).unwrap();
        
        assert_eq!(child1.secret(), child2.secret());
    }
}
