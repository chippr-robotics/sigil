//! Sigil Mother TUI - Air-Gapped MPC Guardian Terminal Interface
//!
//! This application provides a secure terminal user interface for managing
//! MPC (Multi-Party Computation) signing operations using physical floppy disks.
//!
//! # Security Notice
//! This interface protects cryptocurrency assets. All operations are designed
//! with security as the primary concern.

// Many utility functions and components are prepared for future use
#![allow(dead_code)]

use std::io;
use std::panic;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod app;
mod auth;
mod operations;
mod reports;
mod ui;
mod utils;

use app::App;

/// Application entry point with panic handling for terminal restoration
fn main() -> Result<()> {
    // Set up panic hook to restore terminal on crash
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Attempt to restore terminal state
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(EnvFilter::from_default_env().add_directive("sigil_mother_tui=info".parse()?))
        .init();

    // Run the application
    let result = run_app();

    // Ensure terminal is restored even on error
    if let Err(e) = &result {
        tracing::error!("Application error: {}", e);
    }

    result
}

/// Main application runner
fn run_app() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run event loop
    let mut app = App::new()?;
    let result = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
