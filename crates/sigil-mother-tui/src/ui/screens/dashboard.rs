//! Dashboard screen - main menu

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Menu items
const MENU_ITEMS: [&str; 6] = [
    "Children     - Manage child disks",
    "Agents       - Manage signing agents",
    "Reconcile    - Reconcile returned disks",
    "Reports      - Generate audit reports",
    "Help         - View documentation",
    "Quit         - Exit application",
];

/// Render the dashboard
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Menu
            Constraint::Length(3), // Status bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Dashboard");

    // Menu
    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.menu_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("  {}  ", item)).style(style)
        })
        .collect();

    let menu = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Main Menu "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(menu, chunks[1]);

    // Status bar
    let (active_agents, _, nullified_agents) = state.agent_registry.count_by_status();
    let (active_children, _, _) = state.child_registry.count_by_status();

    let status_text = format!(
        " Agents: {} active, {} nullified | Children: {} active | Press ? for help ",
        active_agents, nullified_agents, active_children
    );

    let status =
        Paragraph::new(status_text).style(Style::default().fg(Color::White).bg(Color::DarkGray));

    frame.render_widget(status, chunks[2]);
}
