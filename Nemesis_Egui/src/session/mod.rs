//! Lifecycle of a patch run: starting the engine, collecting its output into the
//! log, tracking progress and deciding the final outcome.
//!
//! This module holds no drawing code; the UI reads [`PatchSession::status`] and
//! the [`LogBuffer`].

mod log;

use std::time::{Duration, Instant};

pub use log::{LogBuffer, LogKind};

use crate::engine::{EngineEvent, EngineProcess, OutputLine, PatchRequest};

/// Outcome of a finished patch run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// Completed without errors.
    Success,
    /// The engine reported an error, crashed or could not be started.
    Failed,
    /// The user pressed Cancel.
    Cancelled,
}

/// What the progress bar should show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// Nothing has run yet.
    Ready,
    /// The engine is running.
    Running {
        /// Latest `(step, max)` progress, if any was reported.
        progress: Option<(u32, u32)>,
        /// Time since the run started.
        elapsed: Duration,
    },
    /// The last run finished.
    Finished {
        /// How it ended.
        outcome: RunOutcome,
        /// How long it took.
        elapsed: Duration,
    },
}

/// State of the running engine.
struct Run {
    /// The engine process.
    process: EngineProcess,
    /// When the run started.
    started: Instant,
    /// Latest progress `(step, max)`.
    progress: Option<(u32, u32)>,
    /// Whether an `[ERROR]` line was seen.
    saw_error: bool,
    /// Whether the user cancelled.
    cancelled: bool,
}

/// Runs the engine and keeps track of the current or last run.
#[derive(Default)]
pub struct PatchSession {
    /// The running engine, if any.
    run: Option<Run>,
    /// Outcome and duration of the last finished run.
    last: Option<(RunOutcome, Duration)>,
}

impl PatchSession {
    /// Returns `true` while the engine is running.
    pub const fn is_running(&self) -> bool {
        self.run.is_some()
    }

    /// Returns the state to display.
    pub fn status(&self) -> SessionStatus {
        match (&self.run, self.last) {
            (Some(run), _) => SessionStatus::Running {
                progress: run.progress,
                elapsed: run.started.elapsed(),
            },
            (None, Some((outcome, elapsed))) => SessionStatus::Finished { outcome, elapsed },
            (None, None) => SessionStatus::Ready,
        }
    }

    /// Clears `log`, validates `request` and starts the engine.
    ///
    /// Failures are written to `log` and recorded as [`RunOutcome::Failed`].
    /// `notify` is called from worker threads when new output arrives.
    pub fn start(
        &mut self,
        request: &PatchRequest,
        log: &mut LogBuffer,
        notify: impl Fn() + Send + Sync + 'static,
    ) {
        if self.is_running() {
            return;
        }

        log.clear();
        self.last = None;

        if let Err(message) = request.prepare() {
            self.fail(log, message);
            return;
        }

        log.push(
            LogKind::Gui,
            format!("> {}", request.display_command_line()),
        );

        match EngineProcess::spawn(request, notify) {
            Ok(process) => {
                self.run = Some(Run {
                    process,
                    started: Instant::now(),
                    progress: None,
                    saw_error: false,
                    cancelled: false,
                });
            }
            Err(err) => self.fail(log, format!("Failed to start the engine: {err}")),
        }
    }

    /// Kills the running engine; the outcome becomes [`RunOutcome::Cancelled`].
    pub fn cancel(&mut self) {
        if let Some(run) = &mut self.run {
            run.cancelled = true;
            run.process.kill();
        }
    }

    /// Moves pending engine output into `log` and finishes the run on exit.
    pub fn poll(&mut self, log: &mut LogBuffer) {
        let Some(run) = &mut self.run else {
            return;
        };

        let mut exit = None;

        for event in run.process.drain() {
            match event {
                EngineEvent::Output(OutputLine::Progress { step, max }) => {
                    // Keep the bar where it stopped once an error was reported.
                    if !run.saw_error {
                        run.progress = Some((step, max));
                    }
                }
                EngineEvent::Output(OutputLine::Error(text)) => {
                    run.saw_error = true;
                    log.push(LogKind::Error, text);
                }
                EngineEvent::Output(OutputLine::Info(text)) => log.push(LogKind::Info, text),
                EngineEvent::Stderr(text) => log.push(LogKind::Error, text),
                EngineEvent::Exited(code) => exit = Some(code),
            }
        }

        if let Some(code) = exit {
            self.finish(code, log);
        }
    }

    /// Decides the outcome of the run that just exited with `code`.
    fn finish(&mut self, code: Option<i32>, log: &mut LogBuffer) {
        let Some(run) = self.run.take() else {
            return;
        };
        let elapsed = run.started.elapsed();

        let outcome = if run.cancelled {
            log.push(LogKind::Gui, "Patch cancelled.");
            RunOutcome::Cancelled
        } else if run.saw_error {
            RunOutcome::Failed
        } else if code != Some(0) {
            log.push(
                LogKind::Error,
                format!("[ERROR] Engine exited unexpectedly (exit code: {code:?})"),
            );
            RunOutcome::Failed
        } else {
            log.push(
                LogKind::Success,
                format!("Patch completed in {:.2}s", elapsed.as_secs_f64()),
            );
            RunOutcome::Success
        };

        self.last = Some((outcome, elapsed));
    }

    /// Records a failure that happened before the engine could run.
    fn fail(&mut self, log: &mut LogBuffer, message: String) {
        log.push(LogKind::Error, message);
        self.last = Some((RunOutcome::Failed, Duration::ZERO));
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::config::Platform;

    fn request(engine_path: &str) -> PatchRequest {
        PatchRequest {
            engine_path: PathBuf::from(engine_path),
            embedded: false,
            data_dir: String::new(),
            output_dir: String::new(),
            platform: Platform::Amd64,
            debug_mode: false,
            synchronous: false,
            mods: Vec::new(),
        }
    }

    #[test]
    fn starts_ready() {
        assert_eq!(PatchSession::default().status(), SessionStatus::Ready);
    }

    #[test]
    fn missing_engine_fails_without_running() {
        let mut session = PatchSession::default();
        let mut log = LogBuffer::default();
        log.push(LogKind::Info, "old line");

        session.start(&request("Z:/missing/Nemesis_Engine.exe"), &mut log, || {});

        assert!(!session.is_running());
        assert!(matches!(
            session.status(),
            SessionStatus::Finished {
                outcome: RunOutcome::Failed,
                ..
            }
        ));
        assert!(log.lines().iter().all(|line| line.text != "old line"));
        assert!(log.lines().iter().any(|line| line.kind == LogKind::Error));
    }

    /// Set `NEMESIS_FAKE_ENGINE` to a fake engine executable to enable this test.
    #[test]
    fn fake_engine_run_succeeds() {
        let Some(engine) = std::env::var("NEMESIS_FAKE_ENGINE").ok() else {
            return;
        };

        let mut session = PatchSession::default();
        let mut log = LogBuffer::default();
        session.start(&request(&engine), &mut log, || {});

        while session.is_running() {
            session.poll(&mut log);
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(matches!(
            session.status(),
            SessionStatus::Finished {
                outcome: RunOutcome::Success,
                ..
            }
        ));
        assert_eq!(
            log.lines().last().map(|line| line.kind),
            Some(LogKind::Success)
        );
    }
}
