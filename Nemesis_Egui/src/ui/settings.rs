//! The Settings tab.

use std::path::PathBuf;

use eframe::egui::{self, Button, ScrollArea, Ui};

use super::widgets::{PickKind, path_input};
use crate::config::{AppConfig, Platform};

/// Something the user changed or requested on the settings page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    /// A plain setting changed and should be saved.
    ConfigChanged,
    /// The engine path changed, so the mods must be rescanned.
    EngineChanged,
    /// Dark mode was toggled.
    ThemeChanged,
    /// Sort the mod list by name.
    SortModsByName,
    /// Drop saved entries of mods that are no longer installed.
    ForgetMissingMods,
    /// Open a path in the OS file manager.
    Open(PathBuf),
}

/// Draws the settings page, editing `config` in place.
///
/// `disabled` locks settings that must not change while patching.
pub fn show(ui: &mut Ui, config: &mut AppConfig, disabled: bool) -> Vec<SettingsAction> {
    let mut actions = Vec::new();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            engine_section(ui, config, disabled, &mut actions);
            section(ui, "Patch options");
            patch_options(ui, config, disabled, &mut actions);
            section(ui, "Mod list");
            mod_list_section(ui, config, disabled, &mut actions);
            section(ui, "Appearance");

            if ui.checkbox(&mut config.dark_mode, "Dark mode").changed() {
                actions.push(SettingsAction::ThemeChanged);
            }

            ui.add_space(12.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.weak("Settings file:");
                ui.monospace(AppConfig::file_path().display().to_string());
            });
        });

    actions
}

/// Draws a section heading with spacing.
fn section(ui: &mut Ui, title: &str) {
    ui.add_space(12.0);
    ui.heading(title);
    ui.add_space(4.0);
}

/// Engine path and the derived mods directory.
fn engine_section(
    ui: &mut Ui,
    config: &mut AppConfig,
    disabled: bool,
    actions: &mut Vec<SettingsAction>,
) {
    ui.heading("Engine");
    ui.add_space(4.0);

    let hint = format!("Empty = {}", config.default_engine_path().display());
    let kind = PickKind::Executable;
    if path_input(
        ui,
        "Nemesis_Engine executable",
        &hint,
        &mut config.engine_path,
        kind,
        disabled,
    ) {
        actions.push(SettingsAction::EngineChanged);
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let mods_dir = config.mods_dir();
        ui.label("Mods directory:");
        ui.monospace(mods_dir.display().to_string());

        if ui.small_button("Open").clicked() {
            actions.push(SettingsAction::Open(mods_dir));
        }
    });
}

/// Engine switches: platform, debug and synchronous mode.
fn patch_options(
    ui: &mut Ui,
    config: &mut AppConfig,
    disabled: bool,
    actions: &mut Vec<SettingsAction>,
) {
    let mut changed = false;

    ui.add_enabled_ui(!disabled, |ui| {
        egui::Grid::new("patch_options")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Output platform (-p)");
                egui::ComboBox::from_id_salt("platform")
                    .selected_text(config.platform.label())
                    .show_ui(ui, |ui| {
                        for platform in Platform::ALL {
                            changed |= ui
                                .selectable_value(&mut config.platform, platform, platform.label())
                                .changed();
                        }
                    });
                ui.end_row();

                ui.label("Debug mode (-db)");
                changed |= ui
                    .checkbox(&mut config.debug_mode, "Write extra debug output")
                    .changed();
                ui.end_row();

                ui.label("Synchronous (-s)");
                let text = "Disable multithreading (slow, for debugging)";
                changed |= ui.checkbox(&mut config.synchronous, text).changed();
                ui.end_row();
            });
    });

    if changed {
        actions.push(SettingsAction::ConfigChanged);
    }
}

/// Mod list behaviour and maintenance buttons.
fn mod_list_section(
    ui: &mut Ui,
    config: &mut AppConfig,
    disabled: bool,
    actions: &mut Vec<SettingsAction>,
) {
    let text = "Enable newly installed mods automatically";
    if ui.checkbox(&mut config.enable_new_mods, text).changed() {
        actions.push(SettingsAction::ConfigChanged);
    }

    ui.horizontal(|ui| {
        let sort = ui.add_enabled(!disabled, Button::new("Sort A → Z"));
        if sort.on_hover_text("Sort the list by mod name").clicked() {
            actions.push(SettingsAction::SortModsByName);
        }

        let forget = ui.add_enabled(!disabled, Button::new("Forget missing mods"));
        if forget
            .on_hover_text("Remove saved entries for mods that are no longer installed")
            .clicked()
        {
            actions.push(SettingsAction::ForgetMissingMods);
        }
    });
}
