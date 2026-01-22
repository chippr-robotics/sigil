//! User interface rendering module

pub mod components;
pub mod layout;
pub mod screens;
pub mod theme;

pub use theme::Theme;

use ratatui::prelude::*;

use crate::app::{App, Screen};

/// Main draw function that renders the current screen
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    match &app.state.current_screen {
        Screen::Splash => {
            screens::splash::draw(frame, area, app);
        }
        Screen::PinEntry => {
            screens::pin_entry::draw(frame, area, app);
        }
        Screen::PinSetup => {
            screens::pin_setup::draw(frame, area, app);
        }
        Screen::Lockout(until) => {
            screens::lockout::draw(frame, area, app, *until);
        }
        Screen::Dashboard => {
            screens::dashboard::draw(frame, area, app);
        }
        Screen::DiskStatus => {
            screens::disk::status::draw(frame, area, app);
        }
        Screen::DiskFormat(step) => {
            screens::disk::format::draw(frame, area, app, *step);
        }
        Screen::ChildList => {
            screens::children::list::draw(frame, area, app);
        }
        Screen::ChildCreate(step) => {
            screens::children::create::draw(frame, area, app, *step);
        }
        Screen::ChildDetail(index) => {
            screens::children::detail::draw(frame, area, app, *index);
        }
        Screen::Reconciliation => {
            screens::reconciliation::draw(frame, area, app);
        }
        Screen::Reports => {
            screens::reports::draw(frame, area, app);
        }
        Screen::QrDisplay(qr_type) => {
            screens::qr::display::draw(frame, area, app, qr_type);
        }
        Screen::Settings => {
            screens::settings::draw(frame, area, app);
        }
        Screen::Help => {
            screens::help::draw(frame, area, app);
        }
        Screen::Confirm(action) => {
            screens::confirm::draw(frame, area, app, action);
        }
    }
}
