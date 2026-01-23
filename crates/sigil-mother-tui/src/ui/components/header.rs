//! Header component

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Render the header bar
pub fn render(frame: &mut Frame, area: Rect, title: &str) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M:%S").to_string();

    let header_text = Line::from(vec![
        Span::styled(
            " SIGIL MOTHER ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(area.width.saturating_sub(title.len() as u16 + 24) as usize)),
        Span::styled(format!(" {} ", time_str), Style::default().fg(Color::Cyan)),
    ]);

    let header = Paragraph::new(header_text).style(Style::default().bg(Color::DarkGray));

    frame.render_widget(header, area);
}
