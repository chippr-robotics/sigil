//! Floppy disk visualization component

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::ui::Theme;

/// Floppy disk ASCII art template
const FLOPPY_TEMPLATE: &[&str] = &[
    "┌─────────────────────────────────┐",
    "│ ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄ │",
    "│ █                           █ │",
    "│ █  ┌───────────────────┐    █ │",
    "│ █  │  {label_line1}  │    █ │",
    "│ █  │  ────────────────  │    █ │",
    "│ █  │  {label_line2}  │    █ │",
    "│ █  │  {label_line3}  │    █ │",
    "│ █  └───────────────────┘    █ │",
    "│ █                           █ │",
    "│ █▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█ │",
    "│ █ ████████████████  ╔═════╗ █ │",
    "│ █ ████████████████  ║     ║ █ │",
    "│ █ ████████████████  ╚═════╝ █ │",
    "│ ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀ │",
    "└─────────────────────────────────┘",
];

/// Compact floppy disk display
const FLOPPY_COMPACT: &[&str] = &[
    "┌───────────┐",
    "│ ▄▄▄▄▄▄▄▄▄ │",
    "│ █ {id} █ │",
    "│ ▀▀▀▀▀▀▀▀▀ │",
    "│▄▄▄▄▄▄▄▄▄▄▄│",
    "└───────────┘",
];

/// Status indicator for disk
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskStatus {
    /// Disk is active and healthy
    Active,
    /// Disk has warnings (low presigs, expiring soon)
    Warning,
    /// Disk needs reconciliation
    NeedsReconciliation,
    /// Disk is nullified/disabled
    Nullified,
    /// No disk detected
    Empty,
}

impl DiskStatus {
    /// Get status indicator character
    pub fn indicator(&self) -> &'static str {
        match self {
            DiskStatus::Active => "●",
            DiskStatus::Warning => "⚠",
            DiskStatus::NeedsReconciliation => "◐",
            DiskStatus::Nullified => "✗",
            DiskStatus::Empty => "○",
        }
    }

    /// Get status label
    pub fn label(&self) -> &'static str {
        match self {
            DiskStatus::Active => "Active",
            DiskStatus::Warning => "Warning",
            DiskStatus::NeedsReconciliation => "Needs Reconciliation",
            DiskStatus::Nullified => "Nullified",
            DiskStatus::Empty => "No Disk",
        }
    }
}

/// Floppy disk visual component
pub struct FloppyDisk {
    /// Child ID (short form)
    pub child_id: Option<String>,
    /// Label line 1 (e.g., "CHILD: a1b2c3d4")
    pub label_line1: String,
    /// Label line 2 (e.g., "Created: 2025-01")
    pub label_line2: String,
    /// Label line 3 (e.g., "Presigs: 847/1000")
    pub label_line3: String,
    /// Disk status
    pub status: DiskStatus,
}

impl FloppyDisk {
    /// Create an empty disk visualization
    pub fn empty() -> Self {
        Self {
            child_id: None,
            label_line1: "INSERT DISK".to_string(),
            label_line2: "".to_string(),
            label_line3: "".to_string(),
            status: DiskStatus::Empty,
        }
    }

    /// Create from disk data
    pub fn from_data(
        child_id: &str,
        created: &str,
        presigs_remaining: u32,
        presigs_total: u32,
        status: DiskStatus,
    ) -> Self {
        Self {
            child_id: Some(child_id.to_string()),
            label_line1: format!("CHILD: {}", &child_id[..8.min(child_id.len())]),
            label_line2: format!("Created: {}", created),
            label_line3: format!("Presigs: {}/{}", presigs_remaining, presigs_total),
            status,
        }
    }

    /// Render the floppy disk
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let style = match self.status {
            DiskStatus::Active => theme.success(),
            DiskStatus::Warning => theme.warning(),
            DiskStatus::NeedsReconciliation => theme.warning(),
            DiskStatus::Nullified => theme.danger(),
            DiskStatus::Empty => theme.text_muted(),
        };

        // Use compact version if area is small
        if area.height < 10 || area.width < 35 {
            self.render_compact(frame, area, theme, style);
        } else {
            self.render_full(frame, area, theme, style);
        }
    }

    /// Render full floppy disk
    fn render_full(&self, frame: &mut Frame, area: Rect, theme: &Theme, style: Style) {
        let start_x = area.x + (area.width.saturating_sub(35)) / 2;
        let start_y = area.y;

        for (i, line) in FLOPPY_TEMPLATE.iter().enumerate() {
            let y = start_y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            // Replace placeholders
            let rendered = line
                .replace("{label_line1}", &self.pad_label(&self.label_line1, 17))
                .replace("{label_line2}", &self.pad_label(&self.label_line2, 17))
                .replace("{label_line3}", &self.pad_label(&self.label_line3, 17));

            let text = Paragraph::new(rendered).style(style);
            frame.render_widget(
                text,
                Rect::new(start_x, y, 35, 1),
            );
        }

        // Status indicator below disk
        let status_y = start_y + FLOPPY_TEMPLATE.len() as u16;
        if status_y < area.y + area.height {
            let status_text = format!("{} {}", self.status.indicator(), self.status.label());
            let status_widget = Paragraph::new(status_text)
                .style(style)
                .alignment(Alignment::Center);
            frame.render_widget(
                status_widget,
                Rect::new(area.x, status_y, area.width, 1),
            );
        }
    }

    /// Render compact floppy disk
    fn render_compact(&self, frame: &mut Frame, area: Rect, _theme: &Theme, style: Style) {
        let id = self.child_id.as_deref().unwrap_or("----");
        let short_id = &id[..4.min(id.len())];

        let start_x = area.x + (area.width.saturating_sub(13)) / 2;
        let start_y = area.y;

        for (i, line) in FLOPPY_COMPACT.iter().enumerate() {
            let y = start_y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let rendered = line.replace("{id}", &format!("{:^4}", short_id));
            let text = Paragraph::new(rendered).style(style);
            frame.render_widget(
                text,
                Rect::new(start_x, y, 13, 1),
            );
        }
    }

    /// Pad label to fixed width
    fn pad_label(&self, label: &str, width: usize) -> String {
        if label.len() >= width {
            label[..width].to_string()
        } else {
            format!("{:^width$}", label, width = width)
        }
    }
}

/// Render disk detection status
pub fn render_disk_indicator(
    frame: &mut Frame,
    area: Rect,
    detected: bool,
    theme: &Theme,
) {
    let (indicator, label, style) = if detected {
        ("●", "Disk: Detected", theme.success())
    } else {
        ("○", "Disk: Not Detected", theme.text_muted())
    };

    let text = format!("{} {}", indicator, label);
    let widget = Paragraph::new(text).style(style);
    frame.render_widget(widget, area);
}
