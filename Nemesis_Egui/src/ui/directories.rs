//! The Skyrim data / output directory inputs at the top of the Patch tab.

use eframe::egui::Ui;

use super::widgets::{PickKind, path_input};
use crate::config::AppConfig;

/// Which inputs were committed this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirectoryChanges {
    /// The Skyrim data directory changed.
    pub data_dir: bool,
    /// The output directory changed.
    pub output_dir: bool,
}

/// Draws both inputs, editing `config` in place.
pub fn show(ui: &mut Ui, config: &mut AppConfig, disabled: bool) -> DirectoryChanges {
    let data_dir = path_input(
        ui,
        "Skyrim Data Directory",
        "e.g. C:/Program Files (x86)/Steam/steamapps/common/Skyrim Special Edition/Data",
        &mut config.data_dir,
        PickKind::Folder,
        disabled,
    );

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
