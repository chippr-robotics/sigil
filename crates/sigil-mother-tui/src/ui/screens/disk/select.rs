//! Device selection screen for choosing which removable device to use

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the device selection screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(5),  // Info panel
            Constraint::Min(10),    // Device list
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Select Device");

    // Info panel
    render_info_panel(frame, chunks[1], state);

    // Device list
    render_device_list(frame, chunks[2], state);

    // Help bar
    let help =
        Paragraph::new(" [j/k] Navigate | [Enter] Select | [r] Refresh | [Esc] Cancel ")
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

/// Render the info panel
fn render_info_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let current_device = state
        .selected_device_path
        .as_deref()
        .unwrap_or("None selected");

    let device_count = state.available_devices.len();
    let floppy_count = state
        .available_devices
        .iter()
        .filter(|d| d.is_floppy_size)
        .count();

    let info_text = vec![
        Line::from(format!("  Current device: {}", current_device)),
        Line::from(format!(
            "  Found {} removable device(s), {} floppy-sized",
            device_count, floppy_count
        )),
    ];

    let info_panel = Paragraph::new(info_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Device Selection "),
    );

    frame.render_widget(info_panel, area);
}

/// Render the device list
fn render_device_list(frame: &mut Frame, area: Rect, state: &AppState) {
    if state.available_devices.is_empty() {
        let no_devices = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No removable devices found",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from("  Insert a floppy disk or USB drive and press 'r' to refresh."),
            Line::from(""),
            Line::from("  Tip: USB floppy drives should appear as removable devices."),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Available Devices "),
        );

        frame.render_widget(no_devices, area);
        return;
    }

    let items: Vec<ListItem> = state
        .available_devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            // Format device information
            let mount_status = if device.is_mounted() {
                format!(" [mounted: {}]", device.mountpoint.as_ref().unwrap().display())
            } else {
                " [unmounted]".to_string()
            };

            let label_info = device
                .label
                .as_ref()
                .map(|l| format!(" \"{}\"", l))
                .unwrap_or_default();

            let fstype_info = device
                .fstype
                .as_ref()
                .map(|f| format!(" ({})", f))
                .unwrap_or_default();

            let floppy_marker = if device.is_floppy_size {
                " [FLOPPY]"
            } else {
                ""
            };

            let line = format!(
                "  {} - {}{}{}{}{}",
                device.path,
                device.size_human,
                floppy_marker,
                label_info,
                fstype_info,
                mount_status
            );

            let style = if i == state.device_select_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(if device.is_floppy_size {
                        Color::Green
                    } else {
                        Color::Cyan
                    })
                    .add_modifier(Modifier::BOLD)
            } else if device.is_floppy_size {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Available Devices (green = floppy-sized) "),
    );

    frame.render_widget(list, area);
}
