//! PIN setup screen for first-time configuration

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::layout::centered_rect;

/// Draw the PIN setup screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // Center the setup dialog
    let dialog = centered_rect(60, 60, area);

    // Dialog box
    let block = Block::default()
        .title(" Initial PIN Setup ")
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
            Constraint::Length(2),  // Welcome
            Constraint::Length(4),  // Instructions
            Constraint::Length(2),  // Step indicator
            Constraint::Length(3),  // PIN display
            Constraint::Length(2),  // Requirements
            Constraint::Length(2),  // Error
            Constraint::Min(1),     // Spacer
            Constraint::Length(1),  // Help
        ])
        .split(inner);

    // Welcome message
    let welcome = Paragraph::new("Welcome to Sigil Mother")
        .style(theme.title())
        .alignment(Alignment::Center);
    frame.render_widget(welcome, chunks[0]);

    // Instructions
    let instructions = if app.state.setup_step == 0 {
        "Please create a PIN to protect your Sigil Mother.\n\
         This PIN will be required each time you start the application."
    } else {
        "Please re-enter your PIN to confirm.\n\
         Make sure to remember this PIN - it cannot be recovered."
    };
    let instructions_widget = Paragraph::new(instructions)
        .style(theme.text())
        .alignment(Alignment::Center);
    frame.render_widget(instructions_widget, chunks[1]);

    // Step indicator
    let step_text = if app.state.setup_step == 0 {
        "Step 1 of 2: Create PIN"
    } else {
        "Step 2 of 2: Confirm PIN"
    };
    let step_widget = Paragraph::new(step_text)
        .style(theme.text_secondary())
        .alignment(Alignment::Center);
    frame.render_widget(step_widget, chunks[2]);

    // PIN display
    let current_pin = if app.state.setup_step == 0 {
        &app.state.pin_input
    } else {
        &app.state.pin_confirm
    };
    let pin_len = current_pin.len();
    let max_dots = 12;

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
    frame.render_widget(pin_widget, chunks[3]);

    // Requirements
    let requirements = format!(
        "PIN length: {}/6-12 digits {}",
        pin_len,
        if pin_len >= 6 { "✓" } else { "" }
    );
    let req_style = if pin_len >= 6 {
        theme.success()
    } else {
        theme.text_muted()
    };
    let req_widget = Paragraph::new(requirements)
        .style(req_style)
        .alignment(Alignment::Center);
    frame.render_widget(req_widget, chunks[4]);

    // Error message
    if let Some(error) = &app.state.error_message {
        let error_widget = Paragraph::new(error.as_str())
            .style(theme.danger())
            .alignment(Alignment::Center);
        frame.render_widget(error_widget, chunks[5]);
    }

    // Help text
    let help = if app.state.setup_step == 0 {
        if pin_len >= 6 {
            "[Enter] Continue to confirmation    [Esc] Quit"
        } else {
            "Enter 6-12 digits    [Esc] Quit"
        }
    } else {
        if pin_len >= 6 {
            "[Enter] Complete setup    [Esc] Go back"
        } else {
            "Re-enter your PIN    [Esc] Go back"
        }
    };
    let help_widget = Paragraph::new(help)
        .style(theme.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(help_widget, chunks[7]);

    // Security notice at bottom
    let notice = "⚠ Store your PIN securely. It cannot be recovered if forgotten.";
    let notice_y = dialog.y + dialog.height + 1;
    if notice_y < area.height {
        let notice_widget = Paragraph::new(notice)
            .style(theme.warning())
            .alignment(Alignment::Center);
        frame.render_widget(
            notice_widget,
            Rect::new(area.x, notice_y, area.width, 1),
        );
    }
}
