//! Splash screen with animated logo

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::ui::components::logo::Logo;

/// Draw the splash screen
pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    // Create logo with current animation frame
    let mut logo = Logo::new();
    for _ in 0..app.tick {
        logo.tick();
    }

    // Render logo
    logo.render(frame, area, &app.theme);

    // "Press any key to continue" at bottom
    let prompt = "Press any key to continue...";
    let prompt_y = area.y + area.height.saturating_sub(3);
    let prompt_x = area.x + (area.width.saturating_sub(prompt.len() as u16)) / 2;

    // Blinking effect
    let visible = (app.tick / 30).is_multiple_of(2);
    if visible {
        let prompt_widget = Paragraph::new(prompt).style(app.theme.text_muted());
        frame.render_widget(
            prompt_widget,
            Rect::new(prompt_x, prompt_y, prompt.len() as u16, 1),
        );
    }

    // Version info
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let version_y = area.y + area.height.saturating_sub(1);
    let version_x = area.x + area.width.saturating_sub(version.len() as u16 + 1);
    let version_widget = Paragraph::new(version).style(app.theme.text_muted());
    frame.render_widget(version_widget, Rect::new(version_x, version_y, 10, 1));
}
