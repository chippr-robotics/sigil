//! QR code display component for terminal

use qrcode::{EcLevel, QrCode as QrCodeLib};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::ui::Theme;

/// QR code display component
pub struct QrCode {
    /// The data to encode
    #[allow(dead_code)] // Stored for debugging/reference
    data: String,
    /// Generated QR code matrix
    matrix: Option<Vec<Vec<bool>>>,
    /// Error message if generation failed
    error: Option<String>,
}

impl QrCode {
    /// Create a new QR code from data
    pub fn new(data: impl Into<String>) -> Self {
        let data = data.into();
        let (matrix, error) = match QrCodeLib::with_error_correction_level(&data, EcLevel::M) {
            Ok(code) => {
                let matrix: Vec<Vec<bool>> = code
                    .render::<char>()
                    .quiet_zone(false)
                    .module_dimensions(1, 1)
                    .build()
                    .lines()
                    .map(|line| line.chars().map(|c| c != ' ').collect())
                    .collect();
                (Some(matrix), None)
            }
            Err(e) => (None, Some(format!("QR generation failed: {}", e))),
        };

        Self {
            data,
            matrix,
            error,
        }
    }

    /// Get the raw matrix for custom rendering
    pub fn matrix(&self) -> Option<&Vec<Vec<bool>>> {
        self.matrix.as_ref()
    }

    /// Get the QR code size
    pub fn size(&self) -> Option<usize> {
        self.matrix.as_ref().map(|m| m.len())
    }

    /// Render the QR code using Unicode block characters
    /// Uses ▀ (upper half) ▄ (lower half) █ (full) and space for 2 rows at a time
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        match (&self.matrix, &self.error) {
            (Some(matrix), _) => {
                self.render_matrix(frame, area, matrix, theme);
            }
            (None, Some(error)) => {
                let text = Paragraph::new(error.as_str())
                    .style(theme.danger())
                    .alignment(Alignment::Center);
                frame.render_widget(text, area);
            }
            (None, None) => {
                let text = Paragraph::new("No QR code data")
                    .style(theme.text_muted())
                    .alignment(Alignment::Center);
                frame.render_widget(text, area);
            }
        }
    }

    /// Render the QR matrix using half-block characters
    fn render_matrix(&self, frame: &mut Frame, area: Rect, matrix: &[Vec<bool>], theme: &Theme) {
        let qr_height = matrix.len();
        let qr_width = matrix.first().map(|r| r.len()).unwrap_or(0);

        // Each terminal row represents 2 QR rows using half blocks
        let display_height = qr_height.div_ceil(2);
        let display_width = qr_width;

        // Center the QR code
        let start_x = area.x + (area.width.saturating_sub(display_width as u16)) / 2;
        let start_y = area.y + (area.height.saturating_sub(display_height as u16)) / 2;

        for row in 0..display_height {
            let y = start_y + row as u16;
            if y >= area.y + area.height {
                break;
            }

            let top_row = row * 2;
            let bottom_row = row * 2 + 1;

            let mut line = String::with_capacity(display_width);

            for col in 0..qr_width {
                let top = matrix
                    .get(top_row)
                    .and_then(|r| r.get(col))
                    .copied()
                    .unwrap_or(false);
                let bottom = matrix
                    .get(bottom_row)
                    .and_then(|r| r.get(col))
                    .copied()
                    .unwrap_or(false);

                let char = match (top, bottom) {
                    (true, true) => '█',   // Full block
                    (true, false) => '▀',  // Upper half
                    (false, true) => '▄',  // Lower half
                    (false, false) => ' ', // Empty
                };
                line.push(char);
            }

            let text = Paragraph::new(line).style(theme.text());
            frame.render_widget(text, Rect::new(start_x, y, display_width as u16, 1));
        }
    }

    /// Render QR code info below the code
    pub fn render_with_info(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        chunk_info: Option<(usize, usize)>, // (current, total)
        theme: &Theme,
    ) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(10),   // QR code
                Constraint::Length(2), // Chunk info
            ])
            .split(area);

        // Title
        let title_widget = Paragraph::new(title)
            .style(theme.title())
            .alignment(Alignment::Center);
        frame.render_widget(title_widget, layout[0]);

        // QR code
        self.render(frame, layout[1], theme);

        // Chunk info
        if let Some((current, total)) = chunk_info {
            let info = if total > 1 {
                format!("QR Code {} of {} (scan all codes)", current, total)
            } else {
                "QR Code 1 of 1 (complete)".to_string()
            };
            let info_widget = Paragraph::new(info)
                .style(theme.text_secondary())
                .alignment(Alignment::Center);
            frame.render_widget(info_widget, layout[2]);
        }
    }
}

/// Chunk data for large QR codes
pub struct QrChunker {
    /// Maximum bytes per QR code (Version 40 limit)
    #[allow(dead_code)] // Reserved for future configuration
    max_bytes: usize,
    /// Chunks of data
    chunks: Vec<String>,
}

impl QrChunker {
    /// Create a new chunker with default limit
    pub fn new(data: &str) -> Self {
        Self::with_limit(data, 2000) // Conservative limit for QR Version 40
    }

    /// Create with custom byte limit
    pub fn with_limit(data: &str, max_bytes: usize) -> Self {
        let chunks: Vec<String> = data
            .as_bytes()
            .chunks(max_bytes)
            .enumerate()
            .map(|(i, chunk)| {
                let total = data.len().div_ceil(max_bytes);
                format!(
                    "SIGIL:{}:{}:{}",
                    i + 1,
                    total,
                    String::from_utf8_lossy(chunk)
                )
            })
            .collect();

        Self { max_bytes, chunks }
    }

    /// Get number of chunks
    pub fn count(&self) -> usize {
        self.chunks.len()
    }

    /// Get a specific chunk
    pub fn get(&self, index: usize) -> Option<&str> {
        self.chunks.get(index).map(|s| s.as_str())
    }

    /// Create QR codes for all chunks
    pub fn qr_codes(&self) -> Vec<QrCode> {
        self.chunks.iter().map(QrCode::new).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_code_generation() {
        let qr = QrCode::new("Hello, World!");
        assert!(qr.matrix.is_some());
        assert!(qr.error.is_none());
    }

    #[test]
    fn test_qr_chunker() {
        let data = "A".repeat(5000);
        let chunker = QrChunker::new(&data);
        assert!(chunker.count() > 1);

        for i in 0..chunker.count() {
            let chunk = chunker.get(i).unwrap();
            assert!(chunk.starts_with("SIGIL:"));
        }
    }
}
