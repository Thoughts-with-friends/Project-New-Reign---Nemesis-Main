//! Column geometry of the mod table.

use eframe::egui::{Align2, Color32, FontId, Rect, Ui, Vec2};

/// Height of one row (and of the header).
pub const ROW_HEIGHT: f32 = 34.0;
/// Width of the drag handle column.
const HANDLE_WIDTH: f32 = 26.0;
/// Width of the checkbox column.
const CHECK_WIDTH: f32 = 34.0;
/// Width of the priority column.
const PRIORITY_WIDTH: f32 = 72.0;
/// Horizontal padding inside text cells.
const CELL_PADDING: f32 = 8.0;

/// A column as `(start, end)` offsets from the left edge of the table.
pub type Span = (f32, f32);

/// Horizontal layout of the table columns.
#[derive(Debug, Clone, Copy)]
pub struct Columns {
    /// Left edge of the table.
    pub left: f32,
    /// Total width of the table.
    pub width: f32,
    /// Drag handle column.
    pub handle: Span,
    /// Checkbox column.
    pub check: Span,
    /// Mod name column.
    pub name: Span,
    /// Author column.
    pub author: Span,
    /// Site column.
    pub site: Span,
    /// Priority column.
    pub priority: Span,
}

impl Columns {
    /// Splits `width` into columns. The flexible part is shared 45/20/35 between
    /// name, author and site.
    pub fn new(left: f32, width: f32) -> Self {
        let fixed = HANDLE_WIDTH + CHECK_WIDTH + PRIORITY_WIDTH;
        let flex = (width - fixed).max(120.0);
        let name_w = flex * 0.45;
        let author_w = flex * 0.20;
        let site_w = flex - name_w - author_w;

        let mut x = 0.0;
        let mut next = |w: f32| {
            let span = (x, x + w);
            x += w;
            span
        };

        Self {
            left,
            width,
            handle: next(HANDLE_WIDTH),
            check: next(CHECK_WIDTH),
            name: next(name_w),
            author: next(author_w),
            site: next(site_w),
            priority: next(PRIORITY_WIDTH),
        }
    }

    /// Returns the rectangle of a column inside a row rectangle.
    pub fn cell(&self, row: Rect, (start, end): Span) -> Rect {
        Rect::from_x_y_ranges(row.left() + start..=row.left() + end, row.y_range())
    }

    /// Returns the padded rectangle for text inside a column.
    pub fn text_cell(&self, row: Rect, span: Span) -> Rect {
        self.cell(row, span).shrink2(Vec2::new(CELL_PADDING, 0.0))
    }
}

/// Paints `text` inside `cell`, clipped to the cell. `align` is either
/// left-center or center-center.
pub fn paint_text(ui: &Ui, cell: Rect, align: Align2, text: &str, font: &FontId, color: Color32) {
    let pos = if align == Align2::CENTER_CENTER {
        cell.center()
    } else {
        cell.left_center()
    };
    ui.painter()
        .with_clip_rect(cell)
        .text(pos, align, text, font.clone(), color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_fill_the_width() {
        let columns = Columns::new(10.0, 800.0);
        assert_eq!(columns.handle.0, 0.0);
        assert!((columns.priority.1 - 800.0).abs() < 0.01);
    }

    #[test]
    fn cell_is_offset_from_the_row() {
        let columns = Columns::new(0.0, 800.0);
        let row = Rect::from_min_size(egui_pos(100.0, 50.0), Vec2::new(800.0, ROW_HEIGHT));
        let cell = columns.cell(row, columns.check);

        assert_eq!(cell.left(), 100.0 + HANDLE_WIDTH);
        assert_eq!(cell.width(), CHECK_WIDTH);
        assert_eq!(cell.y_range(), row.y_range());
    }

    fn egui_pos(x: f32, y: f32) -> eframe::egui::Pos2 {
        eframe::egui::pos2(x, y)
    }
}
