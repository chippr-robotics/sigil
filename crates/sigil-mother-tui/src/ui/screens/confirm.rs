//! Confirmation dialog screen

use ratatui::prelude::*;

use crate::app::{App, ConfirmAction};
use crate::ui::components::confirm::dialog_for_action;

/// Draw the confirmation dialog
pub fn draw(frame: &mut Frame, area: Rect, app: &App, action: &ConfirmAction) {
    let dialog = dialog_for_action(action, &app.state.confirm_input);
    dialog.render(frame, area, &app.theme);
}
