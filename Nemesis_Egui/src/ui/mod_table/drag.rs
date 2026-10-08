//! Drag & drop reordering of table rows.

use eframe::egui::{
    self, Color32, CornerRadius, CursorIcon, FontId, Id, LayerId, Order, Pos2, Rect, Stroke,
    StrokeKind, Ui, Vec2,
};

use super::TableEvent;
use super::columns::{Columns, ROW_HEIGHT};
use crate::mods::{ListEdit, ModList};
use crate::ui::style;

/// Distance from the table edge at which dragging starts auto-scrolling.
const AUTO_SCROLL_MARGIN: f32 = 36.0;
/// Scroll speed while auto-scrolling, in points per frame.
const AUTO_SCROLL_STEP: f32 = 8.0;

/// An in-progress drag of a table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drag {
    /// Index of the dragged row.
    pub from: usize,
    /// Insertion index (`0..=len`) under the pointer.
    pub insert_before: usize,
}

impl Drag {
    /// Starts dragging row `from`.
    pub const fn new(from: usize) -> Self {
        Self {
            from,
            insert_before: from,
        }
    }
}

/// Maps a pointer position to the gap between rows it is closest to (`0..=len`).
pub fn insertion_index(pointer_y: f32, first_row_top: f32, len: usize) -> usize {
    let slot = ((pointer_y - first_row_top) / ROW_HEIGHT).round();
    slot.clamp(0.0, len as f32) as usize
}

/// Updates the drag after the rows were drawn: tracks the pointer, draws the drop
/// indicator and preview, auto-scrolls, and emits the move on release.
pub fn update(
    ui: &mut Ui,
    drag: &mut Option<Drag>,
    first_row_top: f32,
    columns: Columns,
    mods: &ModList,
    events: &mut Vec<TableEvent>,
) {
    let Some(current) = drag.as_mut() else {
        return;
    };

    if let Some(pos) = ui.ctx().pointer_interact_pos() {
        current.insert_before = insertion_index(pos.y, first_row_top, mods.len());

        let accent = style::accent(ui);
        let line_y = first_row_top + current.insert_before as f32 * ROW_HEIGHT;
        let line = [
            egui::pos2(columns.left, line_y),
            egui::pos2(columns.left + columns.width, line_y),
        ];
        ui.painter().line_segment(line, Stroke::new(2.5, accent));

        auto_scroll(ui, pos);
        paint_preview(ui, pos, &mods.entries()[current.from].info.name, accent);

        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        ui.ctx().request_repaint();
    }

    let released = ui.input(|i| i.pointer.any_released() || !i.pointer.any_down());
    if released {
        let Drag {
            from,
            insert_before,
        } = *current;
        events.push(TableEvent::Edit(ListEdit::Move {
            from,
            insert_before,
        }));
        *drag = None;
    }
}

/// Scrolls the table when the pointer is near its top or bottom edge.
fn auto_scroll(ui: &mut Ui, pointer: Pos2) {
    let viewport = ui.clip_rect();

    if pointer.y < viewport.top() + AUTO_SCROLL_MARGIN {
        ui.scroll_with_delta(Vec2::new(0.0, AUTO_SCROLL_STEP));
    } else if pointer.y > viewport.bottom() - AUTO_SCROLL_MARGIN {
        ui.scroll_with_delta(Vec2::new(0.0, -AUTO_SCROLL_STEP));
    }
}

/// Draws the dragged mod's name in a small box following the cursor.
fn paint_preview(ui: &Ui, pointer: Pos2, name: &str, accent: Color32) {
    let visuals = ui.visuals();
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Tooltip, Id::new("mod_drag_preview")));
    let text_color = visuals.strong_text_color();
    let galley = painter.layout_no_wrap(name.to_owned(), FontId::proportional(14.0), text_color);
    let rect = Rect::from_min_size(
        pointer + Vec2::new(14.0, 6.0),
        galley.size() + Vec2::splat(12.0),
    );

    painter.rect_filled(rect, CornerRadius::same(4), visuals.extreme_bg_color);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, accent),
        StrokeKind::Inside,
    );
    painter.galley(rect.min + Vec2::splat(6.0), galley, text_color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_maps_to_nearest_gap() {
        let top = 100.0;
        assert_eq!(insertion_index(top - 50.0, top, 3), 0);
        assert_eq!(insertion_index(top + ROW_HEIGHT * 0.4, top, 3), 0);
        assert_eq!(insertion_index(top + ROW_HEIGHT * 0.6, top, 3), 1);
        assert_eq!(insertion_index(top + ROW_HEIGHT * 2.5, top, 3), 3);
        assert_eq!(insertion_index(top + ROW_HEIGHT * 10.0, top, 3), 3);
    }
}
