//! Child disk creation screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the child creation screen
pub fn render(frame: &mut Frame, _state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Create Child Disk");

    // Placeholder content
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from("  Child Disk Creation Wizard"),
        Line::from(""),
        Line::from("  This feature requires:"),
        Line::from("    1. A blank floppy disk inserted"),
        Line::from("    2. Master key to be initialized"),
        Line::from("    3. Agent to be registered"),
        Line::from(""),
        Line::from("  The ceremony will:"),
        Line::from("    - Generate presignatures"),
        Line::from("    - Write cold shares to floppy"),
        Line::from("    - Display encrypted agent shard as QR code"),
        Line::from(""),
        Line::from(Span::styled(
            "  [Coming Soon]",
            Style::default().fg(Color::Yellow),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Create Child "),
    );

    frame.render_widget(content, chunks[1]);

    // Help bar
    let help =
        Paragraph::new(" [Esc] Back ").style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
