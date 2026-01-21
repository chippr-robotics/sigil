//! Disk status screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::components::floppy::{DiskStatus, FloppyDisk};
use crate::ui::components::progress::PresigInventory;
use crate::ui::layout::{render_footer, render_header, ScreenLayout};

/// Draw the disk status screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    render_header(frame, layout.header, "Dashboard > Disk Status", None, theme);

    // Main content
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(layout.content);

    // Left side: Floppy visualization
    render_disk_visual(frame, content_chunks[0], app);

    // Right side: Detailed info
    render_disk_details(frame, content_chunks[1], app);

    // Footer
    let hints = if app.state.disk_detected {
        vec![
            ("R", "Reconcile"),
            ("D", "View Log"),
            ("E", "Eject"),
            ("Esc", "Back"),
        ]
    } else {
        vec![("F", "Format New"), ("Esc", "Back")]
    };
    render_footer(frame, layout.footer, &hints, theme);
}

/// Render the disk visualization
fn render_disk_visual(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Disk ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.state.disk_detected {
        let status = if app.state.disk_presigs_remaining.unwrap_or(0) < 50 {
            DiskStatus::Warning
        } else {
            DiskStatus::Active
        };

        let floppy = FloppyDisk::from_data(
            app.state.disk_child_id.as_deref().unwrap_or("unknown"),
            "2025-01-15",
            app.state.disk_presigs_remaining.unwrap_or(0),
            app.state.disk_presigs_total.unwrap_or(1000),
            status,
        );
        floppy.render(frame, inner, theme);
    } else {
        let floppy = FloppyDisk::empty();
        floppy.render(frame, inner, theme);
    }
}

/// Render detailed disk information
fn render_disk_details(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Info panel
            Constraint::Length(8),  // Presig inventory
            Constraint::Min(5),     // Expiry status
        ])
        .split(area);

    // Info panel
    render_info_panel(frame, chunks[0], app);

    // Presig inventory
    render_presig_panel(frame, chunks[1], app);

    // Expiry status
    render_expiry_panel(frame, chunks[2], app);
}

/// Render disk information panel
fn render_info_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Disk Information ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.state.disk_detected {
        let child_id = app.state.disk_child_id.as_deref().unwrap_or("unknown");
        let lines = vec![
            Line::from(vec![
                Span::styled("Child ID:     ", theme.text_secondary()),
                Span::styled(child_id, theme.text_highlight()),
            ]),
            Line::from(vec![
                Span::styled("Status:       ", theme.text_secondary()),
                Span::styled("● ACTIVE", theme.success()),
            ]),
            Line::from(vec![
                Span::styled("Scheme:       ", theme.text_secondary()),
                Span::raw("ECDSA secp256k1"),
            ]),
            Line::from(vec![
                Span::styled("Address:      ", theme.text_secondary()),
                Span::raw("0x742d35Cc6634C05..."),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Created:      ", theme.text_secondary()),
                Span::raw("2025-01-15 14:23:00"),
            ]),
            Line::from(vec![
                Span::styled("Last Recon:   ", theme.text_secondary()),
                Span::raw("2025-01-18 09:00:00"),
            ]),
        ];

        let info = Paragraph::new(lines);
        frame.render_widget(info, inner);
    } else {
        let msg = Paragraph::new("No disk inserted")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
    }
}

/// Render presignature inventory panel
fn render_presig_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Presignature Inventory ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.state.disk_detected {
        let inventory = PresigInventory::new(
            app.state.disk_presigs_remaining.unwrap_or(847),
            150,
            3,
            app.state.disk_presigs_total.unwrap_or(1000),
        );
        inventory.render(frame, inner, theme);

        // Warning if low
        if app.state.disk_presigs_remaining.unwrap_or(0) < 50 {
            let warning_y = inner.y + inner.height.saturating_sub(1);
            let warning = "⚠ Emergency reserve active - reconcile soon";
            let warning_widget = Paragraph::new(warning)
                .style(theme.warning())
                .alignment(Alignment::Center);
            frame.render_widget(
                warning_widget,
                Rect::new(inner.x, warning_y, inner.width, 1),
            );
        }
    } else {
        let msg = Paragraph::new("No disk inserted")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
    }
}

/// Render expiry status panel
fn render_expiry_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Expiry Status ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.state.disk_detected {
        let days_remaining = app.state.disk_days_until_expiry.unwrap_or(23);
        let validity_style = if days_remaining < 7 {
            theme.warning()
        } else {
            theme.text()
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("Validity:              ", theme.text_secondary()),
                Span::styled(format!("{} days remaining", days_remaining), validity_style),
            ]),
            Line::from(vec![
                Span::styled("Reconciliation Due:    ", theme.text_secondary()),
                Span::raw("12 days remaining"),
            ]),
            Line::from(vec![
                Span::styled("Max Signatures:        ", theme.text_secondary()),
                Span::raw("350/500 remaining"),
            ]),
        ];

        let info = Paragraph::new(lines);
        frame.render_widget(info, inner);
    } else {
        let msg = Paragraph::new("No disk inserted")
            .style(theme.text_muted())
            .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
    }
}
