//! Visual theme and color palette

use ratatui::style::{Color, Modifier, Style};

/// Sigil color palette
pub struct Theme {
    // Primary branding colors
    pub sigil_gold: Color,
    pub sigil_amber: Color,
    pub sigil_dark: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub info: Color,

    // UI element colors
    pub border: Color,
    pub border_focused: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub highlight: Color,
    pub selection: Color,

    // Floppy disk visualization
    pub floppy_body: Color,
    pub floppy_label: Color,
    pub floppy_shutter: Color,

    // Progress bar colors
    pub progress_filled: Color,
    pub progress_empty: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Primary branding - Sigil Gold
            sigil_gold: Color::Rgb(255, 193, 7),  // #FFC107
            sigil_amber: Color::Rgb(255, 152, 0), // #FF9800
            sigil_dark: Color::Rgb(33, 33, 33),   // #212121

            // Status colors
            success: Color::Rgb(76, 175, 80), // #4CAF50 - Green
            warning: Color::Rgb(255, 152, 0), // #FF9800 - Orange
            danger: Color::Rgb(244, 67, 54),  // #F44336 - Red
            info: Color::Rgb(33, 150, 243),   // #2196F3 - Blue

            // UI elements
            border: Color::Rgb(66, 66, 66),            // #424242
            border_focused: Color::Rgb(255, 193, 7),   // #FFC107
            text_primary: Color::Rgb(250, 250, 250),   // #FAFAFA
            text_secondary: Color::Rgb(189, 189, 189), // #BDBDBD
            text_muted: Color::Rgb(117, 117, 117),     // #757575
            highlight: Color::Rgb(255, 193, 7),        // #FFC107
            selection: Color::Rgb(55, 55, 55),         // #373737

            // Floppy disk
            floppy_body: Color::Rgb(30, 30, 30),
            floppy_label: Color::Rgb(255, 193, 7),
            floppy_shutter: Color::Rgb(192, 192, 192),

            // Progress bars
            progress_filled: Color::Rgb(255, 193, 7),
            progress_empty: Color::Rgb(66, 66, 66),
        }
    }
}

impl Theme {
    /// Get default text style
    pub fn text(&self) -> Style {
        Style::default().fg(self.text_primary)
    }

    /// Get secondary text style
    pub fn text_secondary(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Get muted text style
    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.text_muted)
    }

    /// Get highlighted text style
    pub fn text_highlight(&self) -> Style {
        Style::default()
            .fg(self.sigil_gold)
            .add_modifier(Modifier::BOLD)
    }

    /// Get title style
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.sigil_gold)
            .add_modifier(Modifier::BOLD)
    }

    /// Get subtitle style
    pub fn subtitle(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Get border style
    pub fn border(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Get focused border style
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    /// Get success style
    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Get warning style
    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Get danger style
    pub fn danger(&self) -> Style {
        Style::default()
            .fg(self.danger)
            .add_modifier(Modifier::BOLD)
    }

    /// Get info style
    pub fn info(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Get selection/highlight style
    pub fn selection(&self) -> Style {
        Style::default().bg(self.selection).fg(self.sigil_gold)
    }

    /// Get menu item style
    pub fn menu_item(&self, selected: bool) -> Style {
        if selected {
            Style::default()
                .bg(self.selection)
                .fg(self.sigil_gold)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.text_primary)
        }
    }

    /// Get status indicator style
    pub fn status_active(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Get status warning style
    pub fn status_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Get status error style
    pub fn status_error(&self) -> Style {
        Style::default().fg(self.danger)
    }

    /// Get input field style
    pub fn input(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.text_primary).bg(self.sigil_dark)
        } else {
            Style::default().fg(self.text_secondary).bg(self.sigil_dark)
        }
    }

    /// Get PIN dot style
    pub fn pin_dot(&self) -> Style {
        Style::default()
            .fg(self.sigil_gold)
            .add_modifier(Modifier::BOLD)
    }

    /// Get PIN placeholder style
    pub fn pin_placeholder(&self) -> Style {
        Style::default().fg(self.text_muted)
    }

    /// Create a dark theme variant
    pub fn dark() -> Self {
        Self::default()
    }

    /// Create a high-contrast theme variant
    pub fn high_contrast() -> Self {
        Self {
            text_primary: Color::White,
            text_secondary: Color::White,
            text_muted: Color::Gray,
            border: Color::White,
            border_focused: Color::Yellow,
            ..Self::default()
        }
    }
}
