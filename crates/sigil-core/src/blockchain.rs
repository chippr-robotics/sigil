//! Blockchain transaction types and utilities
//!
//! This module provides types and utilities for managing blockchain transactions
//! that can be signed using keyshards.

use crate::error::{Result, SigilError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Represents a blockchain transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transaction {
    /// Transaction ID
    pub id: String,
    
    /// Sender address
    pub from: String,
    
    /// Recipient address
    pub to: String,
    
    /// Amount to transfer
    pub amount: u64,
    
    /// Transaction nonce
    pub nonce: u64,
    
    /// Gas price
    pub gas_price: u64,
    
    /// Gas limit
    pub gas_limit: u64,
    
    /// Optional data payload
    pub data: Option<Vec<u8>>,
    
    /// Transaction timestamp
    pub timestamp: u64,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        from: String,
        to: String,
        amount: u64,
        nonce: u64,
        gas_price: u64,
        gas_limit: u64,
        data: Option<Vec<u8>>,
    ) -> Result<Self> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| SigilError::Blockchain(e.to_string()))?
            .as_secs();

        let mut tx = Self {
            id: String::new(),
            from,
            to,
            amount,
            nonce,
            gas_price,
            gas_limit,
            data,
            timestamp,
        };

        // Calculate transaction ID based on content
        tx.id = tx.calculate_id()?;

        Ok(tx)
    }

    /// Calculate the transaction ID (hash of transaction data)
    fn calculate_id(&self) -> Result<String> {
        let mut hasher = Sha256::new();
        
        hasher.update(self.from.as_bytes());
        hasher.update(self.to.as_bytes());
        hasher.update(&self.amount.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.gas_price.to_le_bytes());
        hasher.update(&self.gas_limit.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        
        if let Some(data) = &self.data {
            hasher.update(data);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Get the message to be signed for this transaction
    pub fn signing_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(self.from.as_bytes());
        message.extend_from_slice(self.to.as_bytes());
        message.extend_from_slice(&self.amount.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message.extend_from_slice(&self.gas_price.to_le_bytes());
        message.extend_from_slice(&self.gas_limit.to_le_bytes());
        message.extend_from_slice(&self.timestamp.to_le_bytes());
        
        if let Some(data) = &self.data {
            message.extend_from_slice(data);
        }

        message
    }

    /// Serialize transaction to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }

    /// Deserialize transaction from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }
}

/// A signed transaction ready for broadcast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// The original transaction
    pub transaction: Transaction,
    
    /// Signature (simplified - in reality would be proper cryptographic signature)
    pub signature: String,
    
    /// Public key or address used for signing
    pub signer: String,
}

impl SignedTransaction {
    /// Create a new signed transaction
    pub fn new(transaction: Transaction, signature: String, signer: String) -> Self {
        Self {
            transaction,
            signature,
            signer,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| SigilError::Serialization(e.to_string()))
    }
}

/// Transaction builder for easier construction
pub struct TransactionBuilder {
    from: Option<String>,
    to: Option<String>,
    amount: u64,
    nonce: u64,
    gas_price: u64,
    gas_limit: u64,
    data: Option<Vec<u8>>,
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            amount: 0,
            nonce: 0,
            gas_price: 1000000000, // 1 Gwei
            gas_limit: 21000,      // Standard transfer
            data: None,
        }
    }
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sender address
    pub fn from(mut self, from: String) -> Self {
        self.from = Some(from);
        self
    }

    /// Set the recipient address
    pub fn to(mut self, to: String) -> Self {
        self.to = Some(to);
        self
    }

    /// Set the amount
    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    /// Set the nonce
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }

    /// Set the gas price
    pub fn gas_price(mut self, gas_price: u64) -> Self {
        self.gas_price = gas_price;
        self
    }

    /// Set the gas limit
    pub fn gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas_limit = gas_limit;
        self
    }

    /// Set the data payload
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    /// Build the transaction
    pub fn build(self) -> Result<Transaction> {
        let from = self.from.ok_or_else(|| {
            SigilError::Blockchain("Sender address is required".to_string())
        })?;

        let to = self.to.ok_or_else(|| {
            SigilError::Blockchain("Recipient address is required".to_string())
        })?;

        Transaction::new(
            from,
            to,
            self.amount,
            self.nonce,
            self.gas_price,
            self.gas_limit,
            self.data,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new(
            "0xSender".to_string(),
            "0xRecipient".to_string(),
            1000,
            1,
            1000000000,
            21000,
            None,
        );
        assert!(tx.is_ok());
        let tx = tx.unwrap();
        assert!(!tx.id.is_empty());
        assert_eq!(tx.amount, 1000);
    }

    #[test]
    fn test_transaction_builder() {
        let tx = TransactionBuilder::new()
            .from("0xSender".to_string())
            .to("0xRecipient".to_string())
            .amount(1000)
            .nonce(5)
            .build();
        
        assert!(tx.is_ok());
        let tx = tx.unwrap();
        assert_eq!(tx.from, "0xSender");
        assert_eq!(tx.to, "0xRecipient");
        assert_eq!(tx.amount, 1000);
        assert_eq!(tx.nonce, 5);
    }

    #[test]
    fn test_transaction_builder_missing_fields() {
        let tx = TransactionBuilder::new()
            .from("0xSender".to_string())
            .amount(1000)
            .build();
        
        assert!(tx.is_err());
    }

    #[test]
    fn test_transaction_serialization() {
        let tx = Transaction::new(
            "0xSender".to_string(),
            "0xRecipient".to_string(),
            1000,
            1,
            1000000000,
            21000,
            None,
        ).unwrap();

        let json = tx.to_json().unwrap();
        let deserialized = Transaction::from_json(&json).unwrap();
        
        assert_eq!(tx, deserialized);
    }

    #[test]
    fn test_signed_transaction() {
        let tx = Transaction::new(
            "0xSender".to_string(),
            "0xRecipient".to_string(),
            1000,
            1,
            1000000000,
            21000,
            None,
        ).unwrap();

        let signed = SignedTransaction::new(
            tx,
            "signature123".to_string(),
            "0xSigner".to_string(),
        );

        let json = signed.to_json().unwrap();
        let deserialized = SignedTransaction::from_json(&json).unwrap();
        
        assert_eq!(signed.signature, deserialized.signature);
        assert_eq!(signed.signer, deserialized.signer);
    }

    #[test]
    fn test_signing_message() {
        let tx = Transaction::new(
            "0xSender".to_string(),
            "0xRecipient".to_string(),
            1000,
            1,
            1000000000,
            21000,
            None,
        ).unwrap();

        let message = tx.signing_message();
        assert!(!message.is_empty());
    }
}
