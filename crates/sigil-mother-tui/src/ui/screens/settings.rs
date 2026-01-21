//! Settings screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::ui::layout::{render_header, render_footer, ScreenLayout};

/// Settings menu items
const SETTINGS_ITEMS: &[(&str, &str, &str)] = &[
    ("🔐", "Change PIN", "Update your authentication PIN"),
    ("⏱️", "Session Timeout", "Configure auto-lock timing"),
    ("📅", "Expiry Settings", "Default validity period for new children"),
    ("💾", "Backup & Restore", "Backup mother state to external storage"),
    ("⚠️", "Factory Reset", "Erase all data and start fresh (DANGEROUS)"),
];

/// Draw the settings screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    render_header(frame, layout.header, "Dashboard > Settings", None, theme);

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(layout.content);

    // Left: Settings menu
    render_menu(frame, content_chunks[0], app);

    // Right: Detail panel
    render_detail(frame, content_chunks[1], app);

    // Footer
    let hints = &[
        ("↑/↓", "Navigate"),
        ("Enter", "Select"),
        ("Esc", "Back"),
    ];
    render_footer(frame, layout.footer, hints, theme);
}

/// Render settings menu
fn render_menu(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Settings ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = SETTINGS_ITEMS
        .iter()
        .enumerate()
        .map(|(i, (icon, title, _))| {
            let style = if i == app.state.settings_index {
                theme.selection()
            } else if i == 4 {
                // Factory reset is danger colored
                theme.danger()
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::raw(format!("  {} ", icon)),
                Span::styled(*title, style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(theme.selection())
        .highlight_symbol("▶ ");

    let mut state = ListState::default().with_selected(Some(app.state.settings_index));
    frame.render_stateful_widget(list, inner, &mut state);
}

/// Render detail panel for selected setting
fn render_detail(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let (_, title, _) = SETTINGS_ITEMS
        .get(app.state.settings_index)
        .unwrap_or(&("", "Settings", ""));

    let block = Block::default()
        .title(format!(" {} ", title))
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Content based on selected item
    let content = match app.state.settings_index {
        0 => render_change_pin_detail(theme),
        1 => render_session_timeout_detail(theme),
        2 => render_expiry_detail(theme),
        3 => render_backup_detail(theme),
        4 => render_factory_reset_detail(theme),
        _ => vec![],
    };

    let paragraph = Paragraph::new(content);
    frame.render_widget(paragraph, inner);
}

fn render_change_pin_detail(theme: &crate::ui::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Change your authentication PIN.", theme.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("You will need to:"),
        ]),
        Line::from(vec![
            Span::raw("  1. Enter your current PIN"),
        ]),
        Line::from(vec![
            Span::raw("  2. Enter a new PIN (6-12 digits)"),
        ]),
        Line::from(vec![
            Span::raw("  3. Confirm the new PIN"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press Enter to change PIN.", theme.text_highlight()),
        ]),
    ]
}

fn render_session_timeout_detail(theme: &crate::ui::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Current timeout: ", theme.text_secondary()),
            Span::raw("5 minutes"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Available options:"),
        ]),
        Line::from(vec![
            Span::raw("  ○ 2 minutes (high security)"),
        ]),
        Line::from(vec![
            Span::styled("  ● 5 minutes (default)", theme.text_highlight()),
        ]),
        Line::from(vec![
            Span::raw("  ○ 15 minutes (convenience)"),
        ]),
        Line::from(vec![
            Span::raw("  ○ 30 minutes"),
        ]),
    ]
}

fn render_expiry_detail(theme: &crate::ui::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Default validity for new children:", theme.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Validity Period:    ", theme.text_secondary()),
            Span::raw("[30] days"),
        ]),
        Line::from(vec![
            Span::styled("Max Signatures:     ", theme.text_secondary()),
            Span::raw("[500] per disk"),
        ]),
        Line::from(vec![
            Span::styled("Warning Period:     ", theme.text_secondary()),
            Span::raw("[7] days before expiry"),
        ]),
        Line::from(vec![
            Span::styled("Emergency Reserve:  ", theme.text_secondary()),
            Span::raw("[50] presignatures"),
        ]),
    ]
}

fn render_backup_detail(theme: &crate::ui::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Backup Mother State", theme.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Creates a backup of:"),
        ]),
        Line::from(vec![
            Span::raw("  • Master key shards (encrypted)"),
        ]),
        Line::from(vec![
            Span::raw("  • Child registry"),
        ]),
        Line::from(vec![
            Span::raw("  • Reconciliation history"),
        ]),
        Line::from(vec![
            Span::raw("  • Configuration settings"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("⚠ Store backup securely - it contains", theme.warning()),
        ]),
        Line::from(vec![
            Span::styled("  sensitive cryptographic material.", theme.warning()),
        ]),
    ]
}

fn render_factory_reset_detail(theme: &crate::ui::Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("⚠ DANGER: Factory Reset", theme.danger()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("This will PERMANENTLY DELETE:", theme.danger()),
        ]),
        Line::from(vec![
            Span::raw("  • Master key shards"),
        ]),
        Line::from(vec![
            Span::raw("  • All child registrations"),
        ]),
        Line::from(vec![
            Span::raw("  • Reconciliation history"),
        ]),
        Line::from(vec![
            Span::raw("  • All configuration"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("This action CANNOT be undone.", theme.danger()),
        ]),
        Line::from(vec![
            Span::styled("All associated funds may be LOST.", theme.danger()),
        ]),
    ]
}
