//! Children list screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

use crate::app::App;
use crate::ui::layout::{render_footer, render_header, ScreenLayout};

/// Sample child data for display
struct ChildRow {
    id: &'static str,
    status: &'static str,
    scheme: &'static str,
    presigs: &'static str,
    expires: &'static str,
    signs: &'static str,
}

const SAMPLE_CHILDREN: &[ChildRow] = &[
    ChildRow {
        id: "a1b2c3d4",
        status: "● Active",
        scheme: "ECDSA",
        presigs: "847/1000",
        expires: "23 days",
        signs: "153",
    },
    ChildRow {
        id: "e5f6g7h8",
        status: "● Active",
        scheme: "Taproot",
        presigs: "234/500",
        expires: "12 days",
        signs: "266",
    },
    ChildRow {
        id: "i9j0k1l2",
        status: "⚠ Warn",
        scheme: "ECDSA",
        presigs: "45/1000",
        expires: "3 days",
        signs: "955",
    },
    ChildRow {
        id: "m3n4o5p6",
        status: "◐ Recon",
        scheme: "Ed25519",
        presigs: "0/1000",
        expires: "OVERDUE",
        signs: "1000",
    },
    ChildRow {
        id: "q7r8s9t0",
        status: "✗ Null",
        scheme: "ECDSA",
        presigs: "-/500",
        expires: "N/A",
        signs: "342",
    },
];

/// Draw the children list screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    render_header(
        frame,
        layout.header,
        &format!("Dashboard > Children ({} total)", SAMPLE_CHILDREN.len()),
        None,
        theme,
    );

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Table
            Constraint::Length(3), // Legend
        ])
        .split(layout.content);

    // Children table
    render_table(frame, content_chunks[0], app);

    // Legend
    render_legend(frame, content_chunks[1], app);

    // Footer
    let hints = &[
        ("↑/↓", "Navigate"),
        ("Enter", "View Details"),
        ("N", "Create New"),
        ("/", "Search"),
        ("Esc", "Back"),
    ];
    render_footer(frame, layout.footer, hints, theme);
}

/// Render the children table
fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // Header
    let header_cells = ["ID", "Status", "Scheme", "Presigs", "Expires", "Signs"]
        .iter()
        .map(|h| Cell::from(*h).style(theme.text_highlight()));
    let header = Row::new(header_cells).height(1);

    // Rows
    let rows: Vec<Row> = SAMPLE_CHILDREN
        .iter()
        .enumerate()
        .map(|(i, child)| {
            let status_style = match child.status.chars().next() {
                Some('●') => theme.success(),
                Some('⚠') => theme.warning(),
                Some('◐') => theme.warning(),
                Some('✗') => theme.danger(),
                _ => theme.text(),
            };

            let cells = vec![
                Cell::from(child.id),
                Cell::from(child.status).style(status_style),
                Cell::from(child.scheme),
                Cell::from(child.presigs),
                Cell::from(child.expires),
                Cell::from(child.signs),
            ];

            let style = if i == app.state.child_list_index {
                theme.selection()
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    // Column widths
    let widths = [
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(" All Children ")
                .title_style(theme.title())
                .borders(Borders::ALL)
                .border_style(theme.border()),
        )
        .highlight_style(theme.selection())
        .highlight_symbol("▶ ");

    let mut state = TableState::default().with_selected(Some(
        app.state
            .child_list_index
            .min(SAMPLE_CHILDREN.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(table, area, &mut state);
}

/// Render status legend
fn render_legend(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let legend_items = vec![
        Span::styled("● Active", theme.success()),
        Span::raw("   "),
        Span::styled("⚠ Warning", theme.warning()),
        Span::raw("   "),
        Span::styled("◐ Needs Reconciliation", theme.warning()),
        Span::raw("   "),
        Span::styled("✗ Nullified", theme.danger()),
    ];

    let legend = ratatui::widgets::Paragraph::new(Line::from(legend_items))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(theme.border()),
        );

    frame.render_widget(legend, area);
}
