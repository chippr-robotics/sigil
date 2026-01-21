//! Agent creation screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the agent creation screen
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
    header::render(frame, chunks[0], "Create Agent");

    match state.agent_create_step {
        0 => render_name_input(frame, state, chunks[1]),
        1 => render_confirm(frame, state, chunks[1]),
        _ => {}
    }

    // Help bar
    let help_text = match state.agent_create_step {
        0 => " Type agent name, then press [Enter] | [Esc] Cancel ",
        1 => " [y] Confirm | [n] Go back | [Esc] Cancel ",
        _ => "",
    };
    let help =
        Paragraph::new(help_text).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[2]);
}

fn render_name_input(frame: &mut Frame, state: &AppState, area: Rect) {
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(1),
        ])
        .margin(1)
        .split(area);

    let instructions = Paragraph::new(vec![
        Line::from(""),
        Line::from("  Enter a name for the new agent."),
        Line::from("  This is for identification purposes only."),
    ])
    .block(Block::default());

    frame.render_widget(instructions, content_chunks[0]);

    let input_display = format!(
        "  > {}{}",
        state.agent_name_input,
        if state.agent_name_input.len() < 32 {
            "_"
        } else {
            ""
        }
    );

    let input = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            input_display,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("  {}/32 characters", state.agent_name_input.len())),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Agent Name "),
    );

    frame.render_widget(input, content_chunks[1]);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Step 1: Name ");

    frame.render_widget(outer, area);
}

fn render_confirm(frame: &mut Frame, state: &AppState, area: Rect) {
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from("  You are about to create a new agent:"),
        Line::from(""),
        Line::from(vec![
            Span::raw("    Name: "),
            Span::styled(
                &state.agent_name_input,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from("  The agent will be registered with the mother device"),
        Line::from("  and assigned a non-membership witness."),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "  Create this agent?",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("    [y] Yes, create agent"),
        Line::from("    [n] No, go back"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Step 2: Confirm "),
    );

    frame.render_widget(content, area);
}
