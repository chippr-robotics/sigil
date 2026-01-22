//! Confirmation dialog component

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::ConfirmAction;
use crate::ui::{layout::centered_rect, Theme};

/// Confirmation dialog configuration
pub struct ConfirmDialog<'a> {
    /// Dialog title
    pub title: &'a str,
    /// Warning message
    pub message: &'a str,
    /// What the user needs to type to confirm
    pub confirm_text: Option<&'a str>,
    /// Current user input
    pub input: &'a str,
    /// Whether this is a dangerous operation
    pub dangerous: bool,
}

impl<'a> ConfirmDialog<'a> {
    /// Create a simple yes/no dialog
    pub fn simple(title: &'a str, message: &'a str) -> Self {
        Self {
            title,
            message,
            confirm_text: None,
            input: "",
            dangerous: false,
        }
    }

    /// Create a typed confirmation dialog
    pub fn typed(title: &'a str, message: &'a str, confirm_text: &'a str, input: &'a str) -> Self {
        Self {
            title,
            message,
            confirm_text: Some(confirm_text),
            input,
            dangerous: true,
        }
    }

    /// Render the dialog
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let dialog_area = centered_rect(60, 50, area);

        // Clear the background
        frame.render_widget(Clear, dialog_area);

        // Dialog box
        let border_style = if self.dangerous {
            theme.danger()
        } else {
            theme.border_focused()
        };

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .title_style(if self.dangerous {
                theme.danger()
            } else {
                theme.title()
            })
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout inner content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(3),    // Message
                Constraint::Length(3), // Input or buttons
                Constraint::Length(1), // Help text
            ])
            .split(inner);

        // Message
        let message_widget = Paragraph::new(self.message)
            .style(theme.text())
            .wrap(Wrap { trim: true });
        frame.render_widget(message_widget, chunks[0]);

        // Input field or simple buttons
        if let Some(confirm_text) = self.confirm_text {
            // Typed confirmation
            let prompt = format!("Type \"{}\" to confirm:", confirm_text);
            let input_display = format!("{}\n[{}]", prompt, self.input);

            let input_style = if self.input == confirm_text {
                theme.success()
            } else {
                theme.text()
            };

            let input_widget = Paragraph::new(input_display)
                .style(input_style)
                .alignment(Alignment::Center);
            frame.render_widget(input_widget, chunks[1]);
        }

        // Help text
        let help = if self.confirm_text.is_some() {
            "[Esc] Cancel    [Enter] Confirm"
        } else {
            "[Y] Yes    [N] No    [Esc] Cancel"
        };
        let help_widget = Paragraph::new(help)
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(help_widget, chunks[2]);
    }
}

/// Get dialog configuration for an action
pub fn dialog_for_action<'a>(action: &ConfirmAction, input: &'a str) -> ConfirmDialog<'a> {
    match action {
        ConfirmAction::Quit => ConfirmDialog::simple(
            "Quit Application",
            "Are you sure you want to quit Sigil Mother?",
        ),
        ConfirmAction::NullifyChild => ConfirmDialog::typed(
            "NULLIFY CHILD",
            "You are about to PERMANENTLY DISABLE this child.\n\n\
             This operation is IRREVERSIBLE. The disk will never\n\
             be able to sign transactions again. Any remaining\n\
             funds MUST be transferred before nullification.",
            "NULLIFY",
            input,
        ),
        ConfirmAction::FactoryReset => ConfirmDialog::typed(
            "FACTORY RESET",
            "You are about to ERASE ALL DATA including:\n\n\
             - Master key shards\n\
             - Child registry\n\
             - All configuration\n\n\
             This operation is IRREVERSIBLE.",
            "FACTORY RESET",
            input,
        ),
        ConfirmAction::RefillDisk => ConfirmDialog::simple(
            "Refill Disk",
            "This will add new presignatures to the disk.\n\n\
             The reconciliation analysis passed. Proceed with refill?",
        ),
        ConfirmAction::ChangePIN => ConfirmDialog::simple(
            "Change PIN",
            "You will be prompted to enter your current PIN,\n\
             then set a new PIN.\n\n\
             Continue?",
        ),
    }
}
