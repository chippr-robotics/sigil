//! Child creation wizard screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::ui::layout::{render_footer, render_header, ScreenLayout};

/// Wizard step titles
const STEP_TITLES: &[&str] = &[
    "Introduction",
    "Configuration",
    "Review",
    "Insert Disk",
    "Generating",
    "Complete",
];

/// Signature scheme options
const SCHEMES: &[(&str, &str)] = &[
    ("ECDSA secp256k1", "EVM chains (Ethereum, Arbitrum, etc.)"),
    ("Taproot/Schnorr", "Bitcoin BIP-340"),
    ("Ed25519", "Solana, Cosmos"),
];

/// Draw the create child wizard
pub fn draw(frame: &mut Frame, area: Rect, app: &App, step: u8) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    let breadcrumb = format!(
        "Dashboard > Children > Create (Step {} of {})",
        step + 1,
        STEP_TITLES.len()
    );
    render_header(frame, layout.header, &breadcrumb, None, theme);

    // Main content area
    let content = layout.content;

    // Render step content
    match step {
        0 => render_step_intro(frame, content, app),
        1 => render_step_config(frame, content, app),
        2 => render_step_review(frame, content, app),
        3 => render_step_insert(frame, content, app),
        4 => render_step_generating(frame, content, app),
        5 => render_step_complete(frame, content, app),
        _ => {}
    }

    // Footer
    let hints = match step {
        0 => vec![("Enter", "Continue"), ("Esc", "Cancel")],
        1 => vec![("↑/↓", "Select"), ("Enter", "Next"), ("Esc", "Back")],
        2 => vec![("Enter", "Begin Creation"), ("Esc", "Back")],
        3 => vec![("Esc", "Cancel")],
        4 => vec![], // No interaction during generation
        5 => vec![("Enter", "View QR Code"), ("Esc", "Done")],
        _ => vec![("Esc", "Back")],
    };
    render_footer(frame, layout.footer, &hints, theme);
}

/// Step 1: Introduction
fn render_step_intro(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" What is a Child? ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = r#"
A "child" is a derived signing key pair managed by a single floppy disk.

Each child has:
  • Unique blockchain address (where funds are stored)
  • Set of presignatures (limited signing capacity)
  • Expiration date (must reconcile before expiry)

The child key is split between:
  • COLD SHARD - stored on the floppy disk (physical containment)
  • AGENT SHARD - given to your AI agent (encrypted transfer)

Neither party can sign alone - BOTH are required for valid signatures.


┌─────────────────┐          ┌─────────────────┐
│   COLD SHARD    │    +     │   AGENT SHARD   │    =    Valid Signature
│   (Floppy Disk) │          │   (AI Agent)    │
└─────────────────┘          └─────────────────┘


This ensures physical containment: the AI agent cannot sign transactions
unless you physically insert the correct floppy disk.

Press Enter to configure your new child.
"#;

    let paragraph = Paragraph::new(content)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Step 2: Configuration
fn render_step_config(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Scheme selection
            Constraint::Length(6),  // Presig count
            Constraint::Length(6),  // Validity period
            Constraint::Min(1),     // Spacer
        ])
        .split(area);

    // Scheme selection
    {
        let block = Block::default()
            .title(" Signature Scheme ")
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border_focused());

        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        let items: Vec<ListItem> = SCHEMES
            .iter()
            .enumerate()
            .map(|(i, (name, desc))| {
                let marker = if i == app.state.selected_scheme {
                    "● "
                } else {
                    "○ "
                };
                ListItem::new(vec![
                    Line::from(vec![Span::raw(marker), Span::styled(*name, theme.text())]),
                    Line::from(Span::styled(format!("    {}", desc), theme.text_muted())),
                ])
            })
            .collect();

        let list = List::new(items).highlight_style(theme.selection());

        let mut state = ListState::default().with_selected(Some(app.state.selected_scheme));
        frame.render_stateful_widget(list, inner, &mut state);
    }

    // Presig count
    {
        let block = Block::default()
            .title(" Presignature Count ")
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(chunks[1]);
        frame.render_widget(block, chunks[1]);

        let presig_text = format!(
            "Number of presignatures: [{}]\n\
             Recommended: 1000 (allows ~1000 transactions before refill)",
            app.state.presig_count
        );
        let presig = Paragraph::new(presig_text).style(theme.text());
        frame.render_widget(presig, inner);
    }

    // Validity period
    {
        let block = Block::default()
            .title(" Validity Period ")
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(chunks[2]);
        frame.render_widget(block, chunks[2]);

        let validity_text = format!(
            "Days until expiration: [{}]\n\
             Disk must be reconciled before this deadline",
            app.state.validity_days
        );
        let validity = Paragraph::new(validity_text).style(theme.text());
        frame.render_widget(validity, inner);
    }
}

/// Step 3: Review
fn render_step_review(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Review Configuration ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let scheme_name = SCHEMES
        .get(app.state.selected_scheme)
        .map(|(n, _)| *n)
        .unwrap_or("Unknown");

    let content = format!(
        r#"
Please review your child configuration:

┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Signature Scheme:     {}                                   │
│  Presig Count:         {}                                   │
│  Validity Period:      {} days                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘

When you press Enter:
  1. A new child key will be derived from the master
  2. {} presignatures will be generated (~30 seconds)
  3. Cold shards will be written to the floppy disk
  4. Agent shard will be displayed as QR code

⚠  Make sure you have a blank formatted disk ready.

Press Enter to begin child creation.
Press Esc to go back and modify settings.
"#,
        scheme_name, app.state.presig_count, app.state.validity_days, app.state.presig_count,
    );

    let paragraph = Paragraph::new(content)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Step 4: Insert disk
fn render_step_insert(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Insert Formatted Disk ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = r#"

Insert a formatted floppy disk to continue.

The disk must be:
  • A 3.5" floppy disk (1.44 MB)
  • Previously formatted with Sigil filesystem
  • Not already assigned to another child


If you need to format a new disk:
  1. Press Esc to cancel
  2. Go to Disk Management > Format
  3. Return to child creation


"#;

    let paragraph = Paragraph::new(content)
        .style(theme.text())
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);

    // Disk status
    let status_y = inner.y + inner.height.saturating_sub(3);
    let status = if app.state.disk_detected {
        Span::styled("● Disk detected - generating...", theme.success())
    } else {
        Span::styled("○ Waiting for disk...", theme.text_muted())
    };
    let status_widget = Paragraph::new(Line::from(status)).alignment(Alignment::Center);
    frame.render_widget(status_widget, Rect::new(inner.x, status_y, inner.width, 1));
}

/// Step 5: Generating
fn render_step_generating(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" Generating Presignatures ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let spinner = spinner_chars[(app.tick as usize / 5) % spinner_chars.len()];

    // Simulate progress
    let progress = ((app.tick % 300) as f64 / 300.0 * 100.0) as u32;

    let spinner_str = spinner.to_string();
    let step3 = if progress > 30 {
        "✓"
    } else {
        spinner_str.as_str()
    };
    let step4 = if progress > 70 {
        "✓"
    } else if progress > 30 {
        spinner_str.as_str()
    } else {
        "○"
    };
    let step5 = if progress > 90 {
        "✓"
    } else if progress > 70 {
        spinner_str.as_str()
    } else {
        "○"
    };

    let content = format!(
        r#"


{} Generating {} presignatures...

Progress: {}%

Steps:
  ✓ Deriving child key from master
  ✓ Allocating child index
  {} Generating presignature pairs
  {} Writing cold shards to disk
  {} Preparing agent shard


DO NOT remove the disk during this process.
"#,
        spinner, app.state.presig_count, progress, step3, step4, step5,
    );

    let paragraph = Paragraph::new(content)
        .style(theme.text())
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);

    // Progress bar
    let bar_width = 50usize;
    let filled = (progress as usize * bar_width / 100).min(bar_width);
    let empty = bar_width - filled;
    let bar = format!(
        "[{}{}] {}%",
        "█".repeat(filled),
        "░".repeat(empty),
        progress
    );

    let bar_y = inner.y + inner.height.saturating_sub(3);
    let bar_widget = Paragraph::new(bar)
        .style(theme.text_highlight())
        .alignment(Alignment::Center);
    frame.render_widget(bar_widget, Rect::new(inner.x, bar_y, inner.width, 1));
}

/// Step 6: Complete
fn render_step_complete(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" ✓ Child Created Successfully ")
        .title_style(theme.success())
        .borders(Borders::ALL)
        .border_style(theme.success());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = r#"

✓ Child created successfully!

Child Details:
  • Child ID:    a1b2c3d4e5f6g7h8
  • Address:     0x742d35Cc6634C0532925a3b844Bc9e7595f01234
  • Presigs:     1000 available
  • Expires:     2025-02-20 (30 days)


IMPORTANT - Next Steps:

  1. Press Enter to display the Agent Shard QR code
  2. Scan the QR code with your agent device
  3. Import using: sigil import-agent-shard
  4. Store the floppy disk in a secure location


⚠  SECURITY: The QR code contains sensitive data.
   Only display it in a secure environment.
   Do not photograph or share it.
"#;

    let paragraph = Paragraph::new(content)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}
