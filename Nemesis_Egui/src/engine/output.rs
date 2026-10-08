//! Classification of the engine's standard output.
//!
//! * progress lines look like `ESC[999P{step} / {max}ESC[999E`;
//! * lines starting with `[ERROR]` mean the patch failed (the engine still exits
//!   with code `0` in that case, so the output is the only reliable signal);
//! * everything else is informational log output.

/// Opening marker of a progress line.
const PROGRESS_START: &str = "\x1b[999P";
/// Closing marker of a progress line.
const PROGRESS_END: &str = "\x1b[999E";
/// Prefix the engine uses for fatal errors.
const ERROR_PREFIX: &str = "[ERROR]";

/// One classified piece of engine output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputLine {
    /// Progress update: `step` out of `max`.
    Progress {
        /// Current step.
        step: u32,
        /// Total number of steps.
        max: u32,
    },
    /// A fatal error reported by the engine.
    Error(String),
    /// Ordinary log output.
    Info(String),
}

/// Classifies one line of engine output.
///
/// A single line may contain a progress marker surrounded by other text when
/// several engine threads write at once, so the result can hold several items.
/// Blank text is dropped.
pub fn classify_line(line: &str) -> Vec<OutputLine> {
    let mut out = Vec::new();
    let mut rest = line;

    while let Some(start) = rest.find(PROGRESS_START) {
        push_text(&mut out, &rest[..start]);

        let after_start = &rest[start + PROGRESS_START.len()..];
        let Some(end) = after_start.find(PROGRESS_END) else {
            rest = after_start;
            break;
        };

        if let Some(progress) = parse_progress(&after_start[..end]) {
            out.push(progress);
        }

        rest = &after_start[end + PROGRESS_END.len()..];
    }

    push_text(&mut out, rest);
    out
}

/// Parses the `{step} / {max}` body of a progress marker.
fn parse_progress(body: &str) -> Option<OutputLine> {
    let (step, max) = body.split_once('/')?;
    Some(OutputLine::Progress {
        step: step.trim().parse().ok()?,
        max: max.trim().parse().ok()?,
    })
}

/// Pushes non-blank text as either an error or an info line.
fn push_text(out: &mut Vec<OutputLine>, text: &str) {
    let text = text.trim_end_matches(['\r', '\n']);

    if text.trim().is_empty() {
        return;
    }

    if text.trim_start().starts_with(ERROR_PREFIX) {
        out.push(OutputLine::Error(text.to_owned()));
    } else {
        out.push(OutputLine::Info(text.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_progress_error_and_info() {
        assert_eq!(
            classify_line("\x1b[999P30 / 120\x1b[999E\n"),
            [OutputLine::Progress { step: 30, max: 120 }]
        );
        assert_eq!(
            classify_line("[ERROR] boom\r\n"),
            [OutputLine::Error("[ERROR] boom".into())]
        );
        assert_eq!(
            classify_line("Active Mod 1: tkuc\n"),
            [OutputLine::Info("Active Mod 1: tkuc".into())]
        );
        assert!(classify_line("   \n").is_empty());
    }

    #[test]
    fn classifies_progress_mixed_with_text() {
        assert_eq!(
            classify_line("Mod Class: abc\x1b[999P5 / 10\x1b[999Edone\n"),
            [
                OutputLine::Info("Mod Class: abc".into()),
                OutputLine::Progress { step: 5, max: 10 },
                OutputLine::Info("done".into()),
            ]
        );
    }
}
