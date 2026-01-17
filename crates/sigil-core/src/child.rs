//! Child disk status and nullification types

use serde::{Deserialize, Serialize};

/// Status of a child disk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChildStatus {
    /// Active and usable
    #[default]
    Active,

    /// Temporarily suspended (can be reactivated)
    Suspended,

    /// Permanently nullified (cannot be reactivated)
    Nullified {
        /// Reason for nullification
        reason: NullificationReason,
        /// Unix timestamp when nullified
        timestamp: u64,
        /// Last presignature index that was valid before nullification
        last_valid_presig_index: u32,
    },
}

impl ChildStatus {
    /// Check if the status allows signing
    pub fn can_sign(&self) -> bool {
        matches!(self, ChildStatus::Active)
    }

    /// Check if the child can be reactivated
    pub fn can_reactivate(&self) -> bool {
        matches!(self, ChildStatus::Suspended)
    }

    /// Check if permanently nullified
    pub fn is_nullified(&self) -> bool {
        matches!(self, ChildStatus::Nullified { .. })
    }

    /// Create a new nullified status
    pub fn nullify(reason: NullificationReason, timestamp: u64, last_valid_index: u32) -> Self {
        ChildStatus::Nullified {
            reason,
            timestamp,
            last_valid_presig_index: last_valid_index,
        }
    }
}

/// Reasons for nullifying a child disk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NullificationReason {
    /// Manual revocation by operator
    ManualRevocation,

    /// Anomaly detected during reconciliation
    ReconciliationAnomaly {
        /// Description of the detected anomaly
        description: String,
    },

    /// Evidence of presignature misuse
    PresigMisuse {
        /// Indices of misused presigs
        affected_indices: Vec<u32>,
    },

    /// Disk reported as lost or stolen
    LostOrStolen {
        /// When the loss was reported
        reported_at: u64,
    },

    /// Agent's key material may be compromised
    CompromisedAgent {
        /// Description of the compromise
        description: String,
    },

    /// Disk failed integrity check
    IntegrityFailure {
        /// What failed
        failure_type: String,
    },

    /// Policy violation (e.g., excessive signing rate)
    PolicyViolation {
        /// Description of the violation
        description: String,
    },
}

impl NullificationReason {
    /// Get a short description of the reason
    pub fn short_description(&self) -> &str {
        match self {
            NullificationReason::ManualRevocation => "Manual revocation",
            NullificationReason::ReconciliationAnomaly { .. } => "Reconciliation anomaly",
            NullificationReason::PresigMisuse { .. } => "Presig misuse",
            NullificationReason::LostOrStolen { .. } => "Lost or stolen",
            NullificationReason::CompromisedAgent { .. } => "Agent compromised",
            NullificationReason::IntegrityFailure { .. } => "Integrity failure",
            NullificationReason::PolicyViolation { .. } => "Policy violation",
        }
    }
}

impl core::fmt::Display for NullificationReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NullificationReason::ManualRevocation => {
                write!(f, "Manual revocation by operator")
            }
            NullificationReason::ReconciliationAnomaly { description } => {
                write!(f, "Reconciliation anomaly: {}", description)
            }
            NullificationReason::PresigMisuse { affected_indices } => {
                write!(
                    f,
                    "Presig misuse detected at indices: {:?}",
                    affected_indices
                )
            }
            NullificationReason::LostOrStolen { reported_at } => {
                write!(f, "Reported lost/stolen at timestamp {}", reported_at)
            }
            NullificationReason::CompromisedAgent { description } => {
                write!(f, "Agent compromised: {}", description)
            }
            NullificationReason::IntegrityFailure { failure_type } => {
                write!(f, "Integrity failure: {}", failure_type)
            }
            NullificationReason::PolicyViolation { description } => {
                write!(f, "Policy violation: {}", description)
            }
        }
    }
}

/// Child registry entry stored on mother device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildRegistryEntry {
    /// Unique identifier for this child
    pub child_id: crate::types::ChildId,

    /// Derivation path used
    pub derivation_path: crate::crypto::DerivationPath,

    /// Current status
    pub status: ChildStatus,

    /// Creation timestamp
    pub created_at: u64,

    /// Last reconciliation timestamp
    pub last_reconciliation: Option<u64>,

    /// Total signatures produced by this child
    pub total_signatures: u64,

    /// Number of times refilled
    pub refill_count: u32,

    /// Nullifier commitment (prevents replay after nullification)
    pub nullifier: Option<[u8; 32]>,
}

impl ChildRegistryEntry {
    /// Create a new registry entry
    pub fn new(
        child_id: crate::types::ChildId,
        derivation_path: crate::crypto::DerivationPath,
        created_at: u64,
    ) -> Self {
        Self {
            child_id,
            derivation_path,
            status: ChildStatus::Active,
            created_at,
            last_reconciliation: None,
            total_signatures: 0,
            refill_count: 0,
            nullifier: None,
        }
    }

    /// Record a reconciliation
    pub fn record_reconciliation(&mut self, timestamp: u64, signatures_since_last: u32) {
        self.last_reconciliation = Some(timestamp);
        self.total_signatures += signatures_since_last as u64;
        self.refill_count += 1;
    }

    /// Nullify this child
    pub fn nullify(&mut self, reason: NullificationReason, timestamp: u64, last_valid_index: u32) {
        // Generate nullifier commitment
        let nullifier = crate::crypto::sha256_multi(&[
            self.child_id.as_bytes(),
            &timestamp.to_le_bytes(),
            reason.short_description().as_bytes(),
        ]);

        self.nullifier = Some(nullifier);
        self.status = ChildStatus::nullify(reason, timestamp, last_valid_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_child_status_can_sign() {
        assert!(ChildStatus::Active.can_sign());
        assert!(!ChildStatus::Suspended.can_sign());
        assert!(!ChildStatus::nullify(NullificationReason::ManualRevocation, 0, 0).can_sign());
    }

    #[test]
    fn test_nullification_reason_display() {
        let reason = NullificationReason::ReconciliationAnomaly {
            description: "Gap in presig indices".to_string(),
        };
        let display = format!("{}", reason);
        assert!(display.contains("Gap in presig indices"));
    }
}
