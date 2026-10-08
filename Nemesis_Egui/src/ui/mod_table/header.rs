//! Header row of the mod table.

use eframe::egui::{Align2, Checkbox, FontId, Sense, Ui, Vec2};

use super::TableEvent;
use super::columns::{Columns, ROW_HEIGHT, paint_text};
use crate::mods::{ListEdit, ModList};

/// Draws the header and returns the column layout derived from its width.
///
/// The header checkbox is checked when every visible row is checked,
/// indeterminate when only some are, and toggles all visible rows on click.
pub fn show(
    ui: &mut Ui,
    mods: &ModList,
    visible: &[usize],
    running: bool,
    events: &mut Vec<TableEvent>,
) -> Columns {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let columns = Columns::new(rect.left(), rect.width());

    let checked = visible
        .iter()
        .filter(|&&i| mods.entries()[i].checked)
        .count();
    let all = !visible.is_empty() && checked == visible.len();
    let mut state = all;
    let checkbox = Checkbox::without_text(&mut state).indeterminate(checked > 0 && !all);

    if ui
        .put(columns.cell(rect, columns.check), checkbox)
        .clicked()
        && !running
    {
        events.push(TableEvent::Edit(ListEdit::SetChecked {
            indices: visible.to_vec(),
            checked: !all,
        }));
    }

    let font = FontId::proportional(14.0);
    let color = ui.visuals().strong_text_color();

    for (span, text, align) in [
        (columns.name, "Mod Name", Align2::LEFT_CENTER),
        (columns.author, "Author", Align2::LEFT_CENTER),
        (columns.site, "Site", Align2::LEFT_CENTER),
        (columns.priority, "Priority", Align2::CENTER_CENTER),
    ] {
        paint_text(ui, columns.text_cell(rect, span), align, text, &font, color);
    }

    columns
}
