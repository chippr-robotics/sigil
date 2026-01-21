//! Session management for authenticated mother access
//!
//! Sessions have configurable timeouts to limit exposure time.
//! The encryption key is kept in memory only during an active session.

use std::time::{Duration, Instant};
use zeroize::Zeroize;

use super::AuthError;

/// Session configuration
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Idle timeout duration
    pub idle_timeout: Duration,
    /// Maximum session duration (absolute timeout)
    pub max_duration: Duration,
    /// Warning period before timeout (for UI notifications)
    pub warning_period: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(300),      // 5 minutes
            max_duration: Duration::from_secs(3600),     // 1 hour
            warning_period: Duration::from_secs(60),     // 1 minute warning
        }
    }
}

impl SessionConfig {
    /// Create a stricter configuration for high-security environments
    pub fn strict() -> Self {
        Self {
            idle_timeout: Duration::from_secs(120),      // 2 minutes
            max_duration: Duration::from_secs(1800),     // 30 minutes
            warning_period: Duration::from_secs(30),     // 30 second warning
        }
    }

    /// Create a more lenient configuration for development
    pub fn development() -> Self {
        Self {
            idle_timeout: Duration::from_secs(1800),     // 30 minutes
            max_duration: Duration::from_secs(14400),    // 4 hours
            warning_period: Duration::from_secs(300),    // 5 minute warning
        }
    }
}

/// Authenticated session holding the encryption key
///
/// The encryption key is zeroized when the session is dropped.
pub struct Session {
    /// The encryption key derived from PIN (zeroized on drop)
    encryption_key: [u8; 32],

    /// When the session was created
    created_at: Instant,

    /// When the session was last active
    last_activity: Instant,

    /// Session configuration
    config: SessionConfig,
}

impl Session {
    /// Create a new authenticated session
    pub fn new(encryption_key: [u8; 32], config: SessionConfig) -> Self {
        let now = Instant::now();
        Self {
            encryption_key,
            created_at: now,
            last_activity: now,
            config,
        }
    }

    /// Create with default configuration
    pub fn with_default_config(encryption_key: [u8; 32]) -> Self {
        Self::new(encryption_key, SessionConfig::default())
    }

    /// Get the encryption key (for use in cryptographic operations)
    pub fn encryption_key(&self) -> &[u8; 32] {
        &self.encryption_key
    }

    /// Touch the session (update last activity time)
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if the session is still valid
    pub fn is_valid(&self) -> bool {
        let now = Instant::now();

        // Check absolute timeout
        if now.duration_since(self.created_at) > self.config.max_duration {
            return false;
        }

        // Check idle timeout
        if now.duration_since(self.last_activity) > self.config.idle_timeout {
            return false;
        }

        true
    }

    /// Get time until session expires (minimum of idle and absolute timeout)
    pub fn time_until_expiry(&self) -> Duration {
        let now = Instant::now();

        let absolute_remaining = self.config.max_duration
            .saturating_sub(now.duration_since(self.created_at));

        let idle_remaining = self.config.idle_timeout
            .saturating_sub(now.duration_since(self.last_activity));

        absolute_remaining.min(idle_remaining)
    }

    /// Check if we're within the warning period
    pub fn should_warn(&self) -> bool {
        self.time_until_expiry() <= self.config.warning_period
    }

    /// Get seconds until idle timeout
    pub fn idle_seconds_remaining(&self) -> u64 {
        let now = Instant::now();
        self.config.idle_timeout
            .saturating_sub(now.duration_since(self.last_activity))
            .as_secs()
    }

    /// Get the session configuration
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Validate session and touch if valid
    pub fn validate_and_touch(&mut self) -> Result<(), AuthError> {
        if !self.is_valid() {
            return Err(AuthError::SessionExpired);
        }
        self.touch();
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Explicitly zeroize the encryption key
        self.encryption_key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_session_creation() {
        let key = [1u8; 32];
        let session = Session::with_default_config(key);
        assert!(session.is_valid());
        assert_eq!(session.encryption_key(), &key);
    }

    #[test]
    fn test_session_touch() {
        let key = [1u8; 32];
        let mut session = Session::with_default_config(key);

        let before = session.idle_seconds_remaining();
        sleep(Duration::from_millis(100));
        let after_wait = session.idle_seconds_remaining();
        assert!(after_wait <= before);

        session.touch();
        let after_touch = session.idle_seconds_remaining();
        assert!(after_touch >= after_wait);
    }

    #[test]
    fn test_session_expiry_detection() {
        let key = [1u8; 32];
        let config = SessionConfig {
            idle_timeout: Duration::from_millis(50),
            max_duration: Duration::from_secs(3600),
            warning_period: Duration::from_millis(25),
        };
        let session = Session::new(key, config);

        assert!(session.is_valid());
        sleep(Duration::from_millis(60));
        assert!(!session.is_valid());
    }

    #[test]
    fn test_warning_period() {
        let key = [1u8; 32];
        let config = SessionConfig {
            idle_timeout: Duration::from_millis(100),
            max_duration: Duration::from_secs(3600),
            warning_period: Duration::from_millis(60),
        };
        let session = Session::new(key, config);

        assert!(!session.should_warn());
        sleep(Duration::from_millis(50));
        assert!(session.should_warn());
    }

    #[test]
    fn test_validate_and_touch() {
        let key = [1u8; 32];
        let mut session = Session::with_default_config(key);
        assert!(session.validate_and_touch().is_ok());
    }
}
