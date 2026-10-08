//! The Settings tab.

use std::path::PathBuf;

use eframe::egui::{self, Button, ScrollArea, Ui};

use super::widgets::{PickKind, path_input};
use crate::config::{AppConfig, Platform};
use crate::engine::embedded;
use crate::location::Locations;

/// Something the user changed or requested on the settings page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    /// A plain setting changed and should be saved.
    ConfigChanged,
    /// A path or the platform changed: paths must be resolved again and the
    /// mods rescanned.
    PathsChanged,
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
/// `locations` shows what empty fields resolve to; `disabled` locks settings that
/// must not change while patching.
pub fn show(
    ui: &mut Ui,
    config: &mut AppConfig,
    locations: &Locations,
    disabled: bool,
) -> Vec<SettingsAction> {
    let mut actions = Vec::new();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            engine_section(ui, config, locations, disabled, &mut actions);
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
                let path = AppConfig::file_path();
                ui.monospace(path.display().to_string());
                if ui.small_button("Open folder").clicked()
                    && let Some(dir) = path.parent()
                {
                    actions.push(SettingsAction::Open(dir.to_path_buf()));
                }
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

/// Engine executable, the `-d` directory and the scanned mod folders.
fn engine_section(
    ui: &mut Ui,
    config: &mut AppConfig,
    locations: &Locations,
    disabled: bool,
    actions: &mut Vec<SettingsAction>,
) {
    ui.heading("Engine");
    ui.add_space(4.0);

    let home = locations
        .engine_path
        .parent()
        .unwrap_or(std::path::Path::new(""));
    // The hint is only visible while the field is empty.
    let hint = if embedded::AVAILABLE {
        format!("Empty = built-in engine (resources in {})", home.display())
    } else {
        format!("Empty = {}", locations.engine_path.display())
    };
    let kind = PickKind::Executable;
    if path_input(
        ui,
        "External Nemesis_Engine executable (optional, replaces the built-in engine)",
        &hint,
        &mut config.engine_path,
        kind,
        disabled,
    ) {
        actions.push(SettingsAction::PathsChanged);
    }

    ui.add_space(4.0);

    let detected = locations.engine_data_dir.as_ref().map_or_else(
        || "not detected".to_owned(),
        |dir| dir.display().to_string(),
    );
    let hint = format!("Empty = auto: {detected}");
    let label = "Skyrim Data for the engine (-d). Under MO2, the game's Data folder (virtualized)";
    if path_input(
        ui,
        label,
        &hint,
        &mut config.engine_data_dir,
        PickKind::Folder,
        disabled,
    ) {
        actions.push(SettingsAction::PathsChanged);
    }

    ui.add_space(4.0);
    ui.label(format!(
        "Scanning {} mod directories from: {}",
        locations.mod_dirs.len(),
        locations.data_source
    ));
}

/// Engine switches: platform, debug and synchronous mode.
fn patch_options(
    ui: &mut Ui,
    config: &mut AppConfig,
    disabled: bool,
    actions: &mut Vec<SettingsAction>,
) {
    let mut changed = false;
    let mut platform_changed = false;

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
                            platform_changed |= ui
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

    // The registry key used to detect Skyrim depends on the platform.
    if platform_changed {
        actions.push(SettingsAction::PathsChanged);
    } else if changed {
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
