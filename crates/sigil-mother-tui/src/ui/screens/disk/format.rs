//! Disk format wizard screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::ui::layout::{render_footer, render_header, ScreenLayout};

/// Wizard step titles
const STEP_TITLES: &[&str] = &[
    "Insert Disk",
    "Confirm Erase",
    "Formatting",
    "Validating",
    "Complete",
];

/// Draw the format wizard
pub fn draw(frame: &mut Frame, area: Rect, app: &App, step: u8) {
    let theme = &app.theme;
    let layout = ScreenLayout::new(area);

    // Header
    let breadcrumb = format!(
        "Dashboard > Disk > Format (Step {} of {})",
        step + 1,
        STEP_TITLES.len()
    );
    render_header(frame, layout.header, &breadcrumb, None, theme);

    // Step indicator
    render_step_indicator(frame, layout.content, app, step);

    // Footer hints vary by step
    let hints = match step {
        0 => vec![("Esc", "Cancel")],
        1 => vec![("Enter", "Confirm"), ("Esc", "Cancel")],
        2 | 3 => vec![], // No user interaction during processing
        4 => vec![("Enter", "Done")],
        _ => vec![("Esc", "Back")],
    };
    render_footer(frame, layout.footer, &hints, theme);
}

/// Render step indicator and content
fn render_step_indicator(frame: &mut Frame, area: Rect, app: &App, step: u8) {
    let theme = &app.theme;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress indicator
            Constraint::Min(10),   // Step content
        ])
        .split(area);

    // Progress dots
    let mut progress_line = Vec::new();
    for (i, title) in STEP_TITLES.iter().enumerate() {
        let is_current = i == step as usize;
        let is_complete = i < step as usize;

        let dot = if is_complete {
            Span::styled("● ", theme.success())
        } else if is_current {
            Span::styled("◉ ", theme.text_highlight())
        } else {
            Span::styled("○ ", theme.text_muted())
        };

        progress_line.push(dot);
        progress_line.push(Span::styled(
            format!("{} ", title),
            if is_current {
                theme.text_highlight()
            } else if is_complete {
                theme.success()
            } else {
                theme.text_muted()
            },
        ));

        if i < STEP_TITLES.len() - 1 {
            progress_line.push(Span::styled("→ ", theme.text_muted()));
        }
    }

    let progress = Paragraph::new(Line::from(progress_line)).alignment(Alignment::Center);
    frame.render_widget(progress, chunks[0]);

    // Step content
    match step {
        0 => render_step_insert(frame, chunks[1], app),
        1 => render_step_confirm(frame, chunks[1], app),
        2 => render_step_formatting(frame, chunks[1], app),
        3 => render_step_validating(frame, chunks[1], app),
        4 => render_step_complete(frame, chunks[1], app),
        _ => {}
    }
}

/// Step 1: Insert disk
fn render_step_insert(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" INSERT BLANK FLOPPY DISK ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        "",
        "This wizard will guide you through formatting a new floppy",
        "disk for use as a signing disk.",
        "",
        "What you need:",
        "  • A blank 3.5\" floppy disk (1.44 MB)",
        "  • Physical access to the floppy drive",
        "",
        "What will happen:",
        "  1. Disk will be formatted with SIGIL filesystem",
        "  2. Disk header will be written with mother signature",
        "  3. Disk will be validated for integrity",
        "",
        "⚠  All existing data on the disk will be ERASED",
        "",
    ];

    let text = content.join("\n");
    let paragraph = Paragraph::new(text)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);

    // Disk status indicator
    let status_y = inner.y + inner.height.saturating_sub(2);
    let status = if app.state.disk_detected {
        Span::styled("● Disk detected - press Enter to continue", theme.success())
    } else {
        Span::styled("○ Waiting for disk insertion...", theme.text_muted())
    };
    let status_widget = Paragraph::new(Line::from(status)).alignment(Alignment::Center);
    frame.render_widget(status_widget, Rect::new(inner.x, status_y, inner.width, 1));
}

/// Step 2: Confirm erase
fn render_step_confirm(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" ⚠ CONFIRM DATA ERASURE ")
        .title_style(theme.danger())
        .borders(Borders::ALL)
        .border_style(theme.danger());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        "",
        "WARNING: This operation will ERASE ALL DATA on the disk.",
        "",
        "The following will be destroyed:",
        "  • All existing files",
        "  • Any previous Sigil data",
        "  • Recovery is NOT possible",
        "",
        "Make sure:",
        "  • This is the correct disk",
        "  • You have backed up any important data",
        "  • You understand this action is permanent",
        "",
        "Press Enter to proceed with formatting.",
        "Press Esc to cancel and return.",
        "",
    ];

    let text = content.join("\n");
    let paragraph = Paragraph::new(text)
        .style(theme.text())
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

/// Step 3: Formatting in progress
fn render_step_formatting(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" FORMATTING DISK ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Animated spinner
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let spinner = spinner_chars[(app.tick as usize / 5) % spinner_chars.len()];

    let content = format!(
        "\n\n\n\n\
         {} Formatting disk...\n\n\
         Writing SIGIL filesystem structure\n\
         Creating disk header\n\
         Generating mother signature\n\n\
         Please wait, do not remove the disk.",
        spinner
    );

    let paragraph = Paragraph::new(content)
        .style(theme.text())
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);

    // Progress bar
    let progress = (app.tick % 100) as f64 / 100.0;
    let bar_width = 40usize;
    let filled = (progress * bar_width as f64) as usize;
    let empty = bar_width - filled;
    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

    let bar_y = inner.y + inner.height - 3;
    let bar_widget = Paragraph::new(bar)
        .style(theme.text_highlight())
        .alignment(Alignment::Center);
    frame.render_widget(bar_widget, Rect::new(inner.x, bar_y, inner.width, 1));
}

/// Step 4: Validating
fn render_step_validating(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" VALIDATING DISK ")
        .title_style(theme.title())
        .borders(Borders::ALL)
        .border_style(theme.border_focused());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let spinner_chars = ['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];
    let spinner = spinner_chars[(app.tick as usize / 5) % spinner_chars.len()];

    let content = format!(
        "\n\n\n\n\
         {} Validating disk integrity...\n\n\
         ✓ Filesystem structure verified\n\
         ✓ Header checksum valid\n\
         {} Mother signature verification\n\n\
         Almost done...",
        spinner, spinner
    );

    let paragraph = Paragraph::new(content)
        .style(theme.text())
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

/// Step 5: Complete
fn render_step_complete(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let block = Block::default()
        .title(" ✓ FORMAT COMPLETE ")
        .title_style(theme.success())
        .borders(Borders::ALL)
        .border_style(theme.success());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = vec![
        "",
        "✓ Disk formatted successfully!",
        "",
        "The disk is now ready to be used for child creation.",
        "",
        "Next steps:",
        "  1. Go to Children > Create New Child",
        "  2. Follow the child creation wizard",
        "  3. The disk will be initialized with presignatures",
        "",
        "Press Enter to return to disk status.",
        "",
    ];

    let text = content.join("\n");
    let paragraph = Paragraph::new(text)
        .style(theme.text())
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}
