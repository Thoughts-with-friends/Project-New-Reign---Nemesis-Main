//! egui front-end for the Nemesis Unlimited Behavior Engine.
//!
//! The GUI lists the mod patches installed in `<engine dir>/mods`, lets the user
//! enable them with checkboxes and reorder them by drag & drop, then runs
//! `Nemesis_Engine` with the enabled mods. The list is merged from top to bottom,
//! so lower rows are applied later and win conflicts.

// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod engine;
mod mods;
mod os;
mod session;
mod ui;

use eframe::egui;

/// Application icon shared with the Qt launcher.
const ICON_PNG: &[u8] = include_bytes!("../../Nemesis_App/resources/icon.png");

fn main() -> eframe::Result {
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
