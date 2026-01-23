//! Utility functions for the TUI

use chrono::{DateTime, Duration, Local, Utc};

/// Format a timestamp for display
pub fn format_timestamp(ts: DateTime<Utc>) -> String {
    ts.with_timezone(&Local)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

/// Format a timestamp as relative time (e.g., "2 hours ago")
pub fn format_relative_time(ts: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(ts);

    if diff < Duration::zero() {
        return "in the future".to_string();
    }

    if diff < Duration::minutes(1) {
        return "just now".to_string();
    }

    if diff < Duration::hours(1) {
        let mins = diff.num_minutes();
        return format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" });
    }

    if diff < Duration::days(1) {
        let hours = diff.num_hours();
        return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" });
    }

    if diff < Duration::days(30) {
        let days = diff.num_days();
        return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
    }

    format_timestamp(ts)
}

/// Format bytes as human-readable size
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate a string with ellipsis
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Format a hex string for display (with optional truncation)
pub fn format_hex(hex: &str, max_len: Option<usize>) -> String {
    let display = if hex.starts_with("0x") {
        hex.to_string()
    } else {
        format!("0x{}", hex)
    };

    match max_len {
        Some(len) if display.len() > len => {
            let half = (len - 5) / 2; // Account for "0x" and "..."
            format!(
                "{}...{}",
                &display[..half + 2],
                &display[display.len() - half..]
            )
        }
        _ => display,
    }
}

/// Format days remaining with appropriate styling hint
pub fn format_days_remaining(days: i64) -> (String, DaysStatus) {
    if days < 0 {
        (format!("{} days OVERDUE", -days), DaysStatus::Danger)
    } else if days == 0 {
        ("TODAY".to_string(), DaysStatus::Danger)
    } else if days <= 3 {
        (format!("{} days", days), DaysStatus::Danger)
    } else if days <= 7 {
        (format!("{} days", days), DaysStatus::Warning)
    } else {
        (format!("{} days", days), DaysStatus::Normal)
    }
}

/// Status for days remaining display
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaysStatus {
    Normal,
    Warning,
    Danger,
}

/// Center a string within a given width
pub fn center_string(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        let padding = width - s.len();
        let left = padding / 2;
        let right = padding - left;
        format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1572864), "1.5 MB");
        assert_eq!(format_bytes(1610612736), "1.5 GB");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_format_hex() {
        assert_eq!(format_hex("abcd", None), "0xabcd");
        assert_eq!(format_hex("0xabcd", None), "0xabcd");
        assert_eq!(format_hex("abcdef1234567890", Some(12)), "0xabc...890");
    }
}
