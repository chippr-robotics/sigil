//! Help screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::AppState;
use crate::ui::components::header;

/// Render the help screen
pub fn render(frame: &mut Frame, _state: &mut AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Help content
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Header
    header::render(frame, chunks[0], "Help");

    // Help content
    let content = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  SIGIL MOTHER - Air-Gapped MPC Key Management",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Navigation:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("    j/k or Up/Down  - Move selection"),
        Line::from("    Enter           - Select / Confirm"),
        Line::from("    Esc or b        - Go back"),
        Line::from("    d               - Disk management (from dashboard)"),
        Line::from("    q               - Quit (from dashboard)"),
        Line::from("    ?               - Show this help"),
        Line::from(""),
        Line::from(Span::styled(
            "  Disk Management:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("    m               - Mount floppy disk"),
        Line::from("    u               - Unmount floppy disk"),
        Line::from("    r               - Refresh disk status"),
        Line::from(""),
        Line::from("    Mount:   Mounts disk at /mnt/floppy"),
        Line::from("    Unmount: Safely unmounts before removal"),
        Line::from("    Format:  Formats disk as ext2 or FAT12"),
        Line::from("    Eject:   Unmounts and ejects (USB floppies)"),
        Line::from(""),
        Line::from(Span::styled(
            "  Agent Management:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("    Agents hold the 'hot' shard of presignatures."),
        Line::from("    They participate in signing ceremonies with cold"),
        Line::from("    shares from floppy disks."),
        Line::from(""),
        Line::from("    - Create: Register a new signing agent"),
        Line::from("    - Suspend: Temporarily disable an agent"),
        Line::from("    - Nullify: Permanently revoke (adds to accumulator)"),
        Line::from(""),
        Line::from(Span::styled(
            "  RSA Accumulator:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("    The accumulator tracks nullified agents."),
        Line::from("    Active agents hold non-membership witnesses."),
        Line::from("    When nullified, the witness becomes invalid."),
        Line::from(""),
        Line::from(Span::styled(
            "  Child Disks:",
            Style::default().fg(Color::Cyan),
        )),
        Line::from("    Floppy disks containing cold presig shares."),
        Line::from("    Created during the child creation ceremony."),
        Line::from(""),
        Line::from(Span::styled("  Security:", Style::default().fg(Color::Red))),
        Line::from("    This device should remain AIR-GAPPED."),
        Line::from("    Never connect to any network."),
        Line::from("    Transfer data only via QR codes or floppy disks."),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help "),
    )
    .scroll((0, 0));

    frame.render_widget(content, chunks[1]);

    // Footer
    let footer = Paragraph::new(" Press [Enter] or [Esc] to return ")
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}
