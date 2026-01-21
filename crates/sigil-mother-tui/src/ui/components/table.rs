//! Table component for data display

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table as RatatuiTable, TableState};

use crate::ui::Theme;

/// Column definition
pub struct Column {
    /// Header text
    pub header: String,
    /// Width constraint
    pub width: Constraint,
    /// Alignment
    pub alignment: Alignment,
}

impl Column {
    /// Create a new column
    pub fn new(header: impl Into<String>, width: Constraint) -> Self {
        Self {
            header: header.into(),
            width,
            alignment: Alignment::Left,
        }
    }

    /// Set alignment
    pub fn align(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Right-aligned column
    pub fn right(mut self) -> Self {
        self.alignment = Alignment::Right;
        self
    }

    /// Center-aligned column
    pub fn center(mut self) -> Self {
        self.alignment = Alignment::Center;
        self
    }
}

/// Data table component
pub struct DataTable<'a> {
    /// Title
    title: &'a str,
    /// Column definitions
    columns: Vec<Column>,
    /// Row data
    rows: Vec<Vec<String>>,
    /// Currently selected row
    selected: usize,
    /// Row styles (optional)
    row_styles: Vec<Option<Style>>,
}

impl<'a> DataTable<'a> {
    /// Create a new data table
    pub fn new(title: &'a str, columns: Vec<Column>) -> Self {
        Self {
            title,
            columns,
            rows: Vec::new(),
            selected: 0,
            row_styles: Vec::new(),
        }
    }

    /// Add a row
    pub fn row(mut self, cells: Vec<impl Into<String>>) -> Self {
        self.rows.push(cells.into_iter().map(|c| c.into()).collect());
        self.row_styles.push(None);
        self
    }

    /// Add a row with custom style
    pub fn styled_row(mut self, cells: Vec<impl Into<String>>, style: Style) -> Self {
        self.rows.push(cells.into_iter().map(|c| c.into()).collect());
        self.row_styles.push(Some(style));
        self
    }

    /// Set selected row
    pub fn select(mut self, index: usize) -> Self {
        self.selected = index.min(self.rows.len().saturating_sub(1));
        self
    }

    /// Get row count
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Render the table
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        // Create header row
        let header_cells: Vec<Cell> = self.columns
            .iter()
            .map(|c| {
                Cell::from(c.header.clone())
                    .style(theme.text_highlight())
            })
            .collect();
        let header = Row::new(header_cells).height(1);

        // Create data rows
        let rows: Vec<Row> = self.rows
            .iter()
            .enumerate()
            .map(|(i, row_data)| {
                let cells: Vec<Cell> = row_data
                    .iter()
                    .enumerate()
                    .map(|(j, cell)| {
                        let alignment = self.columns.get(j)
                            .map(|c| c.alignment)
                            .unwrap_or(Alignment::Left);
                        Cell::from(cell.clone())
                    })
                    .collect();

                let style = if i == self.selected {
                    theme.selection()
                } else {
                    self.row_styles.get(i).copied().flatten().unwrap_or_default()
                };

                Row::new(cells).style(style)
            })
            .collect();

        // Column widths
        let widths: Vec<Constraint> = self.columns
            .iter()
            .map(|c| c.width)
            .collect();

        // Create table
        let table = RatatuiTable::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .title(format!(" {} ", self.title))
                    .title_style(theme.title())
                    .borders(Borders::ALL)
                    .border_style(theme.border())
            )
            .highlight_style(theme.selection());

        // Render with state
        let mut state = TableState::default().with_selected(self.selected);
        frame.render_stateful_widget(table, area, &mut state);
    }
}

/// Simple key-value display table
pub struct InfoTable<'a> {
    /// Title
    title: &'a str,
    /// Key-value pairs
    items: Vec<(&'a str, String, Option<Style>)>,
}

impl<'a> InfoTable<'a> {
    /// Create a new info table
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            items: Vec::new(),
        }
    }

    /// Add an item
    pub fn item(mut self, label: &'a str, value: impl Into<String>) -> Self {
        self.items.push((label, value.into(), None));
        self
    }

    /// Add an item with style
    pub fn styled_item(
        mut self,
        label: &'a str,
        value: impl Into<String>,
        style: Style,
    ) -> Self {
        self.items.push((label, value.into(), Some(style)));
        self
    }

    /// Render the table
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .title_style(theme.title())
            .borders(Borders::ALL)
            .border_style(theme.border());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render items
        for (i, (label, value, style)) in self.items.iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            // Label
            let label_width = 20.min(inner.width as usize / 2);
            let label_text = format!("{:>width$}:", label, width = label_width);
            let label_widget = ratatui::widgets::Paragraph::new(label_text)
                .style(theme.text_secondary());
            frame.render_widget(
                label_widget,
                Rect::new(inner.x, y, label_width as u16 + 1, 1),
            );

            // Value
            let value_x = inner.x + label_width as u16 + 2;
            let value_width = inner.width.saturating_sub(label_width as u16 + 2);
            let value_style = style.unwrap_or_else(|| theme.text());
            let value_widget = ratatui::widgets::Paragraph::new(value.as_str())
                .style(value_style);
            frame.render_widget(
                value_widget,
                Rect::new(value_x, y, value_width, 1),
            );
        }
    }
}
