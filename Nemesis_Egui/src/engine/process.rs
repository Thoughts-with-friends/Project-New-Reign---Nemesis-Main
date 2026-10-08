//! Spawning the engine and streaming its output to the UI thread.

use std::io::{self, BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use super::output::{OutputLine, classify_line};
use super::request::PatchRequest;

/// Callback invoked from worker threads whenever a new event is available.
type Notify = Arc<dyn Fn() + Send + Sync>;

/// Events sent from the engine worker threads to the UI thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    /// Classified standard output.
    Output(OutputLine),
    /// A line written to standard error.
    Stderr(String),
    /// The process ended. `None` when it was killed or the status is unknown.
    Exited(Option<i32>),
}

/// A running engine process.
///
/// Dropping the handle kills the process, so closing the GUI never leaves an
/// orphaned engine behind.
pub struct EngineProcess {
    /// The child, shared with the waiter thread.
    child: Arc<Mutex<Child>>,
    /// Receives output and the exit notification.
    events: Receiver<EngineEvent>,
    /// Set once [`EngineEvent::Exited`] has been received.
    finished: bool,
}

impl EngineProcess {
    /// Starts the engine described by `request`.
    ///
    /// `notify` is invoked from worker threads whenever a new event is available;
    /// pass a closure that requests a repaint of the UI.
    ///
    /// # Errors
    /// Returns an error when the executable does not exist or cannot be started.
    pub fn spawn(
        request: &PatchRequest,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        // No `is_file` pre-check: it can give false negatives inside MO2's
        // virtual file system. Spawning reports a missing executable anyway.
        let mut child = build_command(request).spawn().map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("{err} ({})", request.engine_path.display()),
            )
        })?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(child));
        let (tx, events) = mpsc::channel();
        let notify: Notify = Arc::new(notify);

        let readers = [
            stdout.map(|out| spawn_reader(out, tx.clone(), notify.clone(), stdout_events)),
            stderr.map(|err| spawn_reader(err, tx.clone(), notify.clone(), stderr_events)),
        ];

        let waiter_child = Arc::clone(&child);
        thread::spawn(move || {
            // Readers end at EOF, i.e. when the process exits or is killed.
            for reader in readers.into_iter().flatten() {
                let _ = reader.join();
            }

            let code = waiter_child
                .lock()
                .ok()
                .and_then(|mut child| child.wait().ok())
                .and_then(|status| status.code());

            let _ = tx.send(EngineEvent::Exited(code));
            notify();
        });

        Ok(Self {
            child,
            events,
            finished: false,
        })
    }

    /// Returns all events received since the last call without blocking.
    pub fn drain(&mut self) -> Vec<EngineEvent> {
        let events: Vec<_> = self.events.try_iter().collect();

        if events
            .iter()
            .any(|event| matches!(event, EngineEvent::Exited(_)))
        {
            self.finished = true;
        }

        events
    }

    /// Forcefully terminates the engine. Does nothing if it already exited.
    pub fn kill(&self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

impl Drop for EngineProcess {
    fn drop(&mut self) {
        if !self.finished {
            self.kill();
        }
    }
}

/// Builds the [`Command`]: piped output, no stdin, the engine directory as the
/// working directory, and no console window on Windows.
fn build_command(request: &PatchRequest) -> Command {
    let mut command = Command::new(&request.engine_path);
    command
        .args(request.to_args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(dir) = request.engine_path.parent() {
        command.current_dir(dir);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// Prevents a console window from popping up for the engine.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

/// Converts one standard output line into events.
fn stdout_events(line: String) -> Vec<EngineEvent> {
    classify_line(&line)
        .into_iter()
        .map(EngineEvent::Output)
        .collect()
}

/// Converts one standard error line into events.
fn stderr_events(line: String) -> Vec<EngineEvent> {
    let line = line.trim_end();
    if line.is_empty() {
        Vec::new()
    } else {
        vec![EngineEvent::Stderr(line.to_owned())]
    }
}

/// Reads `source` line by line on a new thread and forwards converted events.
///
/// Bytes are decoded lossily so that a stray non-UTF-8 byte cannot stop the reader.
fn spawn_reader<R>(
    source: R,
    tx: Sender<EngineEvent>,
    notify: Notify,
    convert: fn(String) -> Vec<EngineEvent>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(source);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();

            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let events = convert(String::from_utf8_lossy(&buffer).into_owned());

                    if events.is_empty() {
                        continue;
                    }

                    for event in events {
                        if tx.send(event).is_err() {
                            return;
                        }
                    }

                    notify();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::config::Platform;

    /// Runs the engine to completion and returns every event.
    fn run(engine_path: &Path, mods: Vec<String>) -> Vec<EngineEvent> {
        let request = PatchRequest {
            engine_path: engine_path.to_path_buf(),
            data_dir: String::new(),
            output_dir: String::new(),
            platform: Platform::Amd64,
            debug_mode: false,
            synchronous: false,
            mods,
        };
        let mut process = EngineProcess::spawn(&request, || {}).unwrap();
        let mut events = Vec::new();

        while !events.iter().any(|e| matches!(e, EngineEvent::Exited(_))) {
            events.extend(process.drain());
            thread::sleep(std::time::Duration::from_millis(10));
        }

        events
    }

    #[test]
    fn missing_executable_is_reported() {
        let request = PatchRequest {
            engine_path: PathBuf::from("Z:/missing/Nemesis_Engine.exe"),
            data_dir: String::new(),
            output_dir: String::new(),
            platform: Platform::Win32,
            debug_mode: false,
            synchronous: false,
            mods: Vec::new(),
        };

        // The exact kind depends on which part is missing (the working directory
        // or the file); the message must name the engine path either way.
        let err = EngineProcess::spawn(&request, || {}).err().unwrap();
        assert!(err.to_string().contains("Nemesis_Engine.exe"));
    }

    /// Runs a real executable that imitates the engine's output.
    /// Set `NEMESIS_FAKE_ENGINE` to its path to enable this test.
    #[test]
    fn runs_fake_engine_end_to_end() {
        let Some(engine_path) = std::env::var_os("NEMESIS_FAKE_ENGINE").map(PathBuf::from) else {
            return;
        };

        let ok = run(&engine_path, vec!["tkuc".into(), "bcbi".into()]);
        let args = "args: -p | amd64 | -pi | -m | tkuc | bcbi";
        assert!(ok.contains(&EngineEvent::Output(OutputLine::Info(args.into()))));
        assert!(ok.contains(&EngineEvent::Output(OutputLine::Progress {
            step: 120,
            max: 120
        })));
        assert!(ok.contains(&EngineEvent::Output(OutputLine::Info(
            "bad byte \u{fffd} here".into()
        ))));
        assert!(ok.contains(&EngineEvent::Stderr("stderr line".into())));
        assert_eq!(ok.last(), Some(&EngineEvent::Exited(Some(0))));

        let failed = run(&engine_path, vec!["fail".into()]);
        let error = "[ERROR] Invalid mod code 'fail'";
        assert!(failed.contains(&EngineEvent::Output(OutputLine::Error(error.into()))));
        assert_eq!(failed.last(), Some(&EngineEvent::Exited(Some(0))));
    }
}
