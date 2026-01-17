//! Floppy disk format and I/O operations
//!
//! Disk layout (~1.44MB):
//! ```text
//! OFFSET      SIZE        FIELD
//! ──────────────────────────────────────────────────────
//! 0x0000      8           magic: "SIGILDSK"
//! 0x0008      4           version: 1
//! 0x000C      32          child_id (hash of pubkey)
//! 0x002C      33          child_pubkey (compressed)
//! 0x004D      32          derivation_path (serialized)
//! 0x006D      4           presig_total: 1000
//! 0x0071      4           presig_used: 0
//! 0x0075      8           created_at (unix timestamp)
//! 0x007D      8           expires_at (unix timestamp)
//! 0x0085      8           reconciliation_deadline
//! 0x008D      4           max_uses_before_reconcile
//! 0x0091      4           uses_since_reconcile
//! 0x0095      64          mother_signature (signs header)
//! 0x00D5      43          reserved
//!
//! 0x0100      256000      presig_table[1000] (256 bytes each)
//!
//! 0x3E900     ~1.1MB      usage_log[]
//! ```

use serde::{Deserialize, Serialize};

use crate::crypto::{sha256_multi, DerivationPath, PublicKey};
use crate::error::{Error, Result};
use crate::expiry::DiskExpiry;
use crate::presig::{PresigColdShare, PresigStatus};
use crate::types::{ChildId, Signature};
use crate::usage::UsageLog;
use crate::{MAX_PRESIGS, PRESIG_ENTRY_SIZE, VERSION};

/// Magic bytes identifying a Sigil disk
pub const DISK_MAGIC: &[u8; 8] = b"SIGILDSK";

/// Offset to the presig table
pub const PRESIG_TABLE_OFFSET: usize = 0x0100;

/// Offset to the usage log
pub const USAGE_LOG_OFFSET: usize = 0x3E900;

/// Total header size
pub const HEADER_SIZE: usize = 256;

/// Size of the presig table
pub const PRESIG_TABLE_SIZE: usize = MAX_PRESIGS as usize * PRESIG_ENTRY_SIZE;

/// Disk header structure (256 bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskHeader {
    /// Magic bytes (must be "SIGILDSK")
    pub magic: [u8; 8],

    /// Format version
    pub version: u32,

    /// Child ID (SHA256 of public key)
    pub child_id: ChildId,

    /// Child public key (compressed)
    pub child_pubkey: PublicKey,

    /// HD derivation path
    pub derivation_path: DerivationPath,

    /// Total number of presigs on disk
    pub presig_total: u32,

    /// Number of presigs already used
    pub presig_used: u32,

    /// Creation timestamp
    pub created_at: u64,

    /// Expiration configuration
    pub expiry: DiskExpiry,

    /// Mother's signature over the header (excluding this field)
    pub mother_signature: Signature,
}

impl DiskHeader {
    /// Create a new disk header
    pub fn new(
        child_id: ChildId,
        child_pubkey: PublicKey,
        derivation_path: DerivationPath,
        presig_total: u32,
        created_at: u64,
    ) -> Self {
        Self {
            magic: *DISK_MAGIC,
            version: VERSION,
            child_id,
            child_pubkey,
            derivation_path,
            presig_total,
            presig_used: 0,
            created_at,
            expiry: DiskExpiry::new(created_at),
            mother_signature: Signature::new([0u8; 64]), // Placeholder until signed
        }
    }

    /// Serialize to bytes (256 bytes)
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];

        // Magic (0x0000, 8 bytes)
        bytes[0x0000..0x0008].copy_from_slice(&self.magic);

        // Version (0x0008, 4 bytes)
        bytes[0x0008..0x000C].copy_from_slice(&self.version.to_le_bytes());

        // Child ID (0x000C, 32 bytes)
        bytes[0x000C..0x002C].copy_from_slice(self.child_id.as_bytes());

        // Child pubkey (0x002C, 33 bytes)
        bytes[0x002C..0x004D].copy_from_slice(self.child_pubkey.as_bytes());

        // Derivation path (0x004D, 32 bytes)
        bytes[0x004D..0x006D].copy_from_slice(&self.derivation_path.to_bytes());

        // Presig total (0x006D, 4 bytes)
        bytes[0x006D..0x0071].copy_from_slice(&self.presig_total.to_le_bytes());

        // Presig used (0x0071, 4 bytes)
        bytes[0x0071..0x0075].copy_from_slice(&self.presig_used.to_le_bytes());

        // Created at (0x0075, 8 bytes)
        bytes[0x0075..0x007D].copy_from_slice(&self.created_at.to_le_bytes());

        // Expires at (0x007D, 8 bytes)
        bytes[0x007D..0x0085].copy_from_slice(&self.expiry.expires_at.to_le_bytes());

        // Reconciliation deadline (0x0085, 8 bytes)
        bytes[0x0085..0x008D].copy_from_slice(&self.expiry.reconciliation_deadline.to_le_bytes());

        // Max uses before reconcile (0x008D, 4 bytes)
        bytes[0x008D..0x0091].copy_from_slice(&self.expiry.max_uses_before_reconcile.to_le_bytes());

        // Uses since reconcile (0x0091, 4 bytes)
        bytes[0x0091..0x0095].copy_from_slice(&self.expiry.uses_since_reconcile.to_le_bytes());

        // Mother signature (0x0095, 64 bytes)
        bytes[0x0095..0x00D5].copy_from_slice(self.mother_signature.as_bytes());

        // Reserved (0x00D5, 43 bytes) - already zeroed

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; HEADER_SIZE]) -> Result<Self> {
        // Check magic
        let magic: [u8; 8] = bytes[0x0000..0x0008].try_into().unwrap();
        if &magic != DISK_MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Check version
        let version = u32::from_le_bytes(bytes[0x0008..0x000C].try_into().unwrap());
        if version != VERSION {
            return Err(Error::UnsupportedVersion(version));
        }

        // Parse child_id
        let mut child_id_bytes = [0u8; 32];
        child_id_bytes.copy_from_slice(&bytes[0x000C..0x002C]);
        let child_id = ChildId::new(child_id_bytes);

        // Parse child_pubkey
        let mut pubkey_bytes = [0u8; 33];
        pubkey_bytes.copy_from_slice(&bytes[0x002C..0x004D]);
        let child_pubkey = PublicKey::new(pubkey_bytes);

        // Parse derivation path
        let mut path_bytes = [0u8; 32];
        path_bytes.copy_from_slice(&bytes[0x004D..0x006D]);
        let derivation_path = DerivationPath::from_bytes(&path_bytes)?;

        // Parse presig counts
        let presig_total = u32::from_le_bytes(bytes[0x006D..0x0071].try_into().unwrap());
        let presig_used = u32::from_le_bytes(bytes[0x0071..0x0075].try_into().unwrap());

        // Parse timestamps
        let created_at = u64::from_le_bytes(bytes[0x0075..0x007D].try_into().unwrap());
        let expires_at = u64::from_le_bytes(bytes[0x007D..0x0085].try_into().unwrap());
        let reconciliation_deadline = u64::from_le_bytes(bytes[0x0085..0x008D].try_into().unwrap());
        let max_uses_before_reconcile =
            u32::from_le_bytes(bytes[0x008D..0x0091].try_into().unwrap());
        let uses_since_reconcile = u32::from_le_bytes(bytes[0x0091..0x0095].try_into().unwrap());

        let expiry = DiskExpiry {
            expires_at,
            reconciliation_deadline,
            max_uses_before_reconcile,
            uses_since_reconcile,
        };

        // Parse mother signature
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&bytes[0x0095..0x00D5]);
        let mother_signature = Signature::new(sig_bytes);

        Ok(Self {
            magic,
            version,
            child_id,
            child_pubkey,
            derivation_path,
            presig_total,
            presig_used,
            created_at,
            expiry,
            mother_signature,
        })
    }

    /// Get the bytes that are signed by the mother (header without signature)
    pub fn signable_bytes(&self) -> Vec<u8> {
        let bytes = self.to_bytes();
        // Everything up to the mother signature
        bytes[..0x0095].to_vec()
    }

    /// Compute a hash of the signable portion
    pub fn signable_hash(&self) -> [u8; 32] {
        sha256_multi(&[&self.signable_bytes()])
    }

    /// Get the number of remaining presigs
    pub fn presigs_remaining(&self) -> u32 {
        self.presig_total.saturating_sub(self.presig_used)
    }

    /// Check if any presigs are available
    pub fn has_presigs(&self) -> bool {
        self.presigs_remaining() > 0
    }

    /// Validate the header at the given timestamp
    pub fn validate(&self, current_time: u64) -> Result<()> {
        // Check expiry
        if self.expiry.is_expired(current_time) {
            return Err(Error::DiskExpired(self.expiry.expires_at));
        }

        // Check reconciliation deadline
        if self.expiry.is_reconciliation_overdue(current_time) {
            return Err(Error::ReconciliationDeadlinePassed(
                self.expiry.reconciliation_deadline,
            ));
        }

        // Check max uses
        if self.expiry.is_max_uses_exceeded() {
            return Err(Error::MaxUsesExceeded {
                used: self.expiry.uses_since_reconcile,
                max: self.expiry.max_uses_before_reconcile,
            });
        }

        // Check presigs available
        if !self.has_presigs() {
            return Err(Error::NoPresigsAvailable {
                used: self.presig_used,
                total: self.presig_total,
            });
        }

        Ok(())
    }
}

/// Complete disk format including header, presig table, and usage log
#[derive(Debug, Clone)]
pub struct DiskFormat {
    /// Disk header
    pub header: DiskHeader,

    /// Presignature table
    pub presigs: Vec<PresigColdShare>,

    /// Usage log
    pub usage_log: UsageLog,
}

impl DiskFormat {
    /// Create a new disk with the given header and presigs
    pub fn new(header: DiskHeader, presigs: Vec<PresigColdShare>) -> Self {
        Self {
            header,
            presigs,
            usage_log: UsageLog::new(),
        }
    }

    /// Serialize to a byte vector (for writing to disk)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(USAGE_LOG_OFFSET + 100000);

        // Header
        bytes.extend_from_slice(&self.header.to_bytes());

        // Padding to presig table offset
        bytes.resize(PRESIG_TABLE_OFFSET, 0);

        // Presig table
        for presig in &self.presigs {
            bytes.extend_from_slice(&presig.to_bytes());
        }

        // Pad to fill presig table if not all slots used
        let presig_bytes_written = self.presigs.len() * PRESIG_ENTRY_SIZE;
        let presig_bytes_needed = PRESIG_TABLE_SIZE;
        if presig_bytes_written < presig_bytes_needed {
            bytes.resize(PRESIG_TABLE_OFFSET + presig_bytes_needed, 0);
        }

        // Usage log
        bytes.resize(USAGE_LOG_OFFSET, 0);
        bytes.extend_from_slice(&self.usage_log.to_bytes());

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < PRESIG_TABLE_OFFSET {
            return Err(Error::Deserialization(
                "Disk too small for header".to_string(),
            ));
        }

        // Parse header
        let header_bytes: [u8; HEADER_SIZE] = bytes[..HEADER_SIZE]
            .try_into()
            .map_err(|_| Error::Deserialization("Invalid header size".to_string()))?;
        let header = DiskHeader::from_bytes(&header_bytes)?;

        // Parse presig table
        let mut presigs = Vec::with_capacity(header.presig_total as usize);
        for i in 0..header.presig_total as usize {
            let start = PRESIG_TABLE_OFFSET + i * PRESIG_ENTRY_SIZE;
            let end = start + PRESIG_ENTRY_SIZE;

            if end > bytes.len() {
                break;
            }

            let presig_bytes: [u8; PRESIG_ENTRY_SIZE] = bytes[start..end]
                .try_into()
                .map_err(|_| Error::Deserialization("Invalid presig size".to_string()))?;
            presigs.push(PresigColdShare::from_bytes(&presig_bytes));
        }

        // Parse usage log
        let usage_log = if bytes.len() > USAGE_LOG_OFFSET {
            UsageLog::from_bytes(&bytes[USAGE_LOG_OFFSET..]).unwrap_or_default()
        } else {
            UsageLog::new()
        };

        Ok(Self {
            header,
            presigs,
            usage_log,
        })
    }

    /// Get the next available presig
    pub fn get_next_presig(&self) -> Result<(u32, &PresigColdShare)> {
        for (i, presig) in self.presigs.iter().enumerate() {
            if presig.is_fresh() {
                return Ok((i as u32, presig));
            }
        }
        Err(Error::NoPresigsAvailable {
            used: self.header.presig_used,
            total: self.header.presig_total,
        })
    }

    /// Get a specific presig by index
    pub fn get_presig(&self, index: u32) -> Result<&PresigColdShare> {
        let idx = index as usize;
        if idx >= self.presigs.len() {
            return Err(Error::InvalidPresigIndex {
                index,
                max: self.presigs.len() as u32 - 1,
            });
        }

        let presig = &self.presigs[idx];
        match presig.status {
            PresigStatus::Fresh => Ok(presig),
            PresigStatus::Used => Err(Error::PresigAlreadyUsed(index)),
            PresigStatus::Voided => Err(Error::PresigVoided(index)),
        }
    }

    /// Mark a presig as used and increment counters
    pub fn mark_presig_used(&mut self, index: u32) -> Result<()> {
        let idx = index as usize;
        if idx >= self.presigs.len() {
            return Err(Error::InvalidPresigIndex {
                index,
                max: self.presigs.len() as u32 - 1,
            });
        }

        self.presigs[idx].mark_used();
        self.header.presig_used += 1;
        self.header.expiry.record_use();

        Ok(())
    }

    /// Validate the disk at the given timestamp
    pub fn validate(&self, current_time: u64) -> Result<()> {
        // Validate header
        self.header.validate(current_time)?;

        // Validate usage log
        self.usage_log.validate()?;

        // Cross-check: presig_used should match marked presigs
        let marked_used = self
            .presigs
            .iter()
            .filter(|p| p.status == PresigStatus::Used)
            .count() as u32;

        if marked_used != self.header.presig_used {
            return Err(Error::UsageLogAnomaly(format!(
                "Presig count mismatch: header says {} used, but {} are marked",
                self.header.presig_used, marked_used
            )));
        }

        Ok(())
    }

    /// Get disk status summary for display
    pub fn status_summary(&self, current_time: u64) -> DiskStatus {
        let expiry_status = crate::expiry::ExpiryStatus::from_expiry(
            &self.header.expiry,
            current_time,
            self.header.presigs_remaining(),
        );

        DiskStatus {
            child_id: self.header.child_id,
            presigs_remaining: self.header.presigs_remaining(),
            presigs_total: self.header.presig_total,
            expiry: expiry_status,
            usage_count: self.usage_log.len(),
            is_valid: self.validate(current_time).is_ok(),
        }
    }
}

/// Summary status of a disk
#[derive(Debug, Clone)]
pub struct DiskStatus {
    /// Child ID
    pub child_id: ChildId,
    /// Remaining presigs
    pub presigs_remaining: u32,
    /// Total presigs
    pub presigs_total: u32,
    /// Expiry status
    pub expiry: crate::expiry::ExpiryStatus,
    /// Number of usage log entries
    pub usage_count: usize,
    /// Whether disk passes validation
    pub is_valid: bool,
}

impl DiskStatus {
    /// Format for display
    pub fn display(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Disk: sigil_{}\n", self.child_id.short()));
        output.push_str(&format!(
            "Presigs: {}/{} remaining\n",
            self.presigs_remaining, self.presigs_total
        ));
        output.push_str(&format!("Status: {}\n", self.expiry.message));
        output.push_str(&format!("Signatures: {}\n", self.usage_count));
        output.push_str(&format!(
            "Valid: {}\n",
            if self.is_valid { "Yes" } else { "No" }
        ));
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = DiskHeader::new(
            ChildId::new([1u8; 32]),
            PublicKey::new([2u8; 33]),
            DerivationPath::ethereum_hardened(0),
            1000,
            1700000000,
        );

        let bytes = header.to_bytes();
        let recovered = DiskHeader::from_bytes(&bytes).unwrap();

        assert_eq!(header.child_id, recovered.child_id);
        assert_eq!(header.presig_total, recovered.presig_total);
        assert_eq!(header.created_at, recovered.created_at);
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..8].copy_from_slice(b"BADDISK!");

        let result = DiskHeader::from_bytes(&bytes);
        assert!(matches!(result, Err(Error::InvalidMagic)));
    }

    #[test]
    fn test_disk_format_roundtrip() {
        let header = DiskHeader::new(
            ChildId::new([1u8; 32]),
            PublicKey::new([2u8; 33]),
            DerivationPath::ethereum_hardened(0),
            10,
            1700000000,
        );

        let presigs: Vec<PresigColdShare> = (0..10)
            .map(|i| PresigColdShare::new([i as u8; 33], [i as u8; 32], [i as u8; 32]))
            .collect();

        let disk = DiskFormat::new(header, presigs);
        let bytes = disk.to_bytes();
        let recovered = DiskFormat::from_bytes(&bytes).unwrap();

        assert_eq!(disk.header.child_id, recovered.header.child_id);
        assert_eq!(disk.presigs.len(), recovered.presigs.len());
    }
}
