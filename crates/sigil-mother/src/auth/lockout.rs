//! Progressive lockout policy for brute-force protection
//!
//! The lockout durations increase with failed attempts to make
//! brute-force attacks impractical:
//!
//! - 1-2 failures: No lockout (allow typos)
//! - 3 failures: 30 second lockout
//! - 4 failures: 5 minute lockout
//! - 5 failures: 30 minute lockout
//! - 6+ failures: 24 hour lockout

use std::time::Duration;

/// Progressive lockout policy
#[derive(Clone, Debug)]
pub struct LockoutPolicy {
    /// Maximum attempts before first lockout
    pub threshold: u32,
    /// Lockout durations for each level (in seconds)
    pub lockout_durations: Vec<u64>,
}

impl Default for LockoutPolicy {
    fn default() -> Self {
        Self {
            threshold: 3,
            lockout_durations: vec![
                30,    // 3 failures: 30 seconds
                300,   // 4 failures: 5 minutes
                1800,  // 5 failures: 30 minutes
                86400, // 6+ failures: 24 hours
            ],
        }
    }
}

impl LockoutPolicy {
    /// Create a strict policy (locks out immediately, longer durations)
    pub fn strict() -> Self {
        Self {
            threshold: 2,
            lockout_durations: vec![
                60,    // 2 failures: 1 minute
                600,   // 3 failures: 10 minutes
                3600,  // 4 failures: 1 hour
                86400, // 5+ failures: 24 hours
            ],
        }
    }

    /// Create a lenient policy (more attempts allowed)
    pub fn lenient() -> Self {
        Self {
            threshold: 5,
            lockout_durations: vec![
                15,   // 5 failures: 15 seconds
                60,   // 6 failures: 1 minute
                300,  // 7 failures: 5 minutes
                1800, // 8+ failures: 30 minutes
            ],
        }
    }

    /// Get the lockout duration for the given number of failed attempts
    /// Returns None if not yet locked out
    pub fn lockout_duration(&self, failed_attempts: u32) -> Option<Duration> {
        if failed_attempts < self.threshold {
            return None;
        }

        let lockout_level = (failed_attempts - self.threshold) as usize;
        let duration_index = lockout_level.min(self.lockout_durations.len() - 1);

        Some(Duration::from_secs(self.lockout_durations[duration_index]))
    }

    /// Get the maximum attempts before permanent lockout
    pub fn max_attempts(&self) -> u32 {
        // After threshold + len(durations) - 1, we're at maximum lockout
        self.threshold + self.lockout_durations.len() as u32
    }

    /// Check if the account should be locked
    pub fn is_locked(&self, failed_attempts: u32) -> bool {
        failed_attempts >= self.threshold
    }

    /// Get a human-readable description of the current lockout state
    pub fn describe_lockout(&self, failed_attempts: u32) -> String {
        if let Some(duration) = self.lockout_duration(failed_attempts) {
            let secs = duration.as_secs();
            if secs < 60 {
                format!("Locked for {} seconds", secs)
            } else if secs < 3600 {
                format!("Locked for {} minutes", secs / 60)
            } else if secs < 86400 {
                format!("Locked for {} hours", secs / 3600)
            } else {
                format!("Locked for {} days", secs / 86400)
            }
        } else {
            format!(
                "{} attempts remaining",
                self.threshold.saturating_sub(failed_attempts)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy_no_lockout_initially() {
        let policy = LockoutPolicy::default();
        assert!(policy.lockout_duration(0).is_none());
        assert!(policy.lockout_duration(1).is_none());
        assert!(policy.lockout_duration(2).is_none());
    }

    #[test]
    fn test_default_policy_lockout_at_threshold() {
        let policy = LockoutPolicy::default();
        let duration = policy.lockout_duration(3).unwrap();
        assert_eq!(duration.as_secs(), 30);
    }

    #[test]
    fn test_default_policy_progressive_lockout() {
        let policy = LockoutPolicy::default();

        assert_eq!(policy.lockout_duration(3).unwrap().as_secs(), 30);
        assert_eq!(policy.lockout_duration(4).unwrap().as_secs(), 300);
        assert_eq!(policy.lockout_duration(5).unwrap().as_secs(), 1800);
        assert_eq!(policy.lockout_duration(6).unwrap().as_secs(), 86400);
    }

    #[test]
    fn test_lockout_caps_at_max() {
        let policy = LockoutPolicy::default();

        // Even at 100 failures, should cap at max duration
        assert_eq!(policy.lockout_duration(100).unwrap().as_secs(), 86400);
    }

    #[test]
    fn test_is_locked() {
        let policy = LockoutPolicy::default();

        assert!(!policy.is_locked(0));
        assert!(!policy.is_locked(2));
        assert!(policy.is_locked(3));
        assert!(policy.is_locked(10));
    }

    #[test]
    fn test_max_attempts() {
        let policy = LockoutPolicy::default();
        // threshold (3) + durations (4) = 7
        assert_eq!(policy.max_attempts(), 7);
    }
}
