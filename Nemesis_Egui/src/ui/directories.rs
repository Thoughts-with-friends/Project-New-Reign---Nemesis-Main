//! The data / output directory inputs at the top of the Patch tab.

use eframe::egui::{RichText, Ui};

use super::widgets::{PickKind, path_input};
use crate::config::AppConfig;
use crate::location::Locations;

/// Which inputs were committed this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirectoryChanges {
    /// The data directory / mods pattern changed.
    pub data_dir: bool,
    /// The output directory changed.
    pub output_dir: bool,
}

/// Draws both inputs, editing `config` in place. `locations` provides the
/// automatic default shown when the data field is empty.
pub fn show(
    ui: &mut Ui,
    config: &mut AppConfig,
    locations: &Locations,
    disabled: bool,
) -> DirectoryChanges {
    let hint = if locations.default_data_source.is_empty() {
        "e.g. D:/MO2/mods/*  or  C:/Steam/steamapps/common/Skyrim Special Edition/Data".to_owned()
    } else {
        format!("Empty = auto: {}", locations.default_data_source)
    };

    let data_dir = path_input(
        ui,
        "Skyrim Data Directory (Can Glob: e.g. D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*)",
        &hint,
        &mut config.data_dir,
        PickKind::Folder,
        disabled,
    );
    source_summary(ui, locations);

    ui.add_space(6.0);

    let output_dir = path_input(
        ui,
        "Output Directory",
        "Empty = write into the data directory (e.g. an MO2 mod folder such as …/mods/Nemesis Output)",
        &mut config.output_dir,
        PickKind::Folder,
        disabled,
    );

    DirectoryChanges {
        data_dir,
        output_dir,
    }
}

/// One weak line explaining how the data field was interpreted.
fn source_summary(ui: &mut Ui, locations: &Locations) {
    let engine_data = locations.engine_data_dir.as_ref().map_or_else(
        || "not found — set it in Settings".to_owned(),
        |dir| dir.display().to_string(),
    );

    let text = if locations.is_mod_folders {
        format!(
            "{} MO2 mod folders · engine Data (-d): {engine_data}",
            locations.data_roots.len()
        )
    } else {
        format!("Skyrim Data directory · engine Data (-d): {engine_data}")
    };

    ui.label(RichText::new(text).weak().small());
}
