//! UI rendering

pub mod components;
pub mod screens;

use ratatui::prelude::*;

use crate::app::{AppState, Screen};

/// Main render function - delegates to appropriate screen
pub fn render(frame: &mut Frame, state: &mut AppState) {
    match state.current_screen {
        Screen::Splash => screens::splash::render(frame, state),
        Screen::Dashboard => screens::dashboard::render(frame, state),
        Screen::AgentList => screens::agents::list::render(frame, state),
        Screen::AgentDetail => screens::agents::detail::render(frame, state),
        Screen::AgentCreate => screens::agents::create::render(frame, state),
        Screen::AgentNullify => screens::agents::nullify::render(frame, state),
        Screen::ChildList => screens::children::list::render(frame, state),
        Screen::ChildCreate => screens::children::create::render(frame, state),
        Screen::DiskManagement => screens::disk::status::render(frame, state),
        Screen::DiskSelect => screens::disk::select::render(frame, state),
        Screen::DiskFormat => screens::disk::format::render(frame, state),
        Screen::QrDisplay => screens::qr::display::render(frame, state),
        Screen::Help => screens::help::render(frame, state),
    }
}
