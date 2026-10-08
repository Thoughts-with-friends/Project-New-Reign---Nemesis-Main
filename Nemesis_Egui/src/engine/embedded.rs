//! The Nemesis C++ engine linked into this executable.
//!
//! The engine is compiled from `Nemesis_Core_Engine` by `build.rs` and exposed as
//! `extern "C" nemesis_engine_main` (see `Nemesis_Core_Engine/include/EngineMain.h`).
//! It keeps global state and writes progress to stdout, so it is not called on the
//! GUI thread. Instead the GUI starts *this same executable* in engine mode:
//!
//! ```text
//! Nemesis_Egui.exe --nemesis-engine <engine home> <engine arguments...>
//! ```
//!
//! [`run_if_requested`] handles that mode before any window is created. Running
//! the engine in a child process keeps the program a single binary while giving
//! every patch a fresh engine state, a real cancel (killing the child) and crash
//! isolation. Under MO2 the child inherits the virtual file system.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// First argument that switches the executable into engine mode.
pub const ENGINE_MODE_ARG: &str = "--nemesis-engine";

/// File name the engine believes it runs as. Its directory (the *engine home*)
/// is where it looks for `mods`, `behavior_templates`, `alternate_animations`,
/// `nemesis.cache` and writes `log.txt`.
pub const ENGINE_EXE_NAME: &str = if cfg!(windows) {
    "Nemesis_Engine.exe"
} else {
    "Nemesis_Engine"
};

/// Whether the engine is linked into this build (`embedded-engine` feature).
pub const AVAILABLE: bool = cfg!(feature = "embedded-engine");

/// Builds the arguments that start this executable in engine mode.
pub fn self_args(engine_home: &Path, engine_args: Vec<OsString>) -> Vec<OsString> {
    let mut args = vec![
        OsString::from(ENGINE_MODE_ARG),
        engine_home.as_os_str().to_owned(),
    ];
    args.extend(engine_args);
    args
}

/// Splits engine-mode arguments (without the program name) into the engine home
/// and the arguments for the engine. Returns `None` when not in engine mode.
pub fn parse_self_args(args: &[OsString]) -> Option<(PathBuf, &[OsString])> {
    let (mode, rest) = args.split_first()?;
    if mode != ENGINE_MODE_ARG {
        return None;
    }

    let (home, engine_args) = rest.split_first()?;
    Some((PathBuf::from(home), engine_args))
}

/// The engine's `argv`: `<engine home>/Nemesis_Engine.exe` followed by `engine_args`.
pub fn engine_argv(engine_home: &Path, engine_args: &[OsString]) -> Vec<OsString> {
    std::iter::once(engine_home.join(ENGINE_EXE_NAME).into_os_string())
        .chain(engine_args.iter().cloned())
        .collect()
}

/// Runs the engine when this process was started in engine mode and returns its
/// exit code; returns `None` for a normal GUI start.
pub fn run_if_requested() -> Option<i32> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (home, engine_args) = parse_self_args(&args)?;
    let argv = engine_argv(&home, engine_args);

    if let Err(err) = std::env::set_current_dir(&home) {
        println!(
            "[ERROR] Cannot enter the engine directory {}: {err}",
            home.display()
        );
        return Some(1);
    }

    Some(ffi::call_engine(&argv))
}

#[cfg(all(feature = "embedded-engine", windows))]
mod ffi {
    //! Calling `nemesis_engine_main(int, wchar_t**)`.

    use std::ffi::{OsString, c_int};
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "C" {
        /// Engine entry point; never unwinds (the C++ side catches everything).
        fn nemesis_engine_main(argc: c_int, argv: *mut *mut u16) -> c_int;
    }

    /// Converts `argv` to null-terminated UTF-16 and calls the engine.
    ///
    /// UTF-16 is passed directly so that non-ASCII paths (e.g. Japanese folder
    /// names) reach the engine unchanged.
    pub fn call_engine(argv: &[OsString]) -> i32 {
        let mut wide: Vec<Vec<u16>> = argv
            .iter()
            .map(|arg| arg.encode_wide().chain(std::iter::once(0)).collect())
            .collect();
        let mut pointers: Vec<*mut u16> = wide.iter_mut().map(|arg| arg.as_mut_ptr()).collect();
        pointers.push(std::ptr::null_mut());

        let Ok(argc) = c_int::try_from(argv.len()) else {
            println!("[ERROR] Too many engine arguments");
            return 1;
        };

        // SAFETY: `pointers` holds `argc` valid, null-terminated UTF-16 strings
        // followed by a null pointer, all alive (owned by `wide`) for the call.
        unsafe { nemesis_engine_main(argc, pointers.as_mut_ptr()) }
    }
}

#[cfg(not(all(feature = "embedded-engine", windows)))]
mod ffi {
    //! Fallback when the engine is not linked into this build.

    use std::ffi::OsString;

    /// Reports that this build has no embedded engine.
    pub fn call_engine(_argv: &[OsString]) -> i32 {
        println!(
            "[ERROR] This build does not contain the Nemesis engine (feature `embedded-engine`)"
        );
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_engine_mode_arguments() {
        let home = Path::new("D:/Skyrim/Data/Nemesis_Engine");
        let args = self_args(
            home,
            vec!["-p".into(), "amd64".into(), "-m".into(), "攻撃".into()],
        );

        let (parsed_home, engine_args) = parse_self_args(&args).unwrap();
        assert_eq!(parsed_home, home);
        assert_eq!(engine_args, ["-p", "amd64", "-m", "攻撃"]);

        let argv = engine_argv(&parsed_home, engine_args);
        assert_eq!(argv[0], home.join("Nemesis_Engine.exe").into_os_string());
        assert_eq!(argv.len(), 5);
    }

    #[test]
    fn normal_start_is_not_engine_mode() {
        assert!(parse_self_args(&[]).is_none());
        assert!(parse_self_args(&["--other".into()]).is_none());
        assert!(parse_self_args(&[ENGINE_MODE_ARG.into()]).is_none());
    }
}
