//! Running the Nemesis engine as a child process.
//!
//! By default the C++ engine is linked into this executable (see [`embedded`])
//! and the GUI starts a copy of itself in engine mode; an external
//! `Nemesis_Engine.exe` can be used instead. Either way the engine receives the
//! command line the original Qt launcher used, and its output is parsed.
//!
//! * [`request`]: building the command line from the user's choices;
//! * [`output`]: classifying the engine's standard output;
//! * [`process`]: spawning, reading and killing the engine process;
//! * [`log_path`]: locating the `log.txt` written by the engine;
//! * [`embedded`]: the engine linked into this executable (C++ FFI).

pub mod embedded;
mod log_path;
mod output;
mod process;
mod request;

pub use log_path::log_file_path;
pub use output::OutputLine;
pub use process::{EngineEvent, EngineProcess};
pub use request::PatchRequest;
