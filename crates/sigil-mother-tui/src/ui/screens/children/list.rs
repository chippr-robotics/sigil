//! Child disk list screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the child list screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Table
            Constraint::Length(3), // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Child Disks");

    // Child table
    let children = state.child_registry.list_all();

    if children.is_empty() {
        let empty_msg = Paragraph::new(
            "\n  No child disks registered.\n\n  Press 'n' to create a new child disk.",
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Children "),
        )
        .style(Style::default().fg(Color::Yellow));
        frame.render_widget(empty_msg, chunks[1]);
    } else {
        let header = Row::new(vec![
            Cell::from("ID").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Created").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Signatures").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Path").style(Style::default().add_modifier(Modifier::BOLD)),
        ])
        .style(Style::default().fg(Color::Cyan))
        .height(1);

        let rows: Vec<Row> = children
            .iter()
            .enumerate()
            .map(|(i, child)| {
                let status_style = match &child.status {
                    sigil_core::child::ChildStatus::Active => Style::default().fg(Color::Green),
                    sigil_core::child::ChildStatus::Suspended => Style::default().fg(Color::Yellow),
                    sigil_core::child::ChildStatus::Nullified { .. } => {
                        Style::default().fg(Color::Red)
                    }
                };

                let status_text = match &child.status {
                    sigil_core::child::ChildStatus::Active => "Active",
                    sigil_core::child::ChildStatus::Suspended => "Suspended",
                    sigil_core::child::ChildStatus::Nullified { .. } => "Nullified",
                };

                let created_time = chrono::DateTime::from_timestamp(child.created_at as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                let row_style = if i == state.child_list_index {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(child.child_id.short()),
                    Cell::from(status_text).style(status_style),
                    Cell::from(created_time),
                    Cell::from(format!("{}", child.total_signatures)),
                    Cell::from(child.derivation_path.to_string_path()),
                ])
                .style(row_style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(20),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" Children ({}) ", children.len())),
        );

        frame.render_widget(table, chunks[1]);
    }

    // Help bar
    let help = Paragraph::new(" [n] New child | [Enter] View details | [Esc] Back ")
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
