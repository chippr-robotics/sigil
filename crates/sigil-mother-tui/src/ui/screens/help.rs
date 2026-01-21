//! Help screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::ui::layout::centered_rect;

/// Draw the help screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    // Semi-transparent overlay effect via centered dialog
    let dialog = centered_rect(70, 80, area);

    let block = Block::default()
        .title(" Help - Keyboard Shortcuts ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(dialog);
    frame.render_widget(block, dialog);

    let content = r#"
GLOBAL SHORTCUTS
────────────────────────────────────────────────────────────
  ?           Show this help screen
  Esc         Go back / Cancel
  Q           Quit application (with confirmation)
  Ctrl+C      Force quit

NAVIGATION
────────────────────────────────────────────────────────────
  ↑ / k       Move up
  ↓ / j       Move down
  ← / h       Move left / Previous
  → / l       Move right / Next
  Tab         Next focus area
  Shift+Tab   Previous focus area
  Enter       Select / Confirm
  /           Search (in lists)

QUICK ACCESS (from Dashboard)
────────────────────────────────────────────────────────────
  F1          Disk Status
  F2          Children List
  F3          Reconciliation
  F4          Reports

DISK MANAGEMENT
────────────────────────────────────────────────────────────
  F           Format new disk
  R           Reconcile disk
  E           Eject safely

CHILD MANAGEMENT
────────────────────────────────────────────────────────────
  N           Create new child
  Q           Show QR code
  H           View history

────────────────────────────────────────────────────────────
Press Esc or ? to close this help screen
"#;

    let help = Paragraph::new(content)
        .style(theme.text())
        .wrap(Wrap { trim: false });

    frame.render_widget(help, inner);
}
