//! QR code display screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the QR display screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // QR code
            Constraint::Length(5), // Passcode display
            Constraint::Length(3), // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Encrypted Shard QR");

    // QR code placeholder (in production, use qrcode crate to generate)
    let qr_content = if let Some(ref data) = state.qr_data {
        // Would render actual QR code here
        let preview = if data.len() > 50 {
            format!("{}...", &data[..50])
        } else {
            data.clone()
        };

        Paragraph::new(vec![
            Line::from(""),
            Line::from("  [QR Code would be rendered here]"),
            Line::from(""),
            Line::from(format!("  Data: {}", preview)),
            Line::from(""),
            Line::from(format!(
                "  Chunk {}/{} ",
                state.qr_chunk_index + 1,
                state.qr_total_chunks
            )),
        ])
    } else {
        Paragraph::new(vec![
            Line::from(""),
            Line::from("  No QR data to display"),
            Line::from(""),
            Line::from("  QR codes are generated during:"),
            Line::from("    - Child disk creation (encrypted agent shard)"),
            Line::from("    - Accumulator export"),
            Line::from("    - Witness export"),
        ])
    };

    let qr_block = qr_content.block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" QR Code "),
    );

    frame.render_widget(qr_block, chunks[1]);

    // Passcode display (for encrypted shards)
    let passcode = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Passcode: XXXX-XXXX-XXXX-XXXX-XXXX-XXXX",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  (Record this separately from the QR code)"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Decryption Passcode "),
    );

    frame.render_widget(passcode, chunks[2]);

    // Help bar
    let help_text = if state.qr_total_chunks > 1 {
        " [Left/Right] Previous/Next chunk | [Enter/Esc] Done "
    } else {
        " [Enter/Esc] Done "
    };
    let help =
        Paragraph::new(help_text).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}
