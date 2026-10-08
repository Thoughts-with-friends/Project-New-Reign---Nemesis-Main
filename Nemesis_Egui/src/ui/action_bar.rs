//! The bottom bar of the Patch tab: progress, log buttons and Patch/Cancel.

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui::{Button, Color32, ProgressBar, RichText, Ui, Vec2};

use super::style;
use super::widgets::outline_button;
use crate::session::{RunOutcome, SessionStatus};

/// Height of the buttons in the bar.
const BUTTON_HEIGHT: f32 = 44.0;

/// What the user requested from the action bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarAction {
    /// Start patching.
    Patch,
    /// Stop the running engine.
    Cancel,
    /// Open a file or directory (the log or its folder).
    Open(PathBuf),
}

/// Draws the bar. `can_patch` disables the Patch button (e.g. no mods directory).
pub fn show(
    ui: &mut Ui,
    status: SessionStatus,
    log_path: &Path,
    can_patch: bool,
) -> Option<BarAction> {
    progress(ui, status);
    ui.add_space(6.0);

    let mut action = None;
    let log_dir = log_path.parent().unwrap_or(Path::new("")).to_path_buf();
    let button_size = Vec2::new(0.0, BUTTON_HEIGHT);

    ui.horizontal(|ui| {
        let open_dir = ui.add(outline_button(ui, "🗁 Log (Directory)", button_size));
        if open_dir
            .on_hover_text(log_dir.display().to_string())
            .clicked()
        {
            action = Some(BarAction::Open(log_dir.clone()));
        }

        let open_log = ui.add(outline_button(ui, "🗋 Open Log", button_size));
        if open_log
            .on_hover_text(log_path.display().to_string())
            .clicked()
        {
            action = Some(BarAction::Open(log_path.to_path_buf()));
        }

        let size = Vec2::new(ui.available_width(), BUTTON_HEIGHT);

        if matches!(status, SessionStatus::Running { .. }) {
            if ui
                .add(main_button("■  CANCEL", style::CANCEL_FILL, size))
                .clicked()
            {
                action = Some(BarAction::Cancel);
            }
        } else if ui
            .add_enabled(can_patch, main_button("PATCH  ⚙", style::PATCH_FILL, size))
            .on_hover_text("Run Nemesis_Engine with the enabled mods (top to bottom)")
            .clicked()
        {
            action = Some(BarAction::Patch);
        }
    });

    action
}

/// Draws the progress bar for the current status.
fn progress(ui: &mut Ui, status: SessionStatus) {
    match status {
        SessionStatus::Ready => {
            ui.add(ProgressBar::new(0.0).text("Ready"));
        }
        SessionStatus::Running { progress, elapsed } => {
            let (step, max) = progress.unwrap_or((0, 1));
            let fraction = if max == 0 {
                0.0
            } else {
                step as f32 / max as f32
            };
            let text = format!("Patching… {step} / {max} · {:.1}s", elapsed.as_secs_f32());
            ui.add(
                ProgressBar::new(fraction)
                    .text(text)
                    .fill(style::PATCH_FILL),
            );

            // Keep the elapsed time ticking even without new engine output.
            ui.ctx().request_repaint_after(Duration::from_millis(250));
        }
        SessionStatus::Finished { outcome, elapsed } => {
            let (text, color) = match outcome {
                RunOutcome::Success => (
                    format!("✔ Done in {:.2}s", elapsed.as_secs_f64()),
                    style::SUCCESS_TEXT,
                ),
                RunOutcome::Failed => ("✖ Failed — see the log".to_owned(), style::ERROR_TEXT),
                RunOutcome::Cancelled => ("Cancelled".to_owned(), ui.visuals().weak_text_color()),
            };
            let fraction = if outcome == RunOutcome::Success {
                1.0
            } else {
                0.0
            };
            ui.add(ProgressBar::new(fraction).text(RichText::new(text).color(color)));
        }
    }
}

/// The large filled Patch/Cancel button.
fn main_button(text: &str, fill: Color32, size: Vec2) -> Button<'static> {
    Button::new(RichText::new(text).size(16.0).color(Color32::WHITE))
        .fill(fill)
        .min_size(size)
}
