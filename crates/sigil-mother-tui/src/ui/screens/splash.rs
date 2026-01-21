//! Splash screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;

/// Render the splash screen
pub fn render(frame: &mut Frame, _state: &mut AppState) {
    let area = frame.area();

    let logo = r#"
    ███████╗██╗ ██████╗ ██╗██╗
    ██╔════╝██║██╔════╝ ██║██║
    ███████╗██║██║  ███╗██║██║
    ╚════██║██║██║   ██║██║██║
    ███████║██║╚██████╔╝██║███████╗
    ╚══════╝╚═╝ ╚═════╝ ╚═╝╚══════╝

       MOTHER DEVICE CONSOLE

    Air-Gapped MPC Key Management

    Press ENTER to continue...
"#;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Sigil Mother ");

    let paragraph = Paragraph::new(logo)
        .block(block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green));

    frame.render_widget(paragraph, area);
}
