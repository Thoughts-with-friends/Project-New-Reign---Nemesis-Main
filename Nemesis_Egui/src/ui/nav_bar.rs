//! The footer tab bar.

use eframe::egui::{Button, RichText, Ui, Vec2};

use super::style;

/// Width of one tab button.
const TAB_WIDTH: f32 = 110.0;
/// Height of one tab button.
const TAB_HEIGHT: f32 = 46.0;

/// Pages reachable from the tab bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Mod list and patching.
    #[default]
    Patch,
    /// Engine and application settings.
    Settings,
}

impl Tab {
    /// Every tab with its icon and label, in display order.
    const ALL: [(Self, &'static str, &'static str); 2] = [
        (Self::Patch, "🔨", "Patch"),
        (Self::Settings, "⚙", "Settings"),
    ];
}

/// Draws the centered tab bar and switches `current` on click.
pub fn show(ui: &mut Ui, current: &mut Tab) {
    let accent = style::accent(ui);
    let total = TAB_WIDTH * Tab::ALL.len() as f32;

    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));

        for (tab, icon, label) in Tab::ALL {
            let color = if *current == tab {
                accent
            } else {
                ui.visuals().text_color()
            };
            let text = RichText::new(format!("{icon}\n{label}")).color(color);
            let button = Button::new(text)
                .frame(false)
                .min_size(Vec2::new(TAB_WIDTH, TAB_HEIGHT));

            if ui.add(button).clicked() {
                *current = tab;
            }
        }
    });
}
