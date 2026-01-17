//! Child registry
//!
//! Tracks all child disks created by this mother device.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use sigil_core::{
    child::{ChildRegistryEntry, ChildStatus, NullificationReason},
    crypto::DerivationPath,
    ChildId,
};

use crate::error::{MotherError, Result};

/// Registry of all child disks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChildRegistry {
    /// All registered children
    pub children: HashMap<String, ChildRegistryEntry>,

    /// Derivation paths that have been used
    pub used_paths: Vec<String>,
}

impl ChildRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            used_paths: Vec::new(),
        }
    }

    /// Register a new child
    pub fn register_child(
        &mut self,
        child_id: ChildId,
        derivation_path: DerivationPath,
    ) -> Result<()> {
        let id_hex = child_id.to_hex();
        let path_str = derivation_path.to_string_path();

        // Check if child already exists
        if self.children.contains_key(&id_hex) {
            return Err(MotherError::ChildAlreadyExists(id_hex));
        }

        // Check if path is already used
        if self.used_paths.contains(&path_str) {
            return Err(MotherError::DerivationPathUsed(path_str));
        }

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = ChildRegistryEntry::new(child_id, derivation_path, created_at);

        self.children.insert(id_hex, entry);
        self.used_paths.push(path_str);

        Ok(())
    }

    /// Get a child by ID
    pub fn get_child(&self, child_id: &ChildId) -> Result<&ChildRegistryEntry> {
        let id_hex = child_id.to_hex();
        self.children
            .get(&id_hex)
            .ok_or_else(|| MotherError::ChildNotFound(id_hex))
    }

    /// Get mutable child by ID
    pub fn get_child_mut(&mut self, child_id: &ChildId) -> Result<&mut ChildRegistryEntry> {
        let id_hex = child_id.to_hex();
        self.children
            .get_mut(&id_hex)
            .ok_or_else(|| MotherError::ChildNotFound(id_hex))
    }

    /// Check if a child can sign (is active)
    pub fn can_sign(&self, child_id: &ChildId) -> Result<bool> {
        let entry = self.get_child(child_id)?;
        Ok(entry.status.can_sign())
    }

    /// Nullify a child
    pub fn nullify_child(
        &mut self,
        child_id: &ChildId,
        reason: NullificationReason,
        last_valid_presig_index: u32,
    ) -> Result<()> {
        let entry = self.get_child_mut(child_id)?;

        if entry.status.is_nullified() {
            return Err(MotherError::ChildNullified(child_id.to_hex()));
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        entry.nullify(reason, timestamp, last_valid_presig_index);

        Ok(())
    }

    /// Suspend a child (can be reactivated)
    pub fn suspend_child(&mut self, child_id: &ChildId) -> Result<()> {
        let entry = self.get_child_mut(child_id)?;

        if entry.status.is_nullified() {
            return Err(MotherError::ChildNullified(child_id.to_hex()));
        }

        entry.status = ChildStatus::Suspended;

        Ok(())
    }

    /// Reactivate a suspended child
    pub fn reactivate_child(&mut self, child_id: &ChildId) -> Result<()> {
        let entry = self.get_child_mut(child_id)?;

        if !entry.status.can_reactivate() {
            return Err(MotherError::ChildNullified(child_id.to_hex()));
        }

        entry.status = ChildStatus::Active;

        Ok(())
    }

    /// Record a reconciliation for a child
    pub fn record_reconciliation(
        &mut self,
        child_id: &ChildId,
        signatures_since_last: u32,
    ) -> Result<()> {
        let entry = self.get_child_mut(child_id)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        entry.record_reconciliation(timestamp, signatures_since_last);

        Ok(())
    }

    /// List all active children
    pub fn list_active(&self) -> Vec<&ChildRegistryEntry> {
        self.children
            .values()
            .filter(|e| e.status.can_sign())
            .collect()
    }

    /// List all children
    pub fn list_all(&self) -> Vec<&ChildRegistryEntry> {
        self.children.values().collect()
    }

    /// Get count of children by status
    pub fn count_by_status(&self) -> (usize, usize, usize) {
        let mut active = 0;
        let mut suspended = 0;
        let mut nullified = 0;

        for entry in self.children.values() {
            match &entry.status {
                ChildStatus::Active => active += 1,
                ChildStatus::Suspended => suspended += 1,
                ChildStatus::Nullified { .. } => nullified += 1,
            }
        }

        (active, suspended, nullified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_child() {
        let mut registry = ChildRegistry::new();

        let child_id = ChildId::new([1u8; 32]);
        let path = DerivationPath::ethereum_hardened(0);

        registry.register_child(child_id, path).unwrap();

        assert!(registry.get_child(&child_id).is_ok());
        assert!(registry.can_sign(&child_id).unwrap());
    }

    #[test]
    fn test_nullify_child() {
        let mut registry = ChildRegistry::new();

        let child_id = ChildId::new([1u8; 32]);
        let path = DerivationPath::ethereum_hardened(0);

        registry.register_child(child_id, path).unwrap();
        registry
            .nullify_child(&child_id, NullificationReason::ManualRevocation, 100)
            .unwrap();

        assert!(!registry.can_sign(&child_id).unwrap());
    }
}
