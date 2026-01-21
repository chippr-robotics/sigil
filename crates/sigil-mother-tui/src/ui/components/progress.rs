//! Progress bar and spinner components

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};

use crate::ui::Theme;

/// Progress bar with percentage display
pub struct ProgressBar {
    /// Current progress (0.0 - 1.0)
    progress: f64,
    /// Label text
    label: String,
    /// Whether to show percentage
    show_percent: bool,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(progress: f64, label: impl Into<String>) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            label: label.into(),
            show_percent: true,
        }
    }

    /// Create from count values
    pub fn from_count(current: u32, total: u32, label: impl Into<String>) -> Self {
        let progress = if total > 0 {
            current as f64 / total as f64
        } else {
            0.0
        };
        Self::new(progress, label)
    }

    /// Hide percentage display
    pub fn hide_percent(mut self) -> Self {
        self.show_percent = false;
        self
    }

    /// Render the progress bar
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let percent = (self.progress * 100.0) as u16;

        let label = if self.show_percent {
            format!("{} - {}%", self.label, percent)
        } else {
            self.label.clone()
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::NONE))
            .gauge_style(Style::default().fg(theme.progress_filled).bg(theme.progress_empty))
            .percent(percent)
            .label(label);

        frame.render_widget(gauge, area);
    }
}

/// Spinner animation for indeterminate progress
pub struct Spinner {
    /// Current frame
    frame: usize,
    /// Spinner characters
    chars: Vec<char>,
    /// Label text
    label: String,
}

impl Spinner {
    /// Create a new spinner
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            frame: 0,
            chars: vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
            label: label.into(),
        }
    }

    /// Create a block spinner
    pub fn blocks(label: impl Into<String>) -> Self {
        Self {
            frame: 0,
            chars: vec!['▖', '▘', '▝', '▗'],
            label: label.into(),
        }
    }

    /// Create a dots spinner
    pub fn dots(label: impl Into<String>) -> Self {
        Self {
            frame: 0,
            chars: vec!['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'],
            label: label.into(),
        }
    }

    /// Advance to next frame
    pub fn tick(&mut self) {
        self.frame = (self.frame + 1) % self.chars.len();
    }

    /// Set frame based on tick counter
    pub fn set_tick(&mut self, tick: u64) {
        self.frame = (tick as usize / 5) % self.chars.len();
    }

    /// Render the spinner
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let spinner_char = self.chars[self.frame];
        let text = format!("{} {}", spinner_char, self.label);

        let paragraph = Paragraph::new(text)
            .style(theme.text_highlight());

        frame.render_widget(paragraph, area);
    }
}

/// Presignature inventory visualization
pub struct PresigInventory {
    /// Fresh presigs
    pub fresh: u32,
    /// Used presigs
    pub used: u32,
    /// Voided presigs
    pub voided: u32,
    /// Total capacity
    pub total: u32,
}

impl PresigInventory {
    /// Create new inventory display
    pub fn new(fresh: u32, used: u32, voided: u32, total: u32) -> Self {
        Self { fresh, used, voided, total }
    }

    /// Get percentage remaining
    pub fn percent_remaining(&self) -> f64 {
        if self.total > 0 {
            self.fresh as f64 / self.total as f64
        } else {
            0.0
        }
    }

    /// Render the inventory
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Stats line
                Constraint::Length(1),  // Bar
                Constraint::Length(1),  // Legend
            ])
            .split(area);

        // Stats line
        let stats = format!(
            "Total: {}    Fresh: {}    Used: {}    Voided: {}",
            self.total, self.fresh, self.used, self.voided
        );
        let stats_widget = Paragraph::new(stats).style(theme.text());
        frame.render_widget(stats_widget, layout[0]);

        // Visual bar
        let bar_width = layout[1].width as usize;
        if self.total > 0 && bar_width > 0 {
            let fresh_width = (self.fresh as usize * bar_width / self.total as usize).min(bar_width);
            let used_width = (self.used as usize * bar_width / self.total as usize).min(bar_width - fresh_width);
            let voided_width = (self.voided as usize * bar_width / self.total as usize).min(bar_width - fresh_width - used_width);

            let mut bar = String::new();
            bar.push_str(&"█".repeat(fresh_width));
            bar.push_str(&"▓".repeat(used_width));
            bar.push_str(&"░".repeat(voided_width));
            bar.push_str(&"░".repeat(bar_width.saturating_sub(fresh_width + used_width + voided_width)));

            let bar_widget = Paragraph::new(bar).style(theme.text_highlight());
            frame.render_widget(bar_widget, layout[1]);
        }

        // Percentage
        let percent = format!("{:.1}%", self.percent_remaining() * 100.0);
        let percent_widget = Paragraph::new(percent)
            .style(theme.text_secondary())
            .alignment(Alignment::Right);
        frame.render_widget(percent_widget, layout[2]);
    }
}
