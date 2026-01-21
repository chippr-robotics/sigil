//! PIN entry screen for authentication

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::layout::centered_rect;

/// Draw the PIN entry screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // Center the PIN entry dialog
    let dialog = centered_rect(50, 40, area);

    // Dialog box
    let block = Block::default()
        .title(" Authentication Required ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    // Layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(2), // Instructions
            Constraint::Length(3), // PIN display
            Constraint::Length(2), // Error message
            Constraint::Min(1),    // Spacer
            Constraint::Length(1), // Help
        ])
        .split(inner);

    // Logo/title
    let title = Paragraph::new("◆ SIGIL MOTHER")
        .style(theme.title())
        .alignment(Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // Instructions
    let instructions = Paragraph::new("Enter your PIN to unlock")
        .style(theme.text_secondary())
        .alignment(Alignment::Center);
    frame.render_widget(instructions, chunks[1]);

    // PIN display (masked)
    let pin_len = app.state.pin_input.len();
    let max_dots = 12;

    // Build PIN display with dots and placeholders
    let mut pin_display = String::new();
    pin_display.push_str("[ ");
    for i in 0..max_dots {
        if i < pin_len {
            pin_display.push('●');
        } else {
            pin_display.push('○');
        }
        if i < max_dots - 1 {
            pin_display.push(' ');
        }
    }
    pin_display.push_str(" ]");

    let pin_style = if pin_len >= 6 {
        theme.text_highlight()
    } else {
        theme.text()
    };

    let pin_widget = Paragraph::new(pin_display)
        .style(pin_style)
        .alignment(Alignment::Center);
    frame.render_widget(pin_widget, chunks[2]);

    // Error message
    if let Some(error) = &app.state.error_message {
        let error_widget = Paragraph::new(error.as_str())
            .style(theme.danger())
            .alignment(Alignment::Center);
        frame.render_widget(error_widget, chunks[3]);
    }

    // Help text
    let help = if pin_len >= 6 {
        "[Enter] Unlock    [Esc] Quit"
    } else {
        "Enter 6-12 digit PIN    [Esc] Quit"
    };
    let help_widget = Paragraph::new(help)
        .style(theme.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[5]);

    // Attempts remaining warning
    let attempts = app.pin_manager.attempts_remaining();
    if attempts <= 3 && attempts > 0 {
        let warning = format!("⚠ {} attempts remaining before lockout", attempts);
        let warning_y = dialog.y + dialog.height + 1;
        if warning_y < area.height {
            let warning_widget = Paragraph::new(warning)
                .style(theme.warning())
                .alignment(Alignment::Center);
            frame.render_widget(warning_widget, Rect::new(area.x, warning_y, area.width, 1));
        }
    }
}
