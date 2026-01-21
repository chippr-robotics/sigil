//! Session management with timeout handling

use std::time::{Duration, Instant};

/// Default session timeout (5 minutes)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Warning period before session expires (60 seconds)
const WARNING_PERIOD: Duration = Duration::from_secs(60);

/// User session with activity tracking
#[derive(Clone, Debug)]
pub struct Session {
    /// When the session was created
    created_at: Instant,
    /// Last activity timestamp
    last_activity: Instant,
    /// Session timeout duration
    timeout: Duration,
}

impl Session {
    /// Create a new session with default timeout
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            created_at: now,
            last_activity: now,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Create a session with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        let now = Instant::now();
        Self {
            created_at: now,
            last_activity: now,
            timeout,
        }
    }

    /// Record activity (resets timeout)
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if session has expired
    pub fn is_expired(&self) -> bool {
        self.last_activity.elapsed() > self.timeout
    }

    /// Check if we're in the warning period
    pub fn is_warning_period(&self) -> bool {
        let elapsed = self.last_activity.elapsed();
        !self.is_expired() && elapsed > self.timeout.saturating_sub(WARNING_PERIOD)
    }

    /// Get remaining time until expiry in seconds
    pub fn remaining_seconds(&self) -> u64 {
        let elapsed = self.last_activity.elapsed();
        self.timeout
            .saturating_sub(elapsed)
            .as_secs()
    }

    /// Get session duration since creation
    pub fn duration(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get time since last activity
    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// Change the timeout duration
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Get the current timeout setting
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Format remaining time as MM:SS
    pub fn remaining_formatted(&self) -> String {
        let secs = self.remaining_seconds();
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Session configuration
#[derive(Clone, Debug)]
pub struct SessionConfig {
    /// Timeout duration
    pub timeout: Duration,
    /// Warning period before timeout
    pub warning_period: Duration,
    /// Whether to show countdown
    pub show_countdown: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            warning_period: WARNING_PERIOD,
            show_countdown: true,
        }
    }
}

impl SessionConfig {
    /// Create a config for testing (short timeouts)
    pub fn testing() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            warning_period: Duration::from_secs(10),
            show_countdown: true,
        }
    }

    /// Create a config for high-security environments
    pub fn high_security() -> Self {
        Self {
            timeout: Duration::from_secs(2 * 60), // 2 minutes
            warning_period: Duration::from_secs(30),
            show_countdown: true,
        }
    }

    /// Create a config for convenience (longer timeout)
    pub fn convenience() -> Self {
        Self {
            timeout: Duration::from_secs(15 * 60), // 15 minutes
            warning_period: Duration::from_secs(120),
            show_countdown: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_session_creation() {
        let session = Session::new();
        assert!(!session.is_expired());
        assert!(!session.is_warning_period());
        assert!(session.remaining_seconds() > 0);
    }

    #[test]
    fn test_session_touch() {
        let mut session = Session::with_timeout(Duration::from_millis(100));
        sleep(Duration::from_millis(50));
        session.touch();
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_expiry() {
        let session = Session::with_timeout(Duration::from_millis(50));
        sleep(Duration::from_millis(60));
        assert!(session.is_expired());
    }

    #[test]
    fn test_remaining_formatted() {
        let session = Session::with_timeout(Duration::from_secs(125));
        let formatted = session.remaining_formatted();
        assert!(formatted.starts_with("02:"));
    }
}
