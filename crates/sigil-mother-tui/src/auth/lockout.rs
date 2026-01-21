//! Brute-force protection through progressive lockout

use std::time::Duration;

/// Lockout policy for failed PIN attempts
#[derive(Clone, Debug)]
pub struct LockoutPolicy {
    /// Thresholds and their corresponding lockout durations
    /// Format: (min_attempts, lockout_duration)
    thresholds: Vec<(u32, Duration)>,
}

impl Default for LockoutPolicy {
    fn default() -> Self {
        Self {
            thresholds: vec![
                // Attempts 1-3: No lockout
                // Attempts 4-5: 30 second lockout
                (4, Duration::from_secs(30)),
                // Attempts 6-7: 5 minute lockout
                (6, Duration::from_secs(5 * 60)),
                // Attempts 8-9: 30 minute lockout
                (8, Duration::from_secs(30 * 60)),
                // Attempts 10+: 24 hour lockout
                (10, Duration::from_secs(24 * 60 * 60)),
            ],
        }
    }
}

impl LockoutPolicy {
    /// Get the lockout duration for a given number of failed attempts
    pub fn lockout_duration(&self, failed_attempts: u32) -> Option<Duration> {
        // Find the highest threshold that applies
        self.thresholds
            .iter()
            .rev()
            .find(|(min, _)| failed_attempts >= *min)
            .map(|(_, duration)| *duration)
    }

    /// Get the maximum attempts before any lockout
    pub fn max_attempts(&self) -> u32 {
        self.thresholds
            .first()
            .map(|(min, _)| *min)
            .unwrap_or(3)
    }

    /// Check if currently locked out
    pub fn is_locked_out(&self, failed_attempts: u32) -> bool {
        self.lockout_duration(failed_attempts).is_some()
    }

    /// Get a human-readable description of the lockout
    pub fn lockout_description(&self, failed_attempts: u32) -> Option<String> {
        self.lockout_duration(failed_attempts).map(|duration| {
            let secs = duration.as_secs();
            if secs < 60 {
                format!("{} seconds", secs)
            } else if secs < 3600 {
                format!("{} minutes", secs / 60)
            } else {
                format!("{} hours", secs / 3600)
            }
        })
    }

    /// Create a custom lockout policy
    pub fn custom(thresholds: Vec<(u32, Duration)>) -> Self {
        Self { thresholds }
    }

    /// Create a strict policy (shorter thresholds)
    pub fn strict() -> Self {
        Self {
            thresholds: vec![
                (3, Duration::from_secs(60)),
                (5, Duration::from_secs(10 * 60)),
                (7, Duration::from_secs(60 * 60)),
                (9, Duration::from_secs(24 * 60 * 60)),
            ],
        }
    }

    /// Create a lenient policy (longer thresholds)
    pub fn lenient() -> Self {
        Self {
            thresholds: vec![
                (5, Duration::from_secs(30)),
                (8, Duration::from_secs(5 * 60)),
                (12, Duration::from_secs(30 * 60)),
                (15, Duration::from_secs(24 * 60 * 60)),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = LockoutPolicy::default();

        // No lockout for first 3 attempts
        assert!(policy.lockout_duration(1).is_none());
        assert!(policy.lockout_duration(2).is_none());
        assert!(policy.lockout_duration(3).is_none());

        // 30 seconds for attempts 4-5
        assert_eq!(policy.lockout_duration(4), Some(Duration::from_secs(30)));
        assert_eq!(policy.lockout_duration(5), Some(Duration::from_secs(30)));

        // 5 minutes for attempts 6-7
        assert_eq!(policy.lockout_duration(6), Some(Duration::from_secs(300)));
        assert_eq!(policy.lockout_duration(7), Some(Duration::from_secs(300)));

        // 30 minutes for attempts 8-9
        assert_eq!(policy.lockout_duration(8), Some(Duration::from_secs(1800)));
        assert_eq!(policy.lockout_duration(9), Some(Duration::from_secs(1800)));

        // 24 hours for 10+
        assert_eq!(policy.lockout_duration(10), Some(Duration::from_secs(86400)));
        assert_eq!(policy.lockout_duration(100), Some(Duration::from_secs(86400)));
    }

    #[test]
    fn test_lockout_description() {
        let policy = LockoutPolicy::default();

        assert_eq!(policy.lockout_description(4), Some("30 seconds".to_string()));
        assert_eq!(policy.lockout_description(6), Some("5 minutes".to_string()));
        assert_eq!(policy.lockout_description(10), Some("24 hours".to_string()));
    }
}
