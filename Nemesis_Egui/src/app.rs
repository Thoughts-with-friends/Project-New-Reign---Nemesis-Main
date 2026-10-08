//! The eframe application: owns the state, lays out the panels and applies the
//! events returned by the views in [`crate::ui`].
//!
//! Layout (modelled after D-Merge):
//!
//! * top: Skyrim data (or MO2 mods glob) / output directory inputs;
//! * center: the mod table with checkboxes, search, and drag & drop reordering;
//! * bottom: engine log, progress bar, log buttons and the Patch button;
//! * footer: tab bar switching between *Patch* and *Settings*.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Frame, Margin, Panel, Ui};

use crate::config::{AppConfig, LoadedFrom};
use crate::diagnostics;
use crate::engine::{self, PatchRequest};
use crate::location::Locations;
use crate::mods::{ListEdit, ModList};
use crate::os;
use crate::session::{LogBuffer, LogKind, PatchSession};
use crate::ui::action_bar::{self, BarAction};
use crate::ui::mod_table::{self, TableEvent, TableState};
use crate::ui::nav_bar::{self, Tab};
use crate::ui::settings::{self, SettingsAction};
use crate::ui::{directories, log_view, style};

/// The application.
pub struct NemesisApp {
    /// Persisted settings.
    config: AppConfig,
    /// Whether `config` differs from the file on disk.
    dirty: bool,
    /// Paths resolved from `config` at the last rescan.
    locations: Locations,
    /// Installed mods in merge order.
    mods: ModList,
    /// Search text and drag state of the mod table.
    table: TableState,
    /// Active tab.
    tab: Tab,
    /// The current or last patch run.
    session: PatchSession,
    /// Log panel contents.
    log: LogBuffer,
}

impl NemesisApp {
    /// Creates the app: loads the config, sets up fonts/theme and scans mods.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (config, loaded_from) = AppConfig::load();
        let has_cjk = style::install_fonts(&cc.egui_ctx);
        style::apply_theme(&cc.egui_ctx, config.dark_mode);

        let mut app = Self {
            config,
            // A migrated legacy file is written to the new location right away.
            dirty: matches!(loaded_from, LoadedFrom::Legacy(_)),
            locations: Locations::default(),
            mods: ModList::default(),
            table: TableState::default(),
            tab: Tab::default(),
            session: PatchSession::default(),
            log: LogBuffer::default(),
        };

        for line in diagnostics::startup_lines(&loaded_from) {
            app.log.push(LogKind::Gui, line);
        }

        if !has_cjk {
            let message = "No CJK system font found; non-Latin text may not display correctly.";
            app.log.push(LogKind::Gui, message);
        }

        app.rescan_mods();
        app
    }

    // ---------------------------------------------------------------------
    // Config and mod list
    // ---------------------------------------------------------------------

    /// Resolves the paths again and rescans the mods, preserving the current
    /// order and check states. The result is described in the log.
    fn rescan_mods(&mut self) {
        self.sync_order_to_config();
        self.locations = Locations::resolve(&self.config);

        let config = &self.config;
        let (mods, report) = ModList::scan(
            &self.locations.mod_dirs,
            &config.mod_order,
            config.enable_new_mods,
        );
        self.mods = mods;
        self.table.cancel_drag();
        self.sync_order_to_config();

        for line in diagnostics::scan_lines(&self.locations, &report) {
            self.log.push(LogKind::Gui, line);
        }
    }

    /// Applies an edit to the mod list and records the new order.
    fn edit_mods(&mut self, edit: ListEdit) {
        self.mods.apply(edit);
        self.sync_order_to_config();
    }

    /// Copies the list order into the config and marks it for saving.
    fn sync_order_to_config(&mut self) {
        let order = self.mods.saved_order(&self.config.mod_order);

        if order != self.config.mod_order {
            self.config.mod_order = order;
            self.dirty = true;
        }
    }

    /// Writes the config to disk if it changed.
    fn save_config(&mut self) {
        if !self.dirty {
            return;
        }

        self.dirty = false;
        if let Err(err) = self.config.save() {
            let path = AppConfig::file_path();
            self.log.push(
                LogKind::Error,
                format!("Failed to save settings to {}: {err}", path.display()),
            );
        }
    }

    // ---------------------------------------------------------------------
    // Actions
    // ---------------------------------------------------------------------

    /// Saves the settings and starts the engine with the enabled mods.
    fn start_patch(&mut self, ctx: &egui::Context) {
        self.sync_order_to_config();
        self.save_config();

        let request = PatchRequest {
            engine_path: self.locations.engine_path.clone(),
            data_dir: self.locations.engine_data_arg(),
            output_dir: self.config.output_dir.clone(),
            platform: self.config.platform,
            debug_mode: self.config.debug_mode,
            synchronous: self.config.synchronous,
            mods: self.mods.active_codes(),
        };

        let ctx = ctx.clone();
        self.session
            .start(&request, &mut self.log, move || ctx.request_repaint());
    }

    /// Opens a path with the OS, reporting failures in the log.
    fn open_path(&mut self, path: &Path) {
        if let Err(message) = os::open_path(path) {
            self.log.push(LogKind::Error, message);
        }
    }

    /// Location of the engine's `log.txt` for the current settings.
    fn engine_log_path(&self) -> PathBuf {
        let locations = &self.locations;
        engine::log_file_path(
            &locations.engine_path,
            &locations.engine_data_arg(),
            &self.config.output_dir,
        )
    }

    // ---------------------------------------------------------------------
    // Pages
    // ---------------------------------------------------------------------

    /// Lays out the Patch tab and handles its events.
    fn patch_page(&mut self, ui: &mut Ui) {
        let running = self.session.is_running();

        Panel::bottom("actions")
            .frame(panel_frame(ui, 16, 10))
            .show(ui, |ui| {
                let can_patch = self.mods.scan_error().is_none();
                let log_path = self.engine_log_path();

                match action_bar::show(ui, self.session.status(), &log_path, can_patch) {
                    Some(BarAction::Patch) => self.start_patch(ui.ctx()),
                    Some(BarAction::Cancel) => self.session.cancel(),
                    Some(BarAction::Open(path)) => self.open_path(&path),
                    None => {}
                }
            });

        Panel::bottom("log")
            .resizable(true)
            .default_size(160.0)
            .frame(panel_frame(ui, 16, 8))
            .show(ui, |ui| log_view::show(ui, &mut self.log));

        Panel::top("directories")
            .frame(panel_frame(ui, 16, 12))
            .show(ui, |ui| {
                let changes = directories::show(ui, &mut self.config, &self.locations, running);
                self.dirty |= changes.data_dir || changes.output_dir;

                // The mod folders, `-d` and the default engine path all depend on it.
                if changes.data_dir {
                    self.rescan_mods();
                }
            });

        egui::CentralPanel::default()
            .frame(Frame::central_panel(ui.style()).inner_margin(Margin::symmetric(16, 10)))
            .show(ui, |ui| {
                let source = self.locations.data_source.clone();
                for event in mod_table::show(ui, &self.mods, &mut self.table, &source, running) {
                    match event {
                        TableEvent::Edit(edit) => self.edit_mods(edit),
                        TableEvent::Rescan => self.rescan_mods(),
                    }
                }
            });
    }

    /// Lays out the Settings tab and handles its events.
    fn settings_page(&mut self, ui: &mut Ui) {
        let running = self.session.is_running();

        egui::CentralPanel::default()
            .frame(Frame::central_panel(ui.style()).inner_margin(Margin::same(20)))
            .show(ui, |ui| {
                for action in settings::show(ui, &mut self.config, &self.locations, running) {
                    self.dirty = true;

                    match action {
                        SettingsAction::ConfigChanged => {}
                        SettingsAction::PathsChanged => self.rescan_mods(),
                        SettingsAction::ThemeChanged => {
                            style::apply_theme(ui.ctx(), self.config.dark_mode);
                        }
                        SettingsAction::SortModsByName => self.edit_mods(ListEdit::SortByName),
                        SettingsAction::ForgetMissingMods => {
                            self.config.mod_order = self.mods.saved_order(&[]);
                        }
                        SettingsAction::Open(path) => self.open_path(&path),
                    }
                }
            });
    }
}

impl eframe::App for NemesisApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.session.poll(&mut self.log);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        Panel::bottom("nav").show(ui, |ui| nav_bar::show(ui, &mut self.tab));

        match self.tab {
            Tab::Patch => self.patch_page(ui),
            Tab::Settings => self.settings_page(ui),
        }

        // Persist edits once the user is not typing or dragging anymore.
        let ctx = ui.ctx();
        let interacting = ctx.input(|i| i.pointer.any_down()) || ctx.egui_wants_keyboard_input();
        if self.dirty && !interacting {
            self.save_config();
        }
    }

    fn on_exit(&mut self) {
        self.sync_order_to_config();
        self.dirty = true;
        self.save_config();
    }
}

/// Frame of the side panels with the given horizontal/vertical margins.
fn panel_frame(ui: &Ui, x: i8, y: i8) -> Frame {
    Frame::side_top_panel(ui.style()).inner_margin(Margin::symmetric(x, y))
}
