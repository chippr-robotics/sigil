//! Core type aliases and newtypes

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

/// Child ID - hash of the child public key (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChildId(#[serde(with = "hex_bytes_32")] pub [u8; 32]);

impl ChildId {
    /// Create a new ChildId from bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the bytes of the ChildId
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from hex string
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Short display format (first 4 bytes as hex)
    pub fn short(&self) -> String {
        hex::encode(&self.0[..4])
    }
}

impl AsRef<[u8]> for ChildId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Message hash to be signed (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
pub struct MessageHash(#[serde(with = "hex_bytes_32")] pub [u8; 32]);

impl MessageHash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for MessageHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// ECDSA signature (64 bytes: r || s)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Zeroize)]
pub struct Signature(pub [u8; 64]);

impl Signature {
    pub fn new(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 64];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Get the r component
    pub fn r(&self) -> &[u8; 32] {
        self.0[..32].try_into().unwrap()
    }

    /// Get the s component
    pub fn s(&self) -> &[u8; 32] {
        self.0[32..].try_into().unwrap()
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut bytes = [0u8; 64];
        hex::decode_to_slice(&s, &mut bytes).map_err(serde::de::Error::custom)?;
        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Transaction hash (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxHash(#[serde(with = "hex_bytes_32")] pub [u8; 32]);

impl TxHash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }

    /// Short display format (first 4 bytes as hex)
    pub fn short(&self) -> String {
        format!("0x{}...", hex::encode(&self.0[..4]))
    }
}

impl AsRef<[u8]> for TxHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// zkVM proof hash (32 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkProofHash(#[serde(with = "hex_bytes_32")] pub [u8; 32]);

impl ZkProofHash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for ZkProofHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Chain ID for multi-chain support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainId(pub u32);

impl ChainId {
    /// Ethereum Mainnet
    pub const ETHEREUM: ChainId = ChainId(1);
    /// Ethereum Sepolia Testnet
    pub const SEPOLIA: ChainId = ChainId(11155111);
    /// Arbitrum One
    pub const ARBITRUM: ChainId = ChainId(42161);
    /// Optimism
    pub const OPTIMISM: ChainId = ChainId(10);
    /// Base
    pub const BASE: ChainId = ChainId(8453);
    /// Polygon
    pub const POLYGON: ChainId = ChainId(137);

    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Serde helper for 32-byte arrays as hex strings
pub mod hex_bytes_32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(&s, &mut bytes).map_err(serde::de::Error::custom)?;
        Ok(bytes)
    }
}

/// Serde helper for 33-byte arrays as hex strings
pub mod hex_bytes_33 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 33], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 33], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut bytes = [0u8; 33];
        hex::decode_to_slice(&s, &mut bytes).map_err(serde::de::Error::custom)?;
        Ok(bytes)
    }
}
