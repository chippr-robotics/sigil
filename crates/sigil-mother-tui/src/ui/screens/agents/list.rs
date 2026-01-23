//! Agent list screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the agent list
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
    header::render(frame, chunks[0], "Agent Registry");

    // Agent table
    let agents = state.agent_registry.list_all();

    if agents.is_empty() {
        let empty_msg =
            Paragraph::new("\n  No agents registered.\n\n  Press 'n' to create a new agent.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" Agents "),
                )
                .style(Style::default().fg(Color::Yellow));
        frame.render_widget(empty_msg, chunks[1]);
    } else {
        let header = Row::new(vec![
            Cell::from("ID").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Children").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Signatures").style(Style::default().add_modifier(Modifier::BOLD)),
        ])
        .style(Style::default().fg(Color::Cyan))
        .height(1);

        let rows: Vec<Row> = agents
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                let status_style = match &agent.status {
                    sigil_core::agent::AgentStatus::Active => Style::default().fg(Color::Green),
                    sigil_core::agent::AgentStatus::Suspended => Style::default().fg(Color::Yellow),
                    sigil_core::agent::AgentStatus::Nullified { .. } => {
                        Style::default().fg(Color::Red)
                    }
                };

                let row_style = if i == state.agent_list_index {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(agent.agent_id.short()),
                    Cell::from(agent.name.clone()),
                    Cell::from(format!("{}", agent.status)).style(status_style),
                    Cell::from(format!("{}", agent.authorized_children.len())),
                    Cell::from(format!("{}", agent.total_signatures)),
                ])
                .style(row_style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Min(20),
                Constraint::Length(15),
                Constraint::Length(10),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(" Agents ({}) ", agents.len())),
        );

        frame.render_widget(table, chunks[1]);
    }

    // Help bar
    let help = Paragraph::new(" [n] New agent | [Enter] View details | [Esc] Back ")
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}
