//! The engine log panel.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Label, Layout, RichText, ScrollArea, TextStyle, Ui,
};

use super::style;
use crate::session::{LogBuffer, LogKind};

/// Draws the header (with Copy/Clear) and the scrolling, colored log.
pub fn show(ui: &mut Ui, log: &mut LogBuffer) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Log").strong());

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("Clear").clicked() {
                log.clear();
            }

            if ui.small_button("Copy").clicked() {
                ui.ctx().copy_text(log.to_text());
            }
        });
    });

    let row_height = ui.text_style_height(&TextStyle::Monospace);
    let visuals = ui.visuals().clone();

    egui::Frame::new()
        .fill(visuals.extreme_bg_color)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(6.0)
        .show(ui, |ui| {
            ScrollArea::both()
                .id_salt("log")
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show_rows(ui, row_height, log.lines().len(), |ui, range| {
                    for line in &log.lines()[range] {
                        let color = color_of(line.kind, &visuals);
                        ui.add(
                            Label::new(RichText::new(&line.text).monospace().color(color)).extend(),
                        );
                    }
                });
        });
}

/// Text color for a log line of `kind`.
fn color_of(kind: LogKind, visuals: &egui::Visuals) -> Color32 {
    match kind {
        LogKind::Info => visuals.text_color(),
        LogKind::Error => style::ERROR_TEXT,
        LogKind::Gui => visuals.weak_text_color(),
        LogKind::Success => style::SUCCESS_TEXT,
    }
}
