//! Animated Sigil logo component

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::ui::Theme;

/// ASCII art logo frames for animation
const LOGO_LARGE: &str = r#"
    ███████╗██╗ ██████╗ ██╗██╗
    ██╔════╝██║██╔════╝ ██║██║
    ███████╗██║██║  ███╗██║██║
    ╚════██║██║██║   ██║██║██║
    ███████║██║╚██████╔╝██║███████╗
    ╚══════╝╚═╝ ╚═════╝ ╚═╝╚══════╝
"#;

const LOGO_SMALL: &str = r#"
  ╔═══════════╗
  ║  ◆ SIGIL  ║
  ║   ◇   ◇   ║
  ║    ◆◆◆    ║
  ╚═══════════╝
"#;

const FLOPPY_ART: &str = r#"
   ┌─────────┐
   │ ▄▄▄▄▄▄▄ │
   │ █ MPC █ │
   │ ▀▀▀▀▀▀▀ │
   │▄▄▄▄▄▄▄▄▄│
   └─────────┘
"#;

const SUBTITLE: &str = "MOTHER - Air-Gapped MPC Guardian";

/// Animation frames for floppy disk positions
const FLOPPY_POSITIONS: &[(i16, i16)] = &[
    (-2, -1),   // Top-left
    (2, -1),    // Top-right
    (2, 1),     // Bottom-right
    (-2, 1),    // Bottom-left
];

/// Logo component with animation support
pub struct Logo {
    /// Current animation frame
    frame: usize,
    /// Whether to show large or small logo
    large: bool,
    /// Animation speed (frames per tick)
    speed: usize,
}

impl Logo {
    /// Create a new logo component
    pub fn new() -> Self {
        Self {
            frame: 0,
            large: true,
            speed: 15, // Change position every 15 ticks (~4x/sec at 60fps)
        }
    }

    /// Create a small logo
    pub fn small() -> Self {
        Self {
            large: false,
            ..Self::new()
        }
    }

    /// Advance animation frame
    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// Get current floppy position index
    fn floppy_position_index(&self) -> usize {
        (self.frame / self.speed) % FLOPPY_POSITIONS.len()
    }

    /// Render the logo
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.large {
            self.render_large(frame, area, theme);
        } else {
            self.render_small(frame, area, theme);
        }
    }

    /// Render large logo with animated floppies
    fn render_large(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let logo_lines: Vec<&str> = LOGO_LARGE.lines().collect();
        let floppy_lines: Vec<&str> = FLOPPY_ART.lines().collect();

        // Calculate positions
        let logo_height = logo_lines.len() as u16;
        let total_height = logo_height + 4; // Logo + subtitle + spacing

        let start_y = area.y + (area.height.saturating_sub(total_height)) / 2;

        // Render main logo text
        for (i, line) in logo_lines.iter().enumerate() {
            let y = start_y + i as u16;
            if y < area.y + area.height {
                let x = area.x + (area.width.saturating_sub(line.len() as u16)) / 2;
                let text = Paragraph::new(*line).style(theme.title());
                frame.render_widget(
                    text,
                    Rect::new(x, y, line.len() as u16, 1),
                );
            }
        }

        // Render floppy disk (animated position)
        let pos_idx = self.floppy_position_index();
        let (dx, dy) = FLOPPY_POSITIONS[pos_idx];

        // Position floppy to the right of the logo
        let floppy_x = area.x + (area.width / 2) + 20 + dx as u16;
        let floppy_y = (start_y as i16 + dy) as u16;

        for (i, line) in floppy_lines.iter().enumerate() {
            let y = floppy_y + i as u16;
            if y < area.y + area.height && !line.is_empty() {
                let text = Paragraph::new(*line).style(theme.text_highlight());
                frame.render_widget(
                    text,
                    Rect::new(floppy_x, y, line.len() as u16, 1),
                );
            }
        }

        // Render subtitle
        let subtitle_y = start_y + logo_height + 2;
        if subtitle_y < area.y + area.height {
            let x = area.x + (area.width.saturating_sub(SUBTITLE.len() as u16)) / 2;
            let subtitle = Paragraph::new(SUBTITLE).style(theme.text_secondary());
            frame.render_widget(
                subtitle,
                Rect::new(x, subtitle_y, SUBTITLE.len() as u16, 1),
            );
        }
    }

    /// Render small logo
    fn render_small(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines: Vec<&str> = LOGO_SMALL.lines().filter(|l| !l.is_empty()).collect();
        let height = lines.len() as u16;
        let start_y = area.y + (area.height.saturating_sub(height)) / 2;

        for (i, line) in lines.iter().enumerate() {
            let y = start_y + i as u16;
            if y < area.y + area.height {
                let x = area.x + (area.width.saturating_sub(line.len() as u16)) / 2;
                let text = Paragraph::new(*line).style(theme.title());
                frame.render_widget(
                    text,
                    Rect::new(x, y, line.len() as u16, 1),
                );
            }
        }
    }
}

impl Default for Logo {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a simple text logo (no animation state needed)
pub fn render_logo_static(frame: &mut Frame, area: Rect, theme: &Theme, large: bool) {
    let logo = if large { Logo::new() } else { Logo::small() };
    logo.render(frame, area, theme);
}

/// Get the floppy disk emoji based on tick
pub fn floppy_emoji(tick: u64) -> &'static str {
    // Simple animation: spinning disk
    match (tick / 10) % 4 {
        0 => "💾",
        1 => "📀",
        2 => "💿",
        _ => "📀",
    }
}
