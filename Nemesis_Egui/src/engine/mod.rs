//! Running `Nemesis_Engine` as a child process.
//!
//! The GUI does not link the C++ engine. Instead it launches the engine executable
//! with the same command line the original Qt launcher uses and parses its output.
//!
//! * [`request`]: building the command line from the user's choices;
//! * [`output`]: classifying the engine's standard output;
//! * [`process`]: spawning, reading and killing the engine process;
//! * [`log_path`]: locating the `log.txt` written by the engine.

mod log_path;
mod output;
mod process;
mod request;

pub use log_path::log_file_path;
pub use output::OutputLine;
pub use process::{EngineEvent, EngineProcess};
pub use request::PatchRequest;
