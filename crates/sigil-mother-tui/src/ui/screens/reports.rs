//! Reports screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::ui::layout::{render_footer, render_header, ScreenLayout};

/// Report types
const REPORT_TYPES: &[(&str, &str)] = &[
    (
        "Child Inventory Report",
        "Complete list of all children with status and presig counts",
    ),
    (
        "Signature Audit Trail",
        "All signatures with timestamps, chains, and TX hashes",
    ),
    (
        "Reconciliation History",
        "All reconciliation events and their outcomes",
    ),
    (
        "Security Events Log",
        "Authentication attempts and session activity",
    ),
];

/// Draw the reports screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    render_header(frame, layout.header, "Dashboard > Reports", None, theme);

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout.content);

    // Left: Report type selection
    render_report_types(frame, content_chunks[0], app);

    // Right: Options
    render_options(frame, content_chunks[1], app);

    // Footer
    let hints = &[
        ("↑/↓", "Navigate"),
        ("Enter", "Generate"),
        ("E", "Export to USB"),
        ("Esc", "Back"),
    ];
    render_footer(frame, layout.footer, hints, theme);
}

/// Render report type selection
fn render_report_types(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Report Type ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = REPORT_TYPES
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let marker = if i == app.state.report_type_index {
                "● "
            } else {
                "○ "
            };
            ListItem::new(vec![
                Line::from(vec![Span::raw(marker), Span::styled(*name, theme.text())]),
                Line::from(Span::styled(format!("  {}", desc), theme.text_muted())),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items).highlight_style(theme.selection());

    let mut state = ListState::default().with_selected(Some(app.state.report_type_index));
    frame.render_stateful_widget(list, inner, &mut state);
}

/// Render options panel
fn render_options(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Format options
            Constraint::Length(6), // Export destination
            Constraint::Min(3),    // Status
        ])
        .split(area);

    // Format options
    {
        let block = Block::default()
            .title(" Options ")
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let options = vec![
            Line::from(vec![
                Span::styled("Format:      ", theme.text_secondary()),
                Span::styled("● JSON", theme.text_highlight()),
                Span::raw("   ○ CSV   ○ PDF"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Date Range:  ", theme.text_secondary()),
                Span::raw("[2025-01-01] to [2025-01-21]"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Children:    ", theme.text_secondary()),
                Span::styled("● All", theme.text_highlight()),
                Span::raw("   ○ Select specific..."),
            ]),
        ];

        let options_widget = Paragraph::new(options);
        frame.render_widget(options_widget, inner);
    }

    // Export destination
    {
        let block = Block::default()
            .title(" Export Destination ")
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);

        let dest = vec![
            Line::from(vec![Span::raw("○ Display on screen")]),
            Line::from(vec![Span::styled(
                "● Export to USB drive",
                theme.text_highlight(),
            )]),
            Line::from(vec![
                Span::styled("  Detected: ", theme.text_muted()),
                Span::raw("/media/usb/BACKUP (14.2 GB free)"),
            ]),
        ];

        let dest_widget = Paragraph::new(dest);
        frame.render_widget(dest_widget, inner);
    }

    // Status
    {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(chunks[2]);
        frame.render_widget(block, chunks[2]);

        let status = if let Some(msg) = &app.state.status_message {
            Paragraph::new(msg.as_str())
                .style(theme.success())
                .alignment(Alignment::Center)
        } else {
            Paragraph::new("Press Enter to generate report")
                .style(theme.text_muted())
                .alignment(Alignment::Center)
        };

        frame.render_widget(status, inner);
    }
}
