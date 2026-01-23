//! Layout helpers for consistent screen structure

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::Theme;

/// Standard screen layout with header, content, and footer
pub struct ScreenLayout {
    /// Header area
    pub header: Rect,
    /// Main content area
    pub content: Rect,
    /// Footer/help area
    pub footer: Rect,
}

impl ScreenLayout {
    /// Create a standard layout from the total area
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Content
                Constraint::Length(2), // Footer
            ])
            .split(area);

        Self {
            header: chunks[0],
            content: chunks[1],
            footer: chunks[2],
        }
    }

    /// Create a layout with a sidebar
    pub fn with_sidebar(area: Rect, sidebar_width: u16) -> (Self, Rect) {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(sidebar_width), Constraint::Min(40)])
            .split(area);

        (Self::new(horizontal[1]), horizontal[0])
    }
}

/// Create a centered box for dialogs
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Create a fixed-size centered box
pub fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Render a standard header bar
pub fn render_header(
    frame: &mut Frame,
    area: Rect,
    _title: &str,
    breadcrumb: Option<&str>,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme.border())
        .style(Style::default().bg(theme.sigil_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Min(20),
            Constraint::Length(12),
        ])
        .split(inner);

    // Logo/title
    let logo = Paragraph::new(format!(" {} SIGIL MOTHER", '\u{25C6}')).style(theme.title());
    frame.render_widget(logo, chunks[0]);

    // Breadcrumb
    if let Some(crumb) = breadcrumb {
        let breadcrumb_text = Paragraph::new(crumb)
            .style(theme.text_secondary())
            .alignment(Alignment::Center);
        frame.render_widget(breadcrumb_text, chunks[1]);
    }

    // Time
    let time = chrono::Local::now().format("%H:%M").to_string();
    let time_widget = Paragraph::new(time)
        .style(theme.text_muted())
        .alignment(Alignment::Right);
    frame.render_widget(time_widget, chunks[2]);
}

/// Render a standard footer with help hints
pub fn render_footer(frame: &mut Frame, area: Rect, hints: &[(&str, &str)], theme: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let hint_text: String = hints
        .iter()
        .map(|(key, action)| format!("[{}] {}", key, action))
        .collect::<Vec<_>>()
        .join("  ");

    let footer = Paragraph::new(hint_text)
        .style(theme.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(footer, inner);
}

/// Render a status bar at the bottom
pub fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    status: Option<&str>,
    error: Option<&str>,
    session_warning: Option<&str>,
    theme: &Theme,
) {
    let style = if error.is_some() {
        theme.danger()
    } else if session_warning.is_some() {
        theme.warning()
    } else {
        theme.text_secondary()
    };

    let text = error.or(session_warning).or(status).unwrap_or("");

    let status_bar = Paragraph::new(text)
        .style(style)
        .alignment(Alignment::Center);

    frame.render_widget(status_bar, area);
}

/// Create a section block with title
pub fn section_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(theme.text_highlight())
        .borders(Borders::ALL)
        .border_style(theme.border())
}

/// Create a focused section block
pub fn section_block_focused<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .title(format!(" {} ", title))
        .title_style(theme.text_highlight())
        .borders(Borders::ALL)
        .border_style(theme.border_focused())
}

/// Calculate the number of visible lines in an area
pub fn visible_lines(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize // Account for borders
}

/// Create a two-column layout
pub fn two_column_layout(area: Rect, left_ratio: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_ratio),
            Constraint::Percentage(100 - left_ratio),
        ])
        .split(area);
    (chunks[0], chunks[1])
}

/// Create a three-row layout
pub fn three_row_layout(area: Rect, top: u16, middle: u16) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top),
            Constraint::Length(middle),
            Constraint::Min(3),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}
