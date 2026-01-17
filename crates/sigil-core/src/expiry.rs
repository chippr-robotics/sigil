//! Expiration and time-lock configuration

use serde::{Deserialize, Serialize};

use crate::{
    EMERGENCY_RESERVE, MAX_USES_BEFORE_RECONCILE, PRESIG_VALIDITY_DAYS,
    RECONCILIATION_DEADLINE_DAYS, WARNING_THRESHOLD_DAYS,
};

/// Disk expiration and reconciliation configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskExpiry {
    /// Presigs cannot be used after this Unix timestamp
    pub expires_at: u64,

    /// Must reconcile with mother device by this Unix timestamp
    pub reconciliation_deadline: u64,

    /// Maximum transactions before forced reconciliation
    pub max_uses_before_reconcile: u32,

    /// Current count of uses since last reconciliation
    pub uses_since_reconcile: u32,
}

impl DiskExpiry {
    /// Seconds per day
    const SECONDS_PER_DAY: u64 = 86400;

    /// Create a new DiskExpiry with default values starting from current time
    pub fn new(created_at: u64) -> Self {
        Self {
            expires_at: created_at + (PRESIG_VALIDITY_DAYS as u64 * Self::SECONDS_PER_DAY),
            reconciliation_deadline: created_at
                + (RECONCILIATION_DEADLINE_DAYS as u64 * Self::SECONDS_PER_DAY),
            max_uses_before_reconcile: MAX_USES_BEFORE_RECONCILE,
            uses_since_reconcile: 0,
        }
    }

    /// Create with custom validity periods
    pub fn with_custom(
        created_at: u64,
        validity_days: u32,
        reconciliation_days: u32,
        max_uses: u32,
    ) -> Self {
        Self {
            expires_at: created_at + (validity_days as u64 * Self::SECONDS_PER_DAY),
            reconciliation_deadline: created_at
                + (reconciliation_days as u64 * Self::SECONDS_PER_DAY),
            max_uses_before_reconcile: max_uses,
            uses_since_reconcile: 0,
        }
    }

    /// Check if presigs have expired at the given timestamp
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.expires_at
    }

    /// Check if reconciliation deadline has passed
    pub fn is_reconciliation_overdue(&self, current_time: u64) -> bool {
        current_time >= self.reconciliation_deadline
    }

    /// Check if use count has exceeded the maximum
    pub fn is_max_uses_exceeded(&self) -> bool {
        self.uses_since_reconcile >= self.max_uses_before_reconcile
    }

    /// Check if any expiry condition is triggered
    pub fn is_any_limit_reached(&self, current_time: u64) -> bool {
        self.is_expired(current_time)
            || self.is_reconciliation_overdue(current_time)
            || self.is_max_uses_exceeded()
    }

    /// Check if in warning period (approaching expiry)
    pub fn is_warning_period(&self, current_time: u64) -> bool {
        let warning_threshold = self
            .expires_at
            .saturating_sub(WARNING_THRESHOLD_DAYS as u64 * Self::SECONDS_PER_DAY);
        current_time >= warning_threshold && !self.is_expired(current_time)
    }

    /// Get days until expiry (returns 0 if already expired)
    pub fn days_until_expiry(&self, current_time: u64) -> u32 {
        if current_time >= self.expires_at {
            0
        } else {
            ((self.expires_at - current_time) / Self::SECONDS_PER_DAY) as u32
        }
    }

    /// Get days until reconciliation deadline
    pub fn days_until_reconciliation(&self, current_time: u64) -> u32 {
        if current_time >= self.reconciliation_deadline {
            0
        } else {
            ((self.reconciliation_deadline - current_time) / Self::SECONDS_PER_DAY) as u32
        }
    }

    /// Get remaining uses before forced reconciliation
    pub fn remaining_uses(&self) -> u32 {
        self.max_uses_before_reconcile
            .saturating_sub(self.uses_since_reconcile)
    }

    /// Check if we're in the emergency reserve zone
    pub fn is_emergency_reserve(&self, presigs_remaining: u32) -> bool {
        presigs_remaining <= EMERGENCY_RESERVE
    }

    /// Increment usage counter
    pub fn record_use(&mut self) {
        self.uses_since_reconcile = self.uses_since_reconcile.saturating_add(1);
    }

    /// Reset after reconciliation
    pub fn reset_for_reconciliation(
        &mut self,
        new_expires_at: u64,
        new_reconciliation_deadline: u64,
    ) {
        self.expires_at = new_expires_at;
        self.reconciliation_deadline = new_reconciliation_deadline;
        self.uses_since_reconcile = 0;
    }

    /// Serialize to bytes for disk storage (32 bytes)
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&self.expires_at.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.reconciliation_deadline.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.max_uses_before_reconcile.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.uses_since_reconcile.to_le_bytes());
        // Remaining 8 bytes reserved
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            expires_at: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
            reconciliation_deadline: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            max_uses_before_reconcile: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            uses_since_reconcile: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        }
    }
}

/// Status information about disk expiry for display
#[derive(Debug, Clone)]
pub struct ExpiryStatus {
    /// Days until presigs expire
    pub days_until_expiry: u32,
    /// Days until reconciliation deadline
    pub days_until_reconciliation: u32,
    /// Uses remaining before forced reconciliation
    pub uses_remaining: u32,
    /// Whether in warning period
    pub in_warning_period: bool,
    /// Whether in emergency reserve
    pub in_emergency_reserve: bool,
    /// Whether any limit has been reached
    pub is_blocked: bool,
    /// Human-readable status message
    pub message: String,
}

impl ExpiryStatus {
    /// Create status from DiskExpiry and current state
    pub fn from_expiry(expiry: &DiskExpiry, current_time: u64, presigs_remaining: u32) -> Self {
        let days_until_expiry = expiry.days_until_expiry(current_time);
        let days_until_reconciliation = expiry.days_until_reconciliation(current_time);
        let uses_remaining = expiry.remaining_uses();
        let in_warning_period = expiry.is_warning_period(current_time);
        let in_emergency_reserve = expiry.is_emergency_reserve(presigs_remaining);
        let is_blocked = expiry.is_any_limit_reached(current_time);

        let message = if expiry.is_expired(current_time) {
            "Disk has expired - reconciliation required".to_string()
        } else if expiry.is_reconciliation_overdue(current_time) {
            "Reconciliation deadline passed - return to mother device".to_string()
        } else if expiry.is_max_uses_exceeded() {
            "Maximum uses reached - reconciliation required".to_string()
        } else if in_emergency_reserve {
            format!(
                "Emergency reserve active - {} presigs remaining",
                presigs_remaining
            )
        } else if in_warning_period {
            format!("Warning: {} days until expiry", days_until_expiry)
        } else {
            format!(
                "OK: {} days remaining, {}/{} uses",
                days_until_expiry, expiry.uses_since_reconcile, expiry.max_uses_before_reconcile
            )
        };

        Self {
            days_until_expiry,
            days_until_reconciliation,
            uses_remaining,
            in_warning_period,
            in_emergency_reserve,
            is_blocked,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_expiry_defaults() {
        let created = 1700000000u64;
        let expiry = DiskExpiry::new(created);

        assert_eq!(expiry.uses_since_reconcile, 0);
        assert_eq!(expiry.max_uses_before_reconcile, MAX_USES_BEFORE_RECONCILE);

        // Check expiry is 30 days later
        let expected_expiry = created + (PRESIG_VALIDITY_DAYS as u64 * 86400);
        assert_eq!(expiry.expires_at, expected_expiry);
    }

    #[test]
    fn test_expiry_roundtrip() {
        let expiry = DiskExpiry::new(1700000000);
        let bytes = expiry.to_bytes();
        let recovered = DiskExpiry::from_bytes(&bytes);
        assert_eq!(expiry, recovered);
    }

    #[test]
    fn test_warning_period() {
        let created = 1700000000u64;
        let expiry = DiskExpiry::new(created);

        // Just before warning period
        let before_warning = expiry.expires_at - (WARNING_THRESHOLD_DAYS as u64 * 86400) - 1;
        assert!(!expiry.is_warning_period(before_warning));

        // In warning period
        let in_warning = expiry.expires_at - (WARNING_THRESHOLD_DAYS as u64 * 86400) + 1;
        assert!(expiry.is_warning_period(in_warning));

        // After expiry
        assert!(!expiry.is_warning_period(expiry.expires_at + 1));
    }
}
