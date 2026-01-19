//! Proof storage and manifest management
//!
//! Stores proofs on the mother device's storage with a manifest for indexing.
//!
//! Directory structure:
//! ```text
//! /media/.../SIGIL_MOTHER1/
//!   proofs/
//!     manifest.json              # Index of all proofs
//!     keygen/
//!       proof_<ts>.bin           # Keygen proof binary
//!       public_<ts>.json         # Public output
//!     children/<child_id>/
//!       derive_proof.bin         # Child derivation proof
//!       derive_public.json       # Derivation public output
//!       batch_0000_0999.bin      # Presig batch proof (indices 0-999)
//!       batch_0000_0999.json     # Batch public output
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, ZkvmError};
use crate::types::{
    BatchPresigOutput, DeriveOutput, HardwareOutput, KeygenOutput, ProofMetadata, ProofType,
};

/// Manifest for all proofs stored on a mother device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofManifest {
    /// Version of the manifest format
    pub version: u32,

    /// When the manifest was last updated
    pub updated_at: DateTime<Utc>,

    /// Keygen proofs
    pub keygen_proofs: Vec<ProofEntry>,

    /// Child-specific proofs (indexed by child_id)
    pub child_proofs: Vec<ChildProofs>,
}

impl ProofManifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        Self {
            version: 1,
            updated_at: Utc::now(),
            keygen_proofs: Vec::new(),
            child_proofs: Vec::new(),
        }
    }

    /// Add a keygen proof entry
    pub fn add_keygen_proof(&mut self, entry: ProofEntry) {
        self.keygen_proofs.push(entry);
        self.updated_at = Utc::now();
    }

    /// Get or create child proofs entry
    pub fn get_or_create_child(&mut self, child_id: &str) -> &mut ChildProofs {
        let idx = self
            .child_proofs
            .iter()
            .position(|c| c.child_id == child_id);

        if let Some(idx) = idx {
            &mut self.child_proofs[idx]
        } else {
            self.child_proofs.push(ChildProofs::new(child_id.to_string()));
            self.child_proofs.last_mut().unwrap()
        }
    }
}

impl Default for ProofManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Entry for a single proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEntry {
    /// Relative path to the proof file
    pub proof_path: String,

    /// Relative path to the public output JSON
    pub public_path: String,

    /// When the proof was generated
    pub generated_at: DateTime<Utc>,

    /// Proof type
    pub proof_type: ProofType,

    /// Whether this is a mock proof
    pub is_mock: bool,

    /// Proof size in bytes
    pub proof_size: usize,

    /// Optional description
    pub description: Option<String>,
}

/// Proofs for a specific child
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildProofs {
    /// Child ID (hex string)
    pub child_id: String,

    /// Derivation proof
    pub derive_proof: Option<ProofEntry>,

    /// Hardware derivation proof (if applicable)
    pub hardware_proof: Option<ProofEntry>,

    /// Batch presig proofs
    pub batch_proofs: Vec<BatchProofEntry>,
}

impl ChildProofs {
    /// Create a new child proofs entry
    pub fn new(child_id: String) -> Self {
        Self {
            child_id,
            derive_proof: None,
            hardware_proof: None,
            batch_proofs: Vec::new(),
        }
    }

    /// Add a batch proof entry
    pub fn add_batch_proof(&mut self, entry: BatchProofEntry) {
        self.batch_proofs.push(entry);
    }
}

/// Entry for a batch presig proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProofEntry {
    /// Base proof entry
    #[serde(flatten)]
    pub base: ProofEntry,

    /// Start index of the batch
    pub start_index: u32,

    /// End index (exclusive)
    pub end_index: u32,
}

/// Storage manager for proofs
pub struct ProofStorage {
    /// Base path for proof storage
    base_path: PathBuf,
}

impl ProofStorage {
    /// Create a new proof storage manager
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    /// Get the proofs directory path
    pub fn proofs_dir(&self) -> PathBuf {
        self.base_path.join("proofs")
    }

    /// Ensure the proofs directory structure exists
    pub fn ensure_dirs(&self) -> Result<()> {
        let proofs_dir = self.proofs_dir();
        fs::create_dir_all(proofs_dir.join("keygen"))?;
        Ok(())
    }

    /// Ensure the child directory exists
    pub fn ensure_child_dir(&self, child_id: &str) -> Result<PathBuf> {
        let child_dir = self.proofs_dir().join("children").join(child_id);
        fs::create_dir_all(&child_dir)?;
        Ok(child_dir)
    }

    /// Load the manifest
    pub fn load_manifest(&self) -> Result<ProofManifest> {
        let manifest_path = self.proofs_dir().join("manifest.json");

        if !manifest_path.exists() {
            return Ok(ProofManifest::new());
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest: ProofManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Save the manifest
    pub fn save_manifest(&self, manifest: &ProofManifest) -> Result<()> {
        self.ensure_dirs()?;

        let manifest_path = self.proofs_dir().join("manifest.json");
        let content = serde_json::to_string_pretty(manifest)?;
        fs::write(&manifest_path, content)?;
        Ok(())
    }

    /// Save a keygen proof
    pub fn save_keygen_proof(
        &self,
        output: &KeygenOutput,
        proof: &[u8],
        is_mock: bool,
    ) -> Result<ProofEntry> {
        self.ensure_dirs()?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let proof_filename = format!("proof_{}.bin", timestamp);
        let public_filename = format!("public_{}.json", timestamp);

        let keygen_dir = self.proofs_dir().join("keygen");
        let proof_path = keygen_dir.join(&proof_filename);
        let public_path = keygen_dir.join(&public_filename);

        // Save proof
        fs::write(&proof_path, proof)?;

        // Save public output
        let public_json = serde_json::to_string_pretty(output)?;
        fs::write(&public_path, public_json)?;

        let entry = ProofEntry {
            proof_path: format!("keygen/{}", proof_filename),
            public_path: format!("keygen/{}", public_filename),
            generated_at: Utc::now(),
            proof_type: ProofType::Keygen,
            is_mock,
            proof_size: proof.len(),
            description: None,
        };

        // Update manifest
        let mut manifest = self.load_manifest()?;
        manifest.add_keygen_proof(entry.clone());
        self.save_manifest(&manifest)?;

        Ok(entry)
    }

    /// Save a derive proof for a child
    pub fn save_derive_proof(
        &self,
        child_id: &str,
        output: &DeriveOutput,
        proof: &[u8],
        is_mock: bool,
    ) -> Result<ProofEntry> {
        let child_dir = self.ensure_child_dir(child_id)?;

        let proof_path = child_dir.join("derive_proof.bin");
        let public_path = child_dir.join("derive_public.json");

        // Save proof
        fs::write(&proof_path, proof)?;

        // Save public output
        let public_json = serde_json::to_string_pretty(output)?;
        fs::write(&public_path, public_json)?;

        let entry = ProofEntry {
            proof_path: format!("children/{}/derive_proof.bin", child_id),
            public_path: format!("children/{}/derive_public.json", child_id),
            generated_at: Utc::now(),
            proof_type: ProofType::Derive,
            is_mock,
            proof_size: proof.len(),
            description: None,
        };

        // Update manifest
        let mut manifest = self.load_manifest()?;
        let child = manifest.get_or_create_child(child_id);
        child.derive_proof = Some(entry.clone());
        self.save_manifest(&manifest)?;

        Ok(entry)
    }

    /// Save a batch presig proof for a child
    pub fn save_batch_proof(
        &self,
        child_id: &str,
        output: &BatchPresigOutput,
        proof: &[u8],
        is_mock: bool,
    ) -> Result<BatchProofEntry> {
        let child_dir = self.ensure_child_dir(child_id)?;

        let start = output.start_index;
        let end = start + output.batch_size;
        let batch_name = format!("batch_{:04}_{:04}", start, end - 1);

        let proof_path = child_dir.join(format!("{}.bin", batch_name));
        let public_path = child_dir.join(format!("{}.json", batch_name));

        // Save proof
        fs::write(&proof_path, proof)?;

        // Save public output
        let public_json = serde_json::to_string_pretty(output)?;
        fs::write(&public_path, public_json)?;

        let entry = BatchProofEntry {
            base: ProofEntry {
                proof_path: format!("children/{}/{}.bin", child_id, batch_name),
                public_path: format!("children/{}/{}.json", child_id, batch_name),
                generated_at: Utc::now(),
                proof_type: ProofType::BatchPresig,
                is_mock,
                proof_size: proof.len(),
                description: Some(format!("Presigs {} to {}", start, end - 1)),
            },
            start_index: start,
            end_index: end,
        };

        // Update manifest
        let mut manifest = self.load_manifest()?;
        let child = manifest.get_or_create_child(child_id);
        child.add_batch_proof(entry.clone());
        self.save_manifest(&manifest)?;

        Ok(entry)
    }

    /// Save a hardware derivation proof for a child
    pub fn save_hardware_proof(
        &self,
        child_id: &str,
        output: &HardwareOutput,
        proof: &[u8],
        is_mock: bool,
    ) -> Result<ProofEntry> {
        let child_dir = self.ensure_child_dir(child_id)?;

        let proof_path = child_dir.join("hardware_proof.bin");
        let public_path = child_dir.join("hardware_public.json");

        // Save proof
        fs::write(&proof_path, proof)?;

        // Save public output
        let public_json = serde_json::to_string_pretty(output)?;
        fs::write(&public_path, public_json)?;

        let entry = ProofEntry {
            proof_path: format!("children/{}/hardware_proof.bin", child_id),
            public_path: format!("children/{}/hardware_public.json", child_id),
            generated_at: Utc::now(),
            proof_type: ProofType::Hardware,
            is_mock,
            proof_size: proof.len(),
            description: None,
        };

        // Update manifest
        let mut manifest = self.load_manifest()?;
        let child = manifest.get_or_create_child(child_id);
        child.hardware_proof = Some(entry.clone());
        self.save_manifest(&manifest)?;

        Ok(entry)
    }

    /// Load a proof by path
    pub fn load_proof(&self, relative_path: &str) -> Result<Vec<u8>> {
        let full_path = self.proofs_dir().join(relative_path);
        let content = fs::read(&full_path)?;
        Ok(content)
    }

    /// Load public output by path
    pub fn load_public<T: serde::de::DeserializeOwned>(&self, relative_path: &str) -> Result<T> {
        let full_path = self.proofs_dir().join(relative_path);
        let content = fs::read_to_string(&full_path)?;
        let output: T = serde_json::from_str(&content)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_operations() {
        let mut manifest = ProofManifest::new();

        assert_eq!(manifest.version, 1);
        assert!(manifest.keygen_proofs.is_empty());

        // Add keygen proof
        manifest.add_keygen_proof(ProofEntry {
            proof_path: "keygen/proof_1.bin".into(),
            public_path: "keygen/public_1.json".into(),
            generated_at: Utc::now(),
            proof_type: ProofType::Keygen,
            is_mock: true,
            proof_size: 100,
            description: None,
        });

        assert_eq!(manifest.keygen_proofs.len(), 1);
    }

    #[test]
    fn test_storage_operations() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProofStorage::new(temp_dir.path());

        // Test keygen proof storage
        let output = KeygenOutput {
            master_pubkey: [0x02; 33],
            cold_pubkey: [0x02; 33],
            agent_pubkey: [0x03; 33],
            ceremony_nonce: [0u8; 32],
        };

        let proof = vec![1, 2, 3, 4];
        let entry = storage.save_keygen_proof(&output, &proof, true).unwrap();

        assert!(entry.proof_path.starts_with("keygen/"));
        assert!(entry.is_mock);

        // Load manifest
        let manifest = storage.load_manifest().unwrap();
        assert_eq!(manifest.keygen_proofs.len(), 1);

        // Load proof
        let loaded_proof = storage.load_proof(&entry.proof_path).unwrap();
        assert_eq!(loaded_proof, proof);

        // Load public output
        let loaded_output: KeygenOutput = storage.load_public(&entry.public_path).unwrap();
        assert_eq!(loaded_output, output);
    }

    #[test]
    fn test_child_proof_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProofStorage::new(temp_dir.path());

        let child_id = "abc123";

        // Save derive proof
        let derive_output = DeriveOutput {
            child_pubkey: [0x02; 33],
            cold_child_pubkey: [0x02; 33],
            agent_child_pubkey: [0x03; 33],
            derivation_path: vec![0x80, 0x00, 0x00, 0x2c],
            master_pubkey: [0x02; 33],
        };

        let _derive_entry = storage
            .save_derive_proof(child_id, &derive_output, &[1, 2, 3], true)
            .unwrap();

        // Save batch proof
        let batch_output = BatchPresigOutput {
            r_points_merkle_root: [0u8; 32],
            first_r_point: [0x02; 33],
            last_r_point: [0x03; 33],
            sampled_r_points: vec![],
            batch_size: 100,
            start_index: 0,
            child_pubkey: [0x02; 33],
        };

        let _batch_entry = storage
            .save_batch_proof(child_id, &batch_output, &[4, 5, 6], true)
            .unwrap();

        // Load manifest
        let manifest = storage.load_manifest().unwrap();
        let child = manifest.child_proofs.iter().find(|c| c.child_id == child_id);
        assert!(child.is_some());

        let child = child.unwrap();
        assert!(child.derive_proof.is_some());
        assert_eq!(child.batch_proofs.len(), 1);
    }
}
