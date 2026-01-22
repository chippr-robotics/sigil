//! Disk status and management screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Disk action menu items
const DISK_ACTIONS: [&str; 6] = [
    "Select Device   - Choose removable device",
    "Mount Disk      - Mount the floppy disk",
    "Unmount Disk    - Safely unmount the disk",
    "Format Disk     - Format disk for Sigil use",
    "Eject Disk      - Eject the floppy disk",
    "Back            - Return to dashboard",
];

/// Render the disk status screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(12), // Status panel (increased for more info)
            Constraint::Min(8),     // Actions menu
            Constraint::Length(3),  // Help bar
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Disk Management");

    // Status panel
    render_status_panel(frame, chunks[1], state);

    // Actions menu
    render_actions_menu(frame, chunks[2], state);

    // Help bar
    let help =
        Paragraph::new(" [j/k] Navigate | [Enter] Select | [s] Select Device | [m] Mount | [u] Unmount | [r] Refresh | [Esc] Back ")
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}

/// Render the disk status panel
fn render_status_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    // Get selected device info if available
    let selected_device_info = state
        .selected_device_path
        .as_ref()
        .and_then(|path| state.available_devices.iter().find(|d| &d.path == path));

    let (status_text, status_color, details) = match &state.disk_status {
        Some(status) => {
            use sigil_mother::DiskStatus;
            match status {
                DiskStatus::NoDisk => (
                    "NO DISK DETECTED",
                    Color::Red,
                    vec![
                        format!(
                            "Selected Device: {}",
                            state.selected_device_path.as_deref().unwrap_or("None")
                        ),
                        "Insert a floppy disk into the drive.".to_string(),
                        "Press 's' to select a different device.".to_string(),
                    ],
                ),
                DiskStatus::Unmounted { device } => {
                    let mut details = vec![format!("Device: {}", device)];
                    if let Some(dev_info) = selected_device_info {
                        details.push(format!(
                            "Size: {} {}",
                            dev_info.size_human,
                            if dev_info.is_floppy_size {
                                "(floppy)"
                            } else {
                                ""
                            }
                        ));
                        if let Some(label) = &dev_info.label {
                            details.push(format!("Label: {}", label));
                        }
                        if let Some(fstype) = &dev_info.fstype {
                            details.push(format!("Filesystem: {}", fstype));
                        }
                    }
                    details.push("Press 'm' or select 'Mount Disk' to mount.".to_string());
                    ("DISK DETECTED (UNMOUNTED)", Color::Yellow, details)
                }
                DiskStatus::Mounted {
                    device,
                    mount_point,
                    filesystem,
                    is_sigil_disk,
                } => {
                    let sigil_status = if *is_sigil_disk {
                        "Yes (sigil.disk found)"
                    } else {
                        "No (blank or other format)"
                    };
                    let mut details = vec![
                        format!("Device: {}", device),
                        format!("Mount Point: {}", mount_point.display()),
                        format!("Filesystem: {}", filesystem),
                    ];
                    if let Some(dev_info) = selected_device_info {
                        details.push(format!(
                            "Size: {} {}",
                            dev_info.size_human,
                            if dev_info.is_floppy_size {
                                "(floppy)"
                            } else {
                                ""
                            }
                        ));
                        if let Some(label) = &dev_info.label {
                            details.push(format!("Label: {}", label));
                        }
                    }
                    details.push(format!("Sigil Disk: {}", sigil_status));
                    ("DISK MOUNTED", Color::Green, details)
                }
                DiskStatus::Error(e) => ("ERROR", Color::Red, vec![format!("Error: {}", e)]),
            }
        }
        None => (
            "CHECKING...",
            Color::Gray,
            vec!["Checking disk status...".to_string()],
        ),
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Status: "),
            Span::styled(
                status_text,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    for detail in details {
        lines.push(Line::from(format!("  {}", detail)));
    }

    let status_panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Disk Status "),
    );

    frame.render_widget(status_panel, area);
}

/// Render the actions menu
fn render_actions_menu(frame: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = DISK_ACTIONS
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == state.disk_action_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Gray out unavailable actions based on disk status
                let available = is_action_available(i, &state.disk_status);
                if available {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            };
            ListItem::new(format!("  {}  ", item)).style(style)
        })
        .collect();

    let menu = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Actions "),
    );

    frame.render_widget(menu, area);
}

/// Check if an action is available based on disk status
fn is_action_available(action_index: usize, status: &Option<sigil_mother::DiskStatus>) -> bool {
    use sigil_mother::DiskStatus;

    match status {
        None => action_index == 0 || action_index == 5, // Select device and Back always available
        Some(status) => match action_index {
            0 => true,                                           // Select Device always available
            1 => matches!(status, DiskStatus::Unmounted { .. }), // Mount
            2 => matches!(status, DiskStatus::Mounted { .. }),   // Unmount
            3 => !matches!(status, DiskStatus::NoDisk),          // Format (need disk)
            4 => !matches!(status, DiskStatus::NoDisk),          // Eject
            5 => true,                                           // Back always available
            _ => false,
        },
    }
}
