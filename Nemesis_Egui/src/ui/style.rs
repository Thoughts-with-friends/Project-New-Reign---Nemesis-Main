//! Fonts and colors.
//!
//! egui's bundled fonts do not contain CJK glyphs, so a system font is loaded as a
//! fallback to display Japanese/Chinese/Korean mod names and paths.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Theme, ThemePreference, Visuals,
};

/// Accent color used for enabled rows, links and selections (olive gold).
pub const ACCENT: Color32 = Color32::from_rgb(0xd4, 0xcb, 0x6e);
/// Darker accent for light mode, where the bright gold is hard to read.
pub const ACCENT_LIGHT_MODE: Color32 = Color32::from_rgb(0x7a, 0x6f, 0x12);
/// Fill of the main "Patch" button.
pub const PATCH_FILL: Color32 = Color32::from_rgb(0x5b, 0x76, 0x42);
/// Fill of the "Cancel" button shown while patching.
pub const CANCEL_FILL: Color32 = Color32::from_rgb(0x8a, 0x3b, 0x3b);
/// Color of error lines in the log.
pub const ERROR_TEXT: Color32 = Color32::from_rgb(0xff, 0x7a, 0x6e);
/// Color of success messages.
pub const SUCCESS_TEXT: Color32 = Color32::from_rgb(0x8f, 0xd1, 0x6b);

/// Candidate system fonts with CJK coverage, tried in order.
const CJK_FONT_CANDIDATES: &[&str] = &[
    "C:/Windows/Fonts/YuGothM.ttc",
    "C:/Windows/Fonts/meiryo.ttc",
    "C:/Windows/Fonts/msgothic.ttc",
    "C:/Windows/Fonts/malgun.ttf",
    "C:/Windows/Fonts/msyh.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
];

/// Registers the first available CJK system font as a fallback for all font families.
///
/// Returns `false` when no candidate font was found; the UI still works, but CJK
/// text renders as replacement boxes.
pub fn install_fonts(ctx: &egui::Context) -> bool {
    let Some(bytes) = CJK_FONT_CANDIDATES
        .iter()
        .find_map(|path| std::fs::read(path).ok())
    else {
        return false;
    };

    let mut fonts = FontDefinitions::default();
    let name = "cjk-fallback".to_owned();
    fonts
        .font_data
        .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.entry(family).or_default().push(name.clone());
    }

    ctx.set_fonts(fonts);
    true
}

/// Applies the accent palette to both themes and selects the active one.
pub fn apply_theme(ctx: &egui::Context, dark_mode: bool) {
    ctx.set_visuals_of(Theme::Dark, visuals(Visuals::dark(), ACCENT));
    ctx.set_visuals_of(Theme::Light, visuals(Visuals::light(), ACCENT_LIGHT_MODE));
    ctx.set_theme(if dark_mode {
        ThemePreference::Dark
    } else {
        ThemePreference::Light
    });
}

/// Returns the accent color for the current theme.
pub fn accent(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        ACCENT
    } else {
        ACCENT_LIGHT_MODE
    }
}

/// Tweaks the stock visuals with the accent color.
fn visuals(mut visuals: Visuals, accent: Color32) -> Visuals {
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = accent.gamma_multiply(0.45);
    visuals.selection.stroke.color = accent;
    visuals.widgets.hovered.bg_stroke.color = accent;
    visuals.widgets.active.bg_stroke.color = accent;
    visuals
}
