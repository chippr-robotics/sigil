//! Reconciliation workflow screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::ui::layout::{render_header, render_footer, ScreenLayout};

/// Draw the reconciliation screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    render_header(frame, layout.header, "Dashboard > Reconciliation", None, theme);

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Explanation
            Constraint::Length(10), // Current analysis
            Constraint::Min(5),     // Actions
        ])
        .split(layout.content);

    // Explanation panel
    render_explanation(frame, content_chunks[0], app);

    // Analysis results
    render_analysis(frame, content_chunks[1], app);

    // Actions
    render_actions(frame, content_chunks[2], app);

    // Footer
    let hints = &[
        ("A", "Analyze Disk"),
        ("R", "Refill"),
        ("H", "History"),
        ("Esc", "Back"),
    ];
    render_footer(frame, layout.footer, hints, theme);
}

/// Render explanation panel
fn render_explanation(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" What is Reconciliation? ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text = r#"
Reconciliation verifies that a returning disk has been used correctly.
The Mother device analyzes the usage log for anomalies before allowing
the disk to be refilled with new presignatures.

This process detects:
  • Missing log entries for used presigs
  • Timestamp irregularities
  • Count mismatches between header and actual usage
"#;

    let paragraph = Paragraph::new(text)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Render analysis results
fn render_analysis(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Analysis Results ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.state.disk_detected {
        // Show mock analysis results
        let lines = vec![
            Line::from(vec![
                Span::styled("Recommendation:  ", theme.text_secondary()),
                Span::styled("✓ REFILL APPROVED", theme.success()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Presig Usage:", theme.text_secondary()),
            ]),
            Line::from(vec![
                Span::raw("  Fresh: 847    Used: 150    Voided: 3    Total: 1000"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Log Entries:     ", theme.text_secondary()),
                Span::styled("150 (matches used count ✓)", theme.success()),
            ]),
            Line::from(vec![
                Span::styled("Anomalies:       ", theme.text_secondary()),
                Span::styled("0 detected", theme.success()),
            ]),
        ];

        let analysis = Paragraph::new(lines);
        frame.render_widget(analysis, inner);
    } else {
        let msg = Paragraph::new("Insert a disk to analyze")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
    }
}

/// Render available actions
fn render_actions(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Actions ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let actions = if app.state.disk_detected {
        vec![
            Line::from(vec![
                Span::styled("[A] ", theme.text_highlight()),
                Span::raw("Re-analyze Disk"),
                Span::styled(" - Run fresh analysis", theme.text_muted()),
            ]),
            Line::from(vec![
                Span::styled("[R] ", theme.text_highlight()),
                Span::raw("Approve & Refill"),
                Span::styled(" - Add new presignatures (requires approval)", theme.text_muted()),
            ]),
            Line::from(vec![
                Span::styled("[E] ", theme.text_highlight()),
                Span::raw("Export Report"),
                Span::styled(" - Save analysis to USB", theme.text_muted()),
            ]),
            Line::from(vec![
                Span::styled("[H] ", theme.text_highlight()),
                Span::raw("View History"),
                Span::styled(" - Past reconciliations for this child", theme.text_muted()),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled("Insert a disk to see available actions", theme.text_muted()),
            ]),
        ]
    };

    let actions_widget = Paragraph::new(actions);
    frame.render_widget(actions_widget, inner);
}
