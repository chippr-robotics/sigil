//! Main dashboard screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::ui::components::floppy::{render_disk_indicator, DiskStatus, FloppyDisk};
use crate::ui::layout::{render_footer, render_header, ScreenLayout};

/// Menu items on the dashboard
const MENU_ITEMS: &[(&str, &str, &str)] = &[
    (
        "💾",
        "Disk Management",
        "View disk status and format new disks",
    ),
    ("👶", "Children", "Manage child signing keys"),
    ("🔄", "Reconciliation", "Analyze and refill returning disks"),
    ("📋", "Reports", "Generate and export reports"),
    ("📱", "QR Codes", "Display QR codes for data transfer"),
    ("⚙️", "Settings", "Configure security and system settings"),
];

/// Draw the dashboard
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    render_header(frame, layout.header, "Dashboard", None, theme);

    // Main content area
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(layout.content);

    // Left side: Menu
    render_menu(frame, content_chunks[0], app);

    // Right side: Status panels
    render_status_panels(frame, content_chunks[1], app);

    // Footer
    let hints = &[
        ("↑/↓", "Navigate"),
        ("Enter", "Select"),
        ("F1-F4", "Quick Access"),
        ("?", "Help"),
        ("Q", "Quit"),
    ];
    render_footer(frame, layout.footer, hints, theme);

    // Status message
    if let Some(msg) = &app.state.status_message {
        let msg_y = layout.footer.y.saturating_sub(1);
        let msg_widget = Paragraph::new(msg.as_str())
            .style(theme.success())
            .alignment(Alignment::Center);
        frame.render_widget(msg_widget, Rect::new(area.x, msg_y, area.width, 1));
    }

    // Error message
    if let Some(err) = &app.state.error_message {
        let err_y = layout.footer.y.saturating_sub(1);
        let err_widget = Paragraph::new(err.as_str())
            .style(theme.danger())
            .alignment(Alignment::Center);
        frame.render_widget(err_widget, Rect::new(area.x, err_y, area.width, 1));
    }
}

/// Render the navigation menu
fn render_menu(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Navigation ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create menu items
    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, (icon, title, desc))| {
            let content = if i == app.state.menu_index {
                Line::from(vec![
                    Span::styled(format!(" {} ", icon), theme.text_highlight()),
                    Span::styled(*title, theme.text_highlight()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(format!(" {} ", icon), theme.text()),
                    Span::styled(*title, theme.text()),
                ])
            };
            ListItem::new(vec![
                content,
                Line::from(Span::styled(format!("    {}", desc), theme.text_muted())),
                Line::from(""),
            ])
        })
        .collect();

    let list = List::new(items)
        .highlight_style(theme.selection())
        .highlight_symbol("▶ ");

    let mut state = ListState::default().with_selected(Some(app.state.menu_index));
    frame.render_stateful_widget(list, inner, &mut state);
}

/// Render status panels on the right side
fn render_status_panels(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Disk status
            Constraint::Length(8),  // System status
            Constraint::Min(5),     // Recent activity
        ])
        .split(area);

    // Disk status panel
    render_disk_panel(frame, chunks[0], app);

    // System status panel
    render_system_panel(frame, chunks[1], app);

    // Recent activity panel
    render_activity_panel(frame, chunks[2], app);
}

/// Render disk status panel
fn render_disk_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Current Disk ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.state.disk_detected {
        // Show disk info
        let floppy = FloppyDisk::from_data(
            app.state.disk_child_id.as_deref().unwrap_or("unknown"),
            "2025-01",
            app.state.disk_presigs_remaining.unwrap_or(0),
            app.state.disk_presigs_total.unwrap_or(1000),
            DiskStatus::Active,
        );

        // Compact layout for dashboard
        let info_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(20)])
            .split(inner);

        // Mini floppy
        floppy.render(frame, info_chunks[0], theme);

        // Text info
        let info = vec![
            Line::from(vec![
                Span::raw("Child: "),
                Span::styled(
                    app.state.disk_child_id.as_deref().unwrap_or("----")[..8.min(8)].to_string(),
                    theme.text_highlight(),
                ),
            ]),
            Line::from(vec![
                Span::raw("Presigs: "),
                Span::styled(
                    format!(
                        "{}/{}",
                        app.state.disk_presigs_remaining.unwrap_or(0),
                        app.state.disk_presigs_total.unwrap_or(0)
                    ),
                    theme.text(),
                ),
            ]),
            Line::from(vec![
                Span::raw("Expires: "),
                Span::styled(
                    format!("{} days", app.state.disk_days_until_expiry.unwrap_or(0)),
                    if app.state.disk_days_until_expiry.unwrap_or(30) < 7 {
                        theme.warning()
                    } else {
                        theme.text()
                    },
                ),
            ]),
        ];
        let info_widget = Paragraph::new(info);
        frame.render_widget(info_widget, info_chunks[1]);
    } else {
        // No disk
        let msg = "No disk detected\n\nInsert a Sigil floppy disk";
        let msg_widget = Paragraph::new(msg)
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(msg_widget, inner);
    }
}

/// Render system status panel
fn render_system_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" System Status ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let status_lines = vec![
        Line::from(vec![
            Span::styled("● ", theme.success()),
            Span::raw("Daemon: "),
            Span::styled("Running", theme.success()),
        ]),
        Line::from(vec![
            Span::styled(
                if app.state.disk_detected {
                    "● "
                } else {
                    "○ "
                },
                if app.state.disk_detected {
                    theme.success()
                } else {
                    theme.text_muted()
                },
            ),
            Span::raw("Disk: "),
            Span::styled(
                if app.state.disk_detected {
                    "Detected"
                } else {
                    "Not Detected"
                },
                if app.state.disk_detected {
                    theme.success()
                } else {
                    theme.text_muted()
                },
            ),
        ]),
        Line::from(vec![
            Span::raw("Session: "),
            Span::styled(
                app.session
                    .as_ref()
                    .map(|s| s.remaining_formatted())
                    .unwrap_or_else(|| "N/A".to_string()),
                theme.text(),
            ),
        ]),
    ];

    let status_widget = Paragraph::new(status_lines);
    frame.render_widget(status_widget, inner);
}

/// Render recent activity panel
fn render_activity_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Recent Activity ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Placeholder activity
    let activity = vec!["No recent activity"];

    let activity_widget = Paragraph::new(activity.join("\n"))
        .style(theme.text_muted())
        .alignment(Alignment::Center);
    frame.render_widget(activity_widget, inner);
}
