//! QR code display screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, QrDisplayType};
use crate::ui::components::qr_display::QrCode;
use crate::ui::layout::{render_header, render_footer, ScreenLayout};

/// Draw the QR display screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App, qr_type: &QrDisplayType) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Title based on QR type
    let (title, instructions) = match qr_type {
        QrDisplayType::AgentShard => (
            "Agent Shard QR Code",
            "This QR code contains the AGENT SHARD for signing.\n\
             Scan with your agent device to import.",
        ),
        QrDisplayType::NewChildShard => (
            "New Child Agent Shard",
            "This QR code contains the agent shard for your newly created child.\n\
             Scan with your agent device and import using: sigil import-agent-shard",
        ),
        QrDisplayType::DkgPackage => (
            "DKG Package QR Code",
            "This QR code contains DKG ceremony data.\n\
             Scan with the participant device.",
        ),
    };

    // Header
    render_header(frame, layout.header, title, None, theme);

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Instructions
            Constraint::Min(15),    // QR code
            Constraint::Length(3),  // Security notice
        ])
        .split(layout.content);

    // Instructions
    render_instructions(frame, content_chunks[0], instructions, theme);

    // QR Code
    render_qr(frame, content_chunks[1], app, qr_type);

    // Security notice
    render_security_notice(frame, content_chunks[2], theme);

    // Footer
    let hints = &[
        ("←/→", "Prev/Next"),
        ("S", "Save PNG"),
        ("C", "Copy"),
        ("Esc", "Close"),
    ];
    render_footer(frame, layout.footer, hints, theme);
}

/// Render instructions section
fn render_instructions(frame: &mut Frame, area: Rect, text: &str, theme: &crate::ui::Theme) {
    let block = Block::default()
        .title(" Instructions ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let paragraph = Paragraph::new(text)
        .style(theme.text())
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, inner);
}

/// Render the QR code
fn render_qr(frame: &mut Frame, area: Rect, app: &App, qr_type: &QrDisplayType) {
    let theme = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Generate sample QR data based on type
    let data = match qr_type {
        QrDisplayType::AgentShard => {
            // Sample agent shard data
            "SIGIL:SHARD:a1b2c3d4:7f8e9d0c1b2a3948576d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8g9h0i1j2k3l4m5"
        }
        QrDisplayType::NewChildShard => {
            "SIGIL:SHARD:NEW:a1b2c3d4e5f6g7h8:9a8b7c6d5e4f3g2h1i0j9k8l7m6n5o4p3q2r1s0"
        }
        QrDisplayType::DkgPackage => {
            "SIGIL:DKG:1:3:PACKAGE_DATA_WOULD_BE_HERE_BASE64_ENCODED"
        }
    };

    let qr = QrCode::new(data);

    // Calculate centered position
    let qr_size = qr.size().unwrap_or(21);
    let display_height = (qr_size + 1) / 2;

    // Render QR code
    qr.render(frame, inner, theme);

    // Chunk indicator
    let chunk_text = format!(
        "QR Code {} of {}",
        app.state.qr_chunk_index + 1,
        app.state.qr_total_chunks.max(1)
    );
    let chunk_y = inner.y + inner.height.saturating_sub(1);
    let chunk_widget = Paragraph::new(chunk_text)
        .style(theme.text_secondary())
        .alignment(Alignment::Center);
    frame.render_widget(chunk_widget, Rect::new(inner.x, chunk_y, inner.width, 1));
}

/// Render security notice
fn render_security_notice(frame: &mut Frame, area: Rect, theme: &crate::ui::Theme) {
    let notice = vec![
        Line::from(vec![
            Span::styled("⚠ SECURITY: ", theme.warning()),
            Span::raw("Anyone with this QR code + the floppy disk can sign transactions."),
        ]),
        Line::from(vec![
            Span::raw("   Do not photograph, share, or display in insecure environments."),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.warning());

    let paragraph = Paragraph::new(notice)
        .block(block)
        .style(theme.text())
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
