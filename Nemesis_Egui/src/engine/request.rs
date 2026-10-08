//! The engine command line.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use crate::config::Platform;

/// Everything needed to start one patch run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRequest {
    /// Path of `Nemesis_Engine(.exe)`.
    pub engine_path: PathBuf,
    /// Skyrim data directory (`-d`). Omitted when empty.
    pub data_dir: String,
    /// Output directory (`-o`). Omitted when empty.
    pub output_dir: String,
    /// Output platform (`-p`).
    pub platform: Platform,
    /// Adds `-db`.
    pub debug_mode: bool,
    /// Adds `-s`.
    pub synchronous: bool,
    /// Active mod codes, lowest priority first (see [`crate::mods::active_mod_codes`]).
    pub mods: Vec<String>,
}

impl PatchRequest {
    /// Builds the engine command line (without the program name).
    ///
    /// `-m` must be the last switch because the engine treats every following
    /// argument as a mod code.
    pub fn to_args(&self) -> Vec<OsString> {
        let mut args: Vec<OsString> =
            vec!["-p".into(), self.platform.engine_arg().into(), "-pi".into()];

        if !self.data_dir.trim().is_empty() {
            args.push("-d".into());
            args.push(self.data_dir.trim().into());
        }

        if !self.output_dir.trim().is_empty() {
            args.push("-o".into());
            args.push(self.output_dir.trim().into());
        }

        if self.debug_mode {
            args.push("-db".into());
        }

        if self.synchronous {
            args.push("-s".into());
        }

        args.push("-m".into());
        args.extend(self.mods.iter().map(OsString::from));
        args
    }

    /// Returns the full command line as one human readable string for the log.
    /// Arguments containing spaces are quoted.
    pub fn display_command_line(&self) -> String {
        let args: Vec<String> = self
            .to_args()
            .iter()
            .map(|arg| quote(&arg.to_string_lossy()))
            .collect();
        format!("\"{}\" {}", self.engine_path.display(), args.join(" "))
    }

    /// Checks the directories before launching: the data directory must be
    /// readable and the output directory is created when missing.
    ///
    /// Directories are probed with `read_dir` rather than `Path::is_dir`, which can
    /// report false negatives inside MO2's virtual file system.
    ///
    /// # Errors
    /// Returns a message describing the first problem found.
    pub fn prepare(&self) -> Result<(), String> {
        let data_dir = self.data_dir.trim();
        if !data_dir.is_empty() && fs::read_dir(data_dir).is_err() {
            return Err(format!("Skyrim data directory cannot be read: {data_dir}"));
        }

        let output_dir = self.output_dir.trim();
        if !output_dir.is_empty() {
            // `create_dir_all` checks `is_dir` internally, so only trust its error
            // when the directory really cannot be listed afterwards.
            if let Err(err) = fs::create_dir_all(output_dir)
                && fs::read_dir(output_dir).is_err()
            {
                return Err(format!("Cannot create output directory: {err}"));
            }
        }

        Ok(())
    }
}

/// Wraps `arg` in double quotes when it contains a space.
fn quote(arg: &str) -> String {
    if arg.contains(' ') {
        format!("\"{arg}\"")
    } else {
        arg.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PatchRequest {
        PatchRequest {
            engine_path: PathBuf::from("Nemesis_Engine.exe"),
            data_dir: "D:/Skyrim Special Edition/Data".into(),
            output_dir: String::new(),
            platform: Platform::Amd64,
            debug_mode: true,
            synchronous: false,
            mods: vec!["tkuc".into(), "bcbi".into()],
        }
    }

    #[test]
    fn builds_arguments_with_mods_last() {
        let args: Vec<_> = request()
            .to_args()
            .into_iter()
            .map(|a| a.into_string().unwrap())
            .collect();
        assert_eq!(
            args,
            [
                "-p",
                "amd64",
                "-pi",
                "-d",
                "D:/Skyrim Special Edition/Data",
                "-db",
                "-m",
                "tkuc",
                "bcbi"
            ]
        );
    }

    #[test]
    fn quotes_arguments_with_spaces() {
        assert_eq!(
            request().display_command_line(),
            "\"Nemesis_Engine.exe\" -p amd64 -pi -d \"D:/Skyrim Special Edition/Data\" -db -m tkuc bcbi"
        );
    }

    #[test]
    fn rejects_missing_data_directory() {
        let request = PatchRequest {
            data_dir: "Z:/definitely/missing/dir".into(),
            ..request()
        };
        assert!(request.prepare().is_err());
    }
}
