//! Status bar component

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::ui::Theme;

/// System status indicators
pub struct SystemStatus {
    /// Daemon running status
    pub daemon_running: bool,
    /// Disk detected status
    pub disk_detected: bool,
    /// Session remaining time (formatted)
    pub session_time: Option<String>,
    /// Current child ID (if disk inserted)
    pub child_id: Option<String>,
}

impl Default for SystemStatus {
    fn default() -> Self {
        Self {
            daemon_running: false,
            disk_detected: false,
            session_time: None,
            child_id: None,
        }
    }
}

/// Render the status bar
pub fn render_status_bar(frame: &mut Frame, area: Rect, status: &SystemStatus, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split into sections
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(inner);

    // Left: Daemon and disk status
    let daemon_indicator = if status.daemon_running { "●" } else { "○" };
    let daemon_style = if status.daemon_running {
        theme.success()
    } else {
        theme.text_muted()
    };

    let disk_indicator = if status.disk_detected { "●" } else { "○" };
    let disk_style = if status.disk_detected {
        theme.success()
    } else {
        theme.text_muted()
    };

    let left_text = Line::from(vec![
        Span::styled(format!("{} Daemon ", daemon_indicator), daemon_style),
        Span::styled(format!("{} Disk", disk_indicator), disk_style),
    ]);
    frame.render_widget(Paragraph::new(left_text), chunks[0]);

    // Center: Child ID if disk inserted
    if let Some(child_id) = &status.child_id {
        let center_text = format!("Child: {}", &child_id[..8.min(child_id.len())]);
        let center = Paragraph::new(center_text)
            .style(theme.text())
            .alignment(Alignment::Center);
        frame.render_widget(center, chunks[1]);
    }

    // Right: Session time
    if let Some(time) = &status.session_time {
        let session_text = format!("Session: {}", time);
        let session = Paragraph::new(session_text)
            .style(theme.text_secondary())
            .alignment(Alignment::Right);
        frame.render_widget(session, chunks[2]);
    }
}

/// Render help hints in footer
pub fn render_help_footer(frame: &mut Frame, area: Rect, hints: &[(&str, &str)], theme: &Theme) {
    let hint_spans: Vec<Span> = hints
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(format!("[{}]", key), theme.text_highlight()),
                Span::styled(format!(" {} ", action), theme.text_muted()),
                Span::raw(" "),
            ]
        })
        .collect();

    let line = Line::from(hint_spans);
    let paragraph = Paragraph::new(line).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}
