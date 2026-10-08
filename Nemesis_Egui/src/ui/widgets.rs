//! Small reusable widgets.

use std::path::{Path, PathBuf};

use eframe::egui::{Button, RichText, Stroke, TextEdit, Ui, Vec2};

use super::style;

/// Width of the "Select" button next to a path field.
const SELECT_BUTTON_WIDTH: f32 = 96.0;

/// What a path input's "Select" button picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickKind {
    /// A directory.
    Folder,
    /// The engine executable.
    Executable,
}

/// Draws a labelled path text field with a "Select" button.
///
/// Returns `true` when the value was committed (focus lost / Enter, or picked from
/// the dialog), so callers do not react to every keystroke.
pub fn path_input(
    ui: &mut Ui,
    label: &str,
    hint: &str,
    value: &mut String,
    kind: PickKind,
    disabled: bool,
) -> bool {
    let mut committed = false;

    ui.weak(label);
    ui.add_enabled_ui(!disabled, |ui| {
        ui.horizontal(|ui| {
            let width = ui.available_width() - SELECT_BUTTON_WIDTH - ui.spacing().item_spacing.x;
            let edit = ui.add(
                TextEdit::singleline(value)
                    .hint_text(hint)
                    .desired_width(width),
            );
            committed |= edit.lost_focus();

            if ui
                .add(outline_button(
                    ui,
                    "🗀 SELECT",
                    Vec2::new(SELECT_BUTTON_WIDTH, 28.0),
                ))
                .clicked()
                && let Some(path) = pick(kind, value)
            {
                *value = path.display().to_string();
                committed = true;
            }
        });
    });

    committed
}

/// An accent-colored outlined button, as used throughout D-Merge's UI.
pub fn outline_button(ui: &Ui, text: &str, min_size: Vec2) -> Button<'static> {
    let accent = style::accent(ui);
    Button::new(RichText::new(text).color(accent))
        .stroke(Stroke::new(1.0, accent))
        .min_size(min_size)
}

/// Shows a native file/folder dialog starting near `current`.
fn pick(kind: PickKind, current: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(dir) = existing_dir(current) {
        dialog = dialog.set_directory(dir);
    }

    match kind {
        PickKind::Folder => dialog.pick_folder(),
        PickKind::Executable if cfg!(windows) => {
            dialog.add_filter("Executable", &["exe"]).pick_file()
        }
        PickKind::Executable => dialog.pick_file(),
    }
}

/// Returns the closest existing directory of `value` (itself or an ancestor).
fn existing_dir(value: &str) -> Option<PathBuf> {
    let path = Path::new(value.trim());
    if path.as_os_str().is_empty() {
        return None;
    }

    // `read_dir` instead of `is_dir`: the latter is unreliable under MO2's VFS.
    path.ancestors()
        .find(|dir| !dir.as_os_str().is_empty() && std::fs::read_dir(dir).is_ok())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_dir_walks_up_to_an_existing_ancestor() {
        let temp = std::env::temp_dir();
        let missing = temp.join("nemesis_egui_missing_dir").join("child");

        assert_eq!(existing_dir(&missing.display().to_string()), Some(temp));
        assert_eq!(existing_dir("  "), None);
    }
}
