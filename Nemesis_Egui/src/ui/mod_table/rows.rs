//! Mod rows: background, checkbox, text, link and per-row interaction.

use eframe::egui::{
    self, Align, Align2, Checkbox, Color32, CornerRadius, CursorIcon, FontId, Label, Layout, Rect,
    Response, RichText, Sense, Ui, UiBuilder, Vec2,
};

use super::TableEvent;
use super::columns::{Columns, ROW_HEIGHT, paint_text};
use super::drag::Drag;
use crate::mods::{ListEdit, ModEntry, ModList};
use crate::os;
use crate::ui::style;

/// What the rows allow the user to do.
#[derive(Debug, Clone, Copy)]
pub struct RowMode {
    /// Rows can be dragged and moved via the context menu.
    pub can_reorder: bool,
    /// The engine is running; checkboxes are read-only.
    pub running: bool,
}

/// Draws the `visible` rows. Returns the top of the first row, which the drag
/// logic uses to map the pointer to a row slot.
pub fn show(
    ui: &mut Ui,
    mods: &ModList,
    visible: &[usize],
    columns: Columns,
    mode: RowMode,
    drag: &mut Option<Drag>,
    events: &mut Vec<TableEvent>,
) -> Option<f32> {
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut first_row_top = None;

    for (row, &index) in visible.iter().enumerate() {
        let size = Vec2::new(columns.width, ROW_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        first_row_top.get_or_insert(rect.top());

        let entry = &mods.entries()[index];
        paint_background(ui, rect, &response, row, index, *drag);
        draw_cells(ui, rect, columns, entry, index, mode, events);

        let response = response.on_hover_text(format!(
            "Mod code: {}\nFolder: {}",
            entry.info.code,
            entry.info.dir.display()
        ));

        if response.clicked() && !mode.running {
            events.push(TableEvent::Edit(ListEdit::Toggle(index)));
        }

        if mode.can_reorder {
            if response.drag_started() {
                *drag = Some(Drag::new(index));
            }

            context_menu(&response, index, mods.len(), events);
        }
    }

    first_row_top
}

/// Zebra stripes, hover and drag highlight.
fn paint_background(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    row: usize,
    index: usize,
    drag: Option<Drag>,
) {
    let visuals = ui.visuals();
    let fill = if drag.is_some_and(|d| d.from == index) {
        style::accent(ui).gamma_multiply(0.25)
    } else if response.hovered() && drag.is_none() {
        visuals.widgets.hovered.weak_bg_fill
    } else if row % 2 == 1 {
        visuals.faint_bg_color
    } else {
        Color32::TRANSPARENT
    };

    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
}

/// Handle, checkbox, name, author, site link and priority of one row.
fn draw_cells(
    ui: &mut Ui,
    rect: Rect,
    columns: Columns,
    entry: &ModEntry,
    index: usize,
    mode: RowMode,
    events: &mut Vec<TableEvent>,
) {
    let font = FontId::proportional(14.0);
    let weak = ui.visuals().weak_text_color();
    let handle_color = if mode.can_reorder {
        weak
    } else {
        weak.gamma_multiply(0.3)
    };
    paint_text(
        ui,
        columns.cell(rect, columns.handle),
        Align2::CENTER_CENTER,
        "☰",
        &font,
        handle_color,
    );

    let mut checked = entry.checked;
    let checkbox = ui.put(
        columns.cell(rect, columns.check),
        Checkbox::without_text(&mut checked),
    );
    if checkbox.clicked() && !mode.running {
        events.push(TableEvent::Edit(ListEdit::Toggle(index)));
    }

    let color = if entry.checked {
        style::accent(ui)
    } else {
        ui.visuals().text_color()
    };
    let info = &entry.info;
    paint_text(
        ui,
        columns.text_cell(rect, columns.name),
        Align2::LEFT_CENTER,
        &info.name,
        &font,
        color,
    );
    paint_text(
        ui,
        columns.text_cell(rect, columns.author),
        Align2::LEFT_CENTER,
        &info.author,
        &font,
        color,
    );
    site_link(ui, columns.text_cell(rect, columns.site), &info.site);

    let priority = (index + 1).to_string();
    let priority_cell = columns.cell(rect, columns.priority);
    paint_text(
        ui,
        priority_cell,
        Align2::CENTER_CENTER,
        &priority,
        &font,
        color,
    );
}

/// A truncated, clickable link that opens the mod's site in the browser.
fn site_link(ui: &mut Ui, cell: Rect, site: &str) {
    if site.is_empty() {
        return;
    }

    let color = ui.visuals().hyperlink_color;
    let builder = UiBuilder::new()
        .max_rect(cell)
        .layout(Layout::left_to_right(Align::Center));
    let link = ui
        .scope_builder(builder, |ui| {
            ui.add(
                Label::new(RichText::new(site).color(color))
                    .truncate()
                    .sense(Sense::click()),
            )
        })
        .inner
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(site);

    if link.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(os::web_url(site)));
    }
}

/// Right-click menu with keyboard-free alternatives to dragging.
fn context_menu(response: &Response, index: usize, len: usize, events: &mut Vec<TableEvent>) {
    response.context_menu(|ui| {
        let moves = [
            ("⏶ Move to top", 0),
            ("▲ Move up", index.saturating_sub(1)),
            ("▼ Move down", (index + 2).min(len)),
            ("⏷ Move to bottom", len),
        ];

        for (label, insert_before) in moves {
            if ui.button(label).clicked() {
                events.push(TableEvent::Edit(ListEdit::Move {
                    from: index,
                    insert_before,
                }));
                ui.close();
            }
        }
    });
}
