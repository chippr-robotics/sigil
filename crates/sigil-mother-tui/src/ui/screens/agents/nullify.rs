//! Agent nullification screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the agent nullification confirmation screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Nullify Agent");

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

    let warning_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

    let content = if state.nullify_confirmed {
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  FINAL CONFIRMATION", warning_style)),
            Line::from(""),
            Line::from(vec![
                Span::raw("  Agent: "),
                Span::styled(&agent.name, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::raw("  ID:    "),
                Span::raw(agent.agent_id.short()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  This action is PERMANENT and IRREVERSIBLE.",
                warning_style,
            )),
            Line::from(""),
            Line::from("  The agent will be added to the RSA accumulator."),
            Line::from("  All existing non-membership witnesses will be invalidated."),
            Line::from("  Any presignatures bound to this agent will become unusable."),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  Press ENTER to nullify, or ESC to cancel.",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ])
    } else {
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  WARNING: AGENT NULLIFICATION",
                warning_style,
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("  You are about to nullify agent: "),
                Span::styled(&agent.name, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from("  This will:"),
            Line::from("    - Add the agent to the RSA accumulator"),
            Line::from("    - Increment the accumulator version"),
            Line::from("    - Invalidate the agent's non-membership witness"),
            Line::from("    - Prevent all future signing operations with this agent"),
            Line::from(""),
            Line::from(Span::styled(
                "  THIS ACTION CANNOT BE UNDONE.",
                warning_style,
            )),
            Line::from(""),
            Line::from(""),
            Line::from("  Press [y] to proceed, or [Esc] to cancel."),
        ])
    };

    let content = content.block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" NULLIFY AGENT "),
    );

    frame.render_widget(content, chunks[1]);

    // Help bar
    let help_text = if state.nullify_confirmed {
        " [Enter] NULLIFY | [Esc] Cancel "
    } else {
        " [y] Proceed | [Esc] Cancel "
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(Color::White).bg(Color::Red));
    frame.render_widget(help, chunks[2]);
}
