//! Context-sensitive help component

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::Screen;
use crate::ui::Theme;

/// Help content for each screen
pub struct HelpContent {
    /// Screen title
    pub title: &'static str,
    /// Description
    pub description: &'static str,
    /// Key bindings
    pub keys: &'static [(&'static str, &'static str)],
}

/// Get help content for a screen
pub fn help_for_screen(screen: &Screen) -> HelpContent {
    match screen {
        Screen::Dashboard => HelpContent {
            title: "Dashboard",
            description: "The main dashboard shows an overview of your Sigil Mother system, \
                         including disk status, recent activity, and quick access to all functions.",
            keys: &[
                ("↑/↓", "Navigate menu"),
                ("Enter", "Select item"),
                ("F1", "Disk Status"),
                ("F2", "Children"),
                ("F3", "Reconciliation"),
                ("F4", "Reports"),
                ("?", "Help"),
                ("Q", "Quit"),
            ],
        },
        Screen::DiskStatus => HelpContent {
            title: "Disk Status",
            description: "View the current status of the inserted floppy disk, including \
                         presignature inventory, expiry information, and validation status.",
            keys: &[
                ("F", "Format new disk"),
                ("R", "Reconcile disk"),
                ("D", "View detailed log"),
                ("E", "Eject safely"),
                ("Esc", "Back to dashboard"),
            ],
        },
        Screen::DiskFormat(_) => HelpContent {
            title: "Format Disk",
            description: "This wizard guides you through formatting a new floppy disk for \
                         use with Sigil. WARNING: All data on the disk will be erased.",
            keys: &[
                ("Enter/→", "Next step"),
                ("←", "Previous step"),
                ("Esc", "Cancel"),
            ],
        },
        Screen::ChildList => HelpContent {
            title: "Children List",
            description: "View all child signing keys registered with this Mother device. \
                         Each child represents a blockchain address managed by a floppy disk.",
            keys: &[
                ("↑/↓", "Navigate list"),
                ("Enter", "View details"),
                ("N", "Create new child"),
                ("/", "Search"),
                ("F", "Filter"),
                ("Esc", "Back"),
            ],
        },
        Screen::ChildCreate(_) => HelpContent {
            title: "Create Child",
            description: "Create a new child signing key. This will generate a new blockchain \
                         address split between a floppy disk (cold shard) and your AI agent (agent shard).",
            keys: &[
                ("↑/↓", "Select option"),
                ("Enter/→", "Next step"),
                ("←", "Previous step"),
                ("Esc", "Cancel"),
            ],
        },
        Screen::ChildDetail(_) => HelpContent {
            title: "Child Details",
            description: "View detailed information about a child, including its address, \
                         signature history, and status.",
            keys: &[
                ("Q", "Show agent shard QR"),
                ("N", "Nullify child"),
                ("H", "View history"),
                ("Esc", "Back to list"),
            ],
        },
        Screen::Reconciliation => HelpContent {
            title: "Reconciliation",
            description: "Reconciliation verifies that a returning disk has been used correctly. \
                         It checks for anomalies before allowing the disk to be refilled.",
            keys: &[
                ("A", "Analyze disk"),
                ("R", "Refill (after approval)"),
                ("H", "View history"),
                ("Esc", "Back"),
            ],
        },
        Screen::Reports => HelpContent {
            title: "Reports",
            description: "Generate and export reports for auditing, compliance, and record-keeping.",
            keys: &[
                ("↑/↓", "Select report type"),
                ("Enter", "Generate report"),
                ("E", "Export to USB"),
                ("Esc", "Back"),
            ],
        },
        Screen::QrDisplay(_) => HelpContent {
            title: "QR Code Display",
            description: "This QR code contains sensitive data. Scan it with an authorized device only.",
            keys: &[
                ("←/→", "Previous/next chunk"),
                ("S", "Save as image"),
                ("C", "Copy to clipboard"),
                ("Esc", "Close"),
            ],
        },
        Screen::Settings => HelpContent {
            title: "Settings",
            description: "Configure security settings, expiry parameters, and backup options.",
            keys: &[
                ("↑/↓", "Navigate options"),
                ("Enter", "Select option"),
                ("Esc", "Back"),
            ],
        },
        _ => HelpContent {
            title: "Help",
            description: "Press ? on any screen for context-sensitive help.",
            keys: &[
                ("?", "Show help"),
                ("Esc", "Close help"),
            ],
        },
    }
}

/// Render help content
pub fn render_help(frame: &mut Frame, area: Rect, content: &HelpContent, theme: &Theme) {
    let block = Block::default()
        .title(format!(" Help: {} ", content.title))
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4), // Description
            Constraint::Min(5),    // Key bindings
        ])
        .split(inner);

    // Description
    let desc = Paragraph::new(content.description)
        .style(theme.text())
        .wrap(Wrap { trim: true });
    frame.render_widget(desc, chunks[0]);

    // Key bindings
    let keys_text: String = content
        .keys
        .iter()
        .map(|(key, action)| format!("  [{:^8}]  {}", key, action))
        .collect::<Vec<_>>()
        .join("\n");

    let keys = Paragraph::new(keys_text).style(theme.text_secondary());
    frame.render_widget(keys, chunks[1]);
}
