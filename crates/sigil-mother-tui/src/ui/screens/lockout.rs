//! Lockout screen displayed when too many PIN attempts fail

use std::time::Instant;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::layout::centered_rect;

/// Draw the lockout screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App, until: Instant) {
    let theme = &app.theme;

    // Center the lockout dialog
    let dialog = centered_rect(50, 40, area);

    // Dialog box with danger styling
    let block = Block::default()
        .title(" Account Locked ")
        .title_style(theme.danger())
        .borders(Borders::ALL)
        .border_style(theme.danger());

    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Warning icon
            Constraint::Length(3), // Message
            Constraint::Length(3), // Countdown
            Constraint::Min(1),    // Spacer
            Constraint::Length(2), // Help
        ])
        .split(inner);

    // Warning icon
    let icon = "⚠  LOCKED  ⚠";
    let icon_widget = Paragraph::new(icon)
        .style(theme.danger())
        .alignment(Alignment::Center);
    frame.render_widget(icon_widget, chunks[0]);

    // Message
    let message = "Too many failed PIN attempts.\nPlease wait before trying again.";
    let message_widget = Paragraph::new(message)
        .style(theme.text())
        .alignment(Alignment::Center);
    frame.render_widget(message_widget, chunks[1]);

    // Countdown
    let remaining = until.duration_since(Instant::now());
    let secs = remaining.as_secs();
    let mins = secs / 60;
    let secs = secs % 60;

    let countdown = if mins > 0 {
        format!("Time remaining: {:02}:{:02}", mins, secs)
    } else {
        format!("Time remaining: {} seconds", secs)
    };

    let countdown_widget = Paragraph::new(countdown)
        .style(theme.warning())
        .alignment(Alignment::Center);
    frame.render_widget(countdown_widget, chunks[2]);

    // Help
    let help = "[Esc] Quit application";
    let help_widget = Paragraph::new(help)
        .style(theme.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[4]);

    // Progress bar showing lockout progress (visual only)
    let total_lockout = app.pin_manager.lockout_remaining_seconds().unwrap_or(1) as f64;
    let remaining_secs = remaining.as_secs() as f64;
    let progress = 1.0 - (remaining_secs / total_lockout.max(1.0));

    let bar_y = chunks[3].y;
    let bar_width = chunks[3].width.saturating_sub(4);
    let bar_x = chunks[3].x + 2;

    let filled = (progress * bar_width as f64) as usize;
    let empty = bar_width as usize - filled;

    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));
    let bar_widget = Paragraph::new(bar)
        .style(theme.danger())
        .alignment(Alignment::Center);
    frame.render_widget(bar_widget, Rect::new(bar_x, bar_y, bar_width, 1));
}
