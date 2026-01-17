//! Storage module for reading/writing keyshards to physical media
//!
//! This module provides functionality for storing and retrieving keyshards
//! from floppy disks or other removable media.

use crate::error::{Result, SigilError};
use crate::keyshard::Keyshard;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Maximum size for a floppy disk (1.44 MB)
pub const FLOPPY_SIZE_BYTES: u64 = 1_474_560;

/// Keyshard file extension
pub const KEYSHARD_EXTENSION: &str = "keyshard";

/// Storage manager for keyshards on physical media
#[derive(Debug)]
pub struct StorageManager {
    /// Base path for storage (e.g., /media/floppy)
    base_path: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager with the given base path
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        if !base_path.exists() {
            return Err(SigilError::DeviceNotFound(
                format!("Path does not exist: {}", base_path.display())
            ));
        }

        Ok(Self { base_path })
    }

    /// Write a keyshard to storage
    pub fn write_keyshard(&self, shard: &Keyshard) -> Result<PathBuf> {
        // Create filename from keyshard ID
        let filename = format!("{}.{}", shard.id, KEYSHARD_EXTENSION);
        let filepath = self.base_path.join(&filename);

        // Serialize to bytes
        let data = shard.to_bytes()?;

        // Check if we have space (rough estimate)
        if let Ok(_metadata) = fs::metadata(&self.base_path) {
            // This is a simplified check; real floppy disk space checking is more complex
            if data.len() as u64 > FLOPPY_SIZE_BYTES {
                return Err(SigilError::Storage(
                    "Keyshard too large for floppy disk".to_string()
                ));
            }
        }

        // Write to file
        fs::write(&filepath, data)?;

        Ok(filepath)
    }

    /// Read a keyshard from storage by ID
    pub fn read_keyshard(&self, id: &str) -> Result<Keyshard> {
        let filename = format!("{}.{}", id, KEYSHARD_EXTENSION);
        let filepath = self.base_path.join(&filename);

        if !filepath.exists() {
            return Err(SigilError::Storage(
                format!("Keyshard not found: {}", id)
            ));
        }

        let data = fs::read(&filepath)?;
        let shard = Keyshard::from_bytes(&data)?;

        // Verify integrity
        if !shard.verify_integrity()? {
            return Err(SigilError::InvalidKeyshard(
                format!("Integrity check failed for keyshard: {}", id)
            ));
        }

        Ok(shard)
    }

    /// List all keyshards in storage
    pub fn list_keyshards(&self) -> Result<Vec<String>> {
        let mut keyshard_ids = Vec::new();

        for entry in WalkDir::new(&self.base_path)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == KEYSHARD_EXTENSION {
                        if let Some(stem) = path.file_stem() {
                            keyshard_ids.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(keyshard_ids)
    }

    /// Load all keyshards from storage
    pub fn load_all_keyshards(&self) -> Result<Vec<Keyshard>> {
        let ids = self.list_keyshards()?;
        let mut shards = Vec::new();

        for id in ids {
            match self.read_keyshard(&id) {
                Ok(shard) => shards.push(shard),
                Err(e) => {
                    eprintln!("Warning: Failed to load keyshard {}: {}", id, e);
                }
            }
        }

        Ok(shards)
    }

    /// Delete a keyshard from storage
    pub fn delete_keyshard(&self, id: &str) -> Result<()> {
        let filename = format!("{}.{}", id, KEYSHARD_EXTENSION);
        let filepath = self.base_path.join(&filename);

        if !filepath.exists() {
            return Err(SigilError::Storage(
                format!("Keyshard not found: {}", id)
            ));
        }

        fs::remove_file(&filepath)?;
        Ok(())
    }

    /// Get available space estimate (simplified)
    pub fn get_available_space(&self) -> Result<u64> {
        // This is a simplified implementation
        // Real floppy disk space checking would require platform-specific code
        Ok(FLOPPY_SIZE_BYTES)
    }

    /// Export a keyshard to JSON format
    pub fn export_keyshard_json(&self, id: &str, output_path: &Path) -> Result<()> {
        let shard = self.read_keyshard(id)?;
        let json = shard.to_json()?;
        fs::write(output_path, json)?;
        Ok(())
    }

    /// Import a keyshard from JSON format
    pub fn import_keyshard_json(&self, json_path: &Path) -> Result<PathBuf> {
        let json = fs::read_to_string(json_path)?;
        let shard = Keyshard::from_json(&json)?;
        self.write_keyshard(&shard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyshard::Keyshard;
    use tempfile::TempDir;

    #[test]
    fn test_storage_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path());
        assert!(manager.is_ok());
    }

    #[test]
    fn test_invalid_path() {
        let manager = StorageManager::new("/nonexistent/path");
        assert!(manager.is_err());
    }

    #[test]
    fn test_write_and_read_keyshard() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path()).unwrap();

        let shard = Keyshard::new(
            "test-shard".to_string(),
            1,
            3,
            b"test data".to_vec(),
            "Test purpose".to_string(),
            None,
        ).unwrap();

        // Write keyshard
        let path = manager.write_keyshard(&shard).unwrap();
        assert!(path.exists());

        // Read keyshard back
        let loaded_shard = manager.read_keyshard("test-shard").unwrap();
        assert_eq!(shard, loaded_shard);
    }

    #[test]
    fn test_list_keyshards() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path()).unwrap();

        // Create multiple keyshards
        for i in 1..=3 {
            let shard = Keyshard::new(
                format!("shard-{}", i),
                i,
                3,
                format!("data-{}", i).into_bytes(),
                "Purpose".to_string(),
                None,
            ).unwrap();
            manager.write_keyshard(&shard).unwrap();
        }

        let ids = manager.list_keyshards().unwrap();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_delete_keyshard() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path()).unwrap();

        let shard = Keyshard::new(
            "delete-test".to_string(),
            1,
            1,
            b"data".to_vec(),
            "Purpose".to_string(),
            None,
        ).unwrap();

        manager.write_keyshard(&shard).unwrap();
        assert!(manager.read_keyshard("delete-test").is_ok());

        manager.delete_keyshard("delete-test").unwrap();
        assert!(manager.read_keyshard("delete-test").is_err());
    }

    #[test]
    fn test_json_export_import() {
        let temp_dir = TempDir::new().unwrap();
        let manager = StorageManager::new(temp_dir.path()).unwrap();

        let shard = Keyshard::new(
            "json-test".to_string(),
            1,
            1,
            b"data".to_vec(),
            "Purpose".to_string(),
            None,
        ).unwrap();

        manager.write_keyshard(&shard).unwrap();

        let json_path = temp_dir.path().join("export.json");
        manager.export_keyshard_json("json-test", &json_path).unwrap();
        assert!(json_path.exists());

        // Delete original
        manager.delete_keyshard("json-test").unwrap();

        // Import back
        manager.import_keyshard_json(&json_path).unwrap();
        let loaded = manager.read_keyshard("json-test").unwrap();
        assert_eq!(shard, loaded);
    }
}
