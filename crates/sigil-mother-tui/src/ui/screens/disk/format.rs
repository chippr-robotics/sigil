//! Disk format confirmation screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Format type options
const FORMAT_OPTIONS: [&str; 2] = [
    "ext2   - Linux ext2 (recommended for Sigil)",
    "FAT12  - FAT12 (maximum compatibility)",
];

/// Render the format confirmation screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(8), // Warning
            Constraint::Length(6), // Format type selection
            Constraint::Min(6),    // Confirmation
            Constraint::Length(3), // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Format Disk");

    // Warning panel
    render_warning(frame, chunks[1]);

    // Format type selection
    render_format_selection(frame, chunks[2], state);

    // Confirmation
    render_confirmation(frame, chunks[3], state);

    // Help bar
    let help = if state.format_confirmed {
        " [Enter] CONFIRM FORMAT | [Esc] Cancel "
    } else {
        " [j/k] Select format type | [y] Confirm warning | [Esc] Cancel "
    };
    let help_widget =
        Paragraph::new(help).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help_widget, chunks[4]);
}

/// Render the warning panel
fn render_warning(frame: &mut Frame, area: Rect) {
    let warning = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  WARNING: THIS WILL DESTROY ALL DATA ON THE DISK!",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  All existing data will be permanently erased."),
        Line::from("  Make sure you have the correct disk inserted."),
        Line::from(""),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" Warning "),
    );

    frame.render_widget(warning, area);
}

/// Render format type selection
fn render_format_selection(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = FORMAT_OPTIONS
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.format_type_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("  {}  ", item)).style(style)
        })
        .collect();

    let menu = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Select Format Type "),
    );

    frame.render_widget(menu, area);
}

/// Render confirmation section
fn render_confirmation(frame: &mut Frame, area: Rect, state: &AppState) {
    let content = if state.format_confirmed {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  READY TO FORMAT",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Press [Enter] to proceed with formatting."),
            Line::from("  Press [Esc] to cancel."),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from("  To proceed, press 'y' to confirm you understand"),
            Line::from("  that all data will be destroyed."),
            Line::from(""),
            Line::from(Span::styled(
                "  Press 'y' to confirm...",
                Style::default().fg(Color::Yellow),
            )),
        ]
    };

    let panel = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if state.format_confirmed {
                Color::Yellow
            } else {
                Color::Cyan
            }))
            .title(" Confirmation "),
    );

    frame.render_widget(panel, area);
}
