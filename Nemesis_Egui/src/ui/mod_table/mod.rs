//! The mod table: toolbar, header, rows and footer.
//!
//! * [`columns`]: column geometry;
//! * [`header`]: header row with the tri-state "check all" box;
//! * [`rows`]: one row per mod, with checkbox, link and context menu;
//! * [`drag`]: drag & drop reordering.

mod columns;
mod drag;
mod header;
mod rows;

use std::path::Path;

use eframe::egui::{
    self, Align, Button, Key, Label, Layout, RichText, ScrollArea, TextEdit, Ui,
    scroll_area::{DragScroll, ScrollSource},
};

use self::columns::{Columns, ROW_HEIGHT};
use self::drag::Drag;
use self::rows::RowMode;
use super::style;
use crate::mods::{ListEdit, ModList};

/// Height reserved for the footer below the rows.
const FOOTER_HEIGHT: f32 = 34.0;

/// What the user requested from the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableEvent {
    /// Change the mod list.
    Edit(ListEdit),
    /// Scan the mods directory again.
    Rescan,
}

/// UI-only state of the table that survives between frames.
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// Text of the search box.
    pub search: String,
    /// Row currently being dragged.
    drag: Option<Drag>,
}

impl TableState {
    /// Aborts a drag in progress, e.g. after the list was rescanned.
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }
}

/// Draws the table for `mods` and returns the requested changes.
///
/// Reordering is only possible while not `running` and with an empty search,
/// because row positions are meaningless in a filtered view.
pub fn show(
    ui: &mut Ui,
    mods: &ModList,
    state: &mut TableState,
    mods_dir: &Path,
    running: bool,
) -> Vec<TableEvent> {
    let mut events = Vec::new();
    let visible = mods.visible_indices(&state.search);
    let searching = !state.search.trim().is_empty();
    let mode = RowMode {
        can_reorder: !running && !searching,
        running,
    };

    egui::Frame::group(ui.style())
        .inner_margin(0.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            toolbar(ui, state, mods_dir, running, &mut events);
            ui.separator();

            let columns = header::show(ui, mods, &visible, running, &mut events);
            ui.separator();

            let height = (ui.available_height() - FOOTER_HEIGHT).max(ROW_HEIGHT * 2.0);
            ScrollArea::vertical()
                .id_salt("mod_table")
                .auto_shrink([false, false])
                // Row dragging reorders mods, so the list must not scroll by dragging.
                .scroll_source(ScrollSource {
                    drag: DragScroll::Never,
                    ..Default::default()
                })
                .max_height(height)
                .min_scrolled_height(height)
                .show(ui, |ui| {
                    body(
                        ui,
                        mods,
                        &visible,
                        columns,
                        mode,
                        &mut state.drag,
                        &mut events,
                    )
                });

            ui.separator();
            footer(ui, mods, visible.len(), searching);
        });

    events
}

/// Title, mods path, search box and rescan button.
fn toolbar(
    ui: &mut Ui,
    state: &mut TableState,
    mods_dir: &Path,
    running: bool,
    events: &mut Vec<TableEvent>,
) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Mods").strong().size(15.0));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let rescan = ui.add_enabled(!running, Button::new("⟳"));
                    if rescan.on_hover_text("Rescan mods").clicked() {
                        events.push(TableEvent::Rescan);
                    }

                    let search = TextEdit::singleline(&mut state.search)
                        .hint_text("🔍 Search")
                        .desired_width(180.0);
                    if ui.add(search).has_focus() && ui.input(|i| i.key_pressed(Key::Escape)) {
                        state.search.clear();
                    }

                    // The mods path takes whatever space is left and is truncated.
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        let path = mods_dir.display().to_string();
                        ui.add(Label::new(RichText::new(&path).weak().small()).truncate())
                            .on_hover_text(format!(
                                "Mods are read from <engine directory>/mods\n{path}"
                            ));
                    });
                });
            });
        });
}

/// Rows, or a message when there is nothing to show.
fn body(
    ui: &mut Ui,
    mods: &ModList,
    visible: &[usize],
    columns: Columns,
    mode: RowMode,
    drag: &mut Option<Drag>,
    events: &mut Vec<TableEvent>,
) {
    if let Some(err) = mods.scan_error() {
        ui.add_space(12.0);
        ui.colored_label(style::ERROR_TEXT, err);
        ui.label("Check the Skyrim data directory, or set the engine path in Settings.");
        return;
    }

    if visible.is_empty() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.weak(if mods.is_empty() {
                "No mods found."
            } else {
                "No mods match the search."
            });
        });
        return;
    }

    if let Some(first_row_top) = rows::show(ui, mods, visible, columns, mode, drag, events) {
        drag::update(ui, drag, first_row_top, columns, mods, events);
    }
}

/// Enabled count and a usage hint.
fn footer(ui: &mut Ui, mods: &ModList, shown: usize, searching: bool) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(format!(
            "{} / {} mods enabled",
            mods.enabled_count(),
            mods.len()
        ));

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(10.0);
            if searching {
                ui.weak(format!("{shown} shown · clear the search to reorder"));
            } else {
                ui.weak("Drag rows to reorder · lower rows are merged later and win conflicts");
            }
        });
    });
}
