//! Sigil Mother TUI - Terminal User Interface
//!
//! Entry point for the Sigil Mother air-gapped device TUI.

use std::io;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use sigil_mother_tui::{App, AppResult};

fn main() -> AppResult<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("sigil_mother_tui=debug")
        .with_target(false)
        .init();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let mut app = App::new();
    let result = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
