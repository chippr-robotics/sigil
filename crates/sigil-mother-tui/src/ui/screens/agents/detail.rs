//! Agent detail screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

const ACTIONS: [&str; 4] = [
    "View non-membership witness",
    "Suspend / Reactivate",
    "Nullify (PERMANENT)",
    "Back to list",
];

/// Render the agent detail screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(12), // Agent info
            Constraint::Min(8),     // Actions
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Agent Details");

    // Get selected agent
    let agents = state.agent_registry.list_all();
    let agent = match agents.get(state.agent_list_index) {
        Some(a) => *a,
        None => {
            let msg = Paragraph::new("Agent not found").style(Style::default().fg(Color::Red));
            frame.render_widget(msg, chunks[1]);
            return;
        }
    };

    // Agent info
    let status_color = match &agent.status {
        sigil_core::agent::AgentStatus::Active => Color::Green,
        sigil_core::agent::AgentStatus::Suspended => Color::Yellow,
        sigil_core::agent::AgentStatus::Nullified { .. } => Color::Red,
    };

    let created_time = chrono::DateTime::from_timestamp(agent.created_at as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let info_text = vec![
        Line::from(vec![
            Span::raw("  Agent ID:     "),
            Span::styled(agent.agent_id.to_hex(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("  Name:         "),
            Span::styled(
                &agent.name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Status:       "),
            Span::styled(
                format!("{}", agent.status),
                Style::default().fg(status_color),
            ),
        ]),
        Line::from(vec![Span::raw("  Created:      "), Span::raw(created_time)]),
        Line::from(vec![
            Span::raw("  Children:     "),
            Span::raw(format!("{}", agent.authorized_children.len())),
        ]),
        Line::from(vec![
            Span::raw("  Signatures:   "),
            Span::raw(format!("{}", agent.total_signatures)),
        ]),
        Line::from(vec![
            Span::raw("  Acc. Version: "),
            Span::raw(format!("{}", state.agent_registry.accumulator_version())),
        ]),
    ];

    let info = Paragraph::new(info_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" {} ", agent.name)),
    );

    frame.render_widget(info, chunks[1]);

    // Actions list
    let items: Vec<ListItem> = ACTIONS
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == state.agent_action_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if i == 2 {
                // Nullify action in red
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("  {}  ", action)).style(style)
        })
        .collect();

    let actions = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Actions "),
    );

    frame.render_widget(actions, chunks[2]);

    // Help bar
    let help = Paragraph::new(" [Enter] Select action | [Esc] Back ")
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}
