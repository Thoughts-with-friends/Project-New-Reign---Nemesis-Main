//! egui front-end for the Nemesis Unlimited Behavior Engine.
//!
//! The GUI lists the Nemesis mods found in this user's Skyrim `Data` directory,
//! lets the user enable them with checkboxes and reorder them by drag & drop, then
//! runs the Nemesis engine with the enabled mods. The list is merged from top to
//! bottom, so lower rows are applied later and win conflicts.
//!
//! The C++ engine is linked into this executable. Started as
//! `Nemesis_Egui.exe --nemesis-engine <engine home> <args...>`, the program runs
//! the engine instead of the GUI (see [`engine::embedded`]).

// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod diagnostics;
mod engine;
mod location;
mod mods;
mod os;
mod session;
mod ui;

use eframe::egui;

/// Application icon shared with the Qt launcher.
const ICON_PNG: &[u8] = include_bytes!("../../Nemesis_App/resources/icon.png");

fn main() -> eframe::Result {
    // Engine mode: the GUI started this executable to run a patch.
    if let Some(code) = engine::embedded::run_if_requested() {
        std::process::exit(code);
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Nemesis Unlimited Behavior Engine")
        .with_inner_size([900.0, 780.0])
        .with_min_inner_size([640.0, 560.0]);

    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Nemesis Egui",
        options,
        Box::new(|cc| Ok(Box::new(app::NemesisApp::new(cc)))),
    )
}
