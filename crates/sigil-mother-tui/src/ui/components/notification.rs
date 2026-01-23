//! Toast notification component

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::ui::Theme;

/// Notification severity level
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Toast notification
pub struct Notification {
    /// Message to display
    pub message: String,
    /// Severity level
    pub level: NotificationLevel,
    /// Remaining ticks until dismissal
    pub ttl: u64,
}

impl Notification {
    /// Create a new notification
    pub fn new(message: impl Into<String>, level: NotificationLevel, ttl: u64) -> Self {
        Self {
            message: message.into(),
            level,
            ttl,
        }
    }

    /// Create an info notification
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Info, 180) // ~3 seconds at 60fps
    }

    /// Create a success notification
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Success, 180)
    }

    /// Create a warning notification
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Warning, 300) // ~5 seconds
    }

    /// Create an error notification
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, NotificationLevel::Error, 360) // ~6 seconds
    }

    /// Check if notification should be dismissed
    pub fn is_expired(&self) -> bool {
        self.ttl == 0
    }

    /// Decrement TTL
    pub fn tick(&mut self) {
        self.ttl = self.ttl.saturating_sub(1);
    }

    /// Get icon for level
    pub fn icon(&self) -> &'static str {
        match self.level {
            NotificationLevel::Info => "ℹ",
            NotificationLevel::Success => "✓",
            NotificationLevel::Warning => "⚠",
            NotificationLevel::Error => "✗",
        }
    }

    /// Render the notification
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let style = match self.level {
            NotificationLevel::Info => theme.info(),
            NotificationLevel::Success => theme.success(),
            NotificationLevel::Warning => theme.warning(),
            NotificationLevel::Error => theme.danger(),
        };

        // Position at top-right
        let width = (self.message.len() + 6).min(60) as u16;
        let height = 3;
        let x = area.x + area.width.saturating_sub(width + 2);
        let y = area.y + 1;

        let toast_area = Rect::new(x, y, width, height);

        // Clear background
        frame.render_widget(Clear, toast_area);

        // Render toast
        let block = Block::default().borders(Borders::ALL).border_style(style);

        let text = format!("{} {}", self.icon(), self.message);
        let content = Paragraph::new(text)
            .style(style)
            .alignment(Alignment::Center)
            .block(block);

        frame.render_widget(content, toast_area);
    }
}

/// Notification manager for multiple toasts
pub struct NotificationManager {
    /// Active notifications
    notifications: Vec<Notification>,
    /// Maximum notifications to show
    max_visible: usize,
}

impl NotificationManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            max_visible: 3,
        }
    }

    /// Add a notification
    pub fn push(&mut self, notification: Notification) {
        self.notifications.push(notification);
        // Keep only the most recent
        while self.notifications.len() > self.max_visible {
            self.notifications.remove(0);
        }
    }

    /// Tick all notifications and remove expired
    pub fn tick(&mut self) {
        for n in &mut self.notifications {
            n.tick();
        }
        self.notifications.retain(|n| !n.is_expired());
    }

    /// Render all active notifications
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        for (i, notification) in self.notifications.iter().enumerate() {
            let offset_y = (i * 4) as u16;
            let adjusted_area = Rect::new(
                area.x,
                area.y + offset_y,
                area.width,
                area.height.saturating_sub(offset_y),
            );
            notification.render(frame, adjusted_area, theme);
        }
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}
