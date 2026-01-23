//! Child detail screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::layout::{render_footer, render_header, ScreenLayout};

/// Draw the child detail screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App, _index: usize) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    render_header(
        frame,
        layout.header,
        "Dashboard > Children > a1b2c3d4",
        None,
        theme,
    );

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Basic info
            Constraint::Length(10), // Status
            Constraint::Min(5),     // Actions
        ])
        .split(layout.content);

    // Basic information
    render_info(frame, content_chunks[0], app);

    // Status and metrics
    render_status(frame, content_chunks[1], app);

    // Actions
    render_actions(frame, content_chunks[2], app);

    // Footer
    let hints = &[
        ("Q", "Show QR"),
        ("H", "History"),
        ("N", "Nullify"),
        ("Esc", "Back"),
    ];
    render_footer(frame, layout.footer, hints, theme);
}

/// Render basic child information
fn render_info(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Child Information ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(vec![
            Span::styled("Child ID:          ", theme.text_secondary()),
            Span::styled("a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6", theme.text_highlight()),
        ]),
        Line::from(vec![
            Span::styled("Short ID:          ", theme.text_secondary()),
            Span::raw("a1b2c3d4"),
        ]),
        Line::from(vec![
            Span::styled("Derivation Path:   ", theme.text_secondary()),
            Span::raw("m/44'/60'/0'/0"),
        ]),
        Line::from(vec![
            Span::styled("Signature Scheme:  ", theme.text_secondary()),
            Span::raw("ECDSA secp256k1"),
        ]),
        Line::from(vec![
            Span::styled("Address:           ", theme.text_secondary()),
            Span::raw("0x742d35Cc6634C0532925a3b844Bc9e7595f01234"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Created:           ", theme.text_secondary()),
            Span::raw("2025-01-15 14:23:00 UTC"),
        ]),
        Line::from(vec![
            Span::styled("Last Reconciled:   ", theme.text_secondary()),
            Span::raw("2025-01-18 09:00:00 UTC"),
        ]),
    ];

    let info = Paragraph::new(lines);
    frame.render_widget(info, inner);
}

/// Render status and metrics
fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Status panel
    {
        let block = Block::default()
            .title(" Status ")
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let lines = vec![
            Line::from(vec![
                Span::styled("Status:      ", theme.text_secondary()),
                Span::styled("● ACTIVE", theme.success()),
            ]),
            Line::from(vec![
                Span::styled("Presigs:     ", theme.text_secondary()),
                Span::raw("847/1000 (84.7%)"),
            ]),
            Line::from(vec![
                Span::styled("Expires In:  ", theme.text_secondary()),
                Span::raw("23 days"),
            ]),
            Line::from(vec![
                Span::styled("Recon Due:   ", theme.text_secondary()),
                Span::raw("12 days"),
            ]),
        ];

        let status = Paragraph::new(lines);
        frame.render_widget(status, inner);
    }

    // Metrics panel
    {
        let block = Block::default()
            .title(" Metrics ")
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);

        let lines = vec![
            Line::from(vec![
                Span::styled("Total Signatures:  ", theme.text_secondary()),
                Span::raw("153"),
            ]),
            Line::from(vec![
                Span::styled("Refill Count:      ", theme.text_secondary()),
                Span::raw("0"),
            ]),
            Line::from(vec![
                Span::styled("Last Signature:    ", theme.text_secondary()),
                Span::raw("2025-01-20 15:30"),
            ]),
            Line::from(vec![
                Span::styled("Chains Used:       ", theme.text_secondary()),
                Span::raw("ETH, ARB, BASE"),
            ]),
        ];

        let metrics = Paragraph::new(lines);
        frame.render_widget(metrics, inner);
    }
}

/// Render available actions
fn render_actions(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Available Actions ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let actions = vec![
        Line::from(vec![
            Span::styled("[Q] ", theme.text_highlight()),
            Span::raw("Show Agent Shard QR Code"),
            Span::styled(" - Display QR for agent import", theme.text_muted()),
        ]),
        Line::from(vec![
            Span::styled("[H] ", theme.text_highlight()),
            Span::raw("View Signature History"),
            Span::styled(" - Browse all transactions signed", theme.text_muted()),
        ]),
        Line::from(vec![
            Span::styled("[N] ", theme.text_highlight()),
            Span::styled("Nullify Child", theme.danger()),
            Span::styled(" - Permanently disable (DANGEROUS)", theme.text_muted()),
        ]),
    ];

    let actions_widget = Paragraph::new(actions);
    frame.render_widget(actions_widget, inner);
}
