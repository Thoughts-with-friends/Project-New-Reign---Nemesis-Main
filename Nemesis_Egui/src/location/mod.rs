//! Resolving the settings into concrete paths: where mods are scanned, which
//! Skyrim `Data` directory the engine gets, and where the engine lives.
//!
//! The *data* field accepts, like D-Merge:
//!
//! * a Skyrim `Data` directory, e.g. the game's own folder seen through MO2's
//!   virtual file system;
//! * a glob over mod folders, e.g. `D:\MO2\mods\*`, where every match is laid
//!   out like `Data` (an MO2 mod folder);
//! * an MO2 `mods` folder without `\*`, which is treated as `mods\*`.
//!
//! When the field is empty, the Skyrim `Data` directory of this user's game
//! install is detected (see [`detect`]). Under MO2 the game's `Data` is
//! virtualized, so it shows every enabled mod and the engine's `meshes`
//! templates in one place.
//!
//! * [`glob`]: wildcard expansion;
//! * [`detect`]: MO2 / Skyrim detection.

pub mod detect;
pub mod glob;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::engine::embedded;

/// File name of the engine executable.
const ENGINE_EXE: &str = embedded::ENGINE_EXE_NAME;

/// Facts about the running process that path resolution depends on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    /// Path of the running GUI executable.
    pub exe: Option<PathBuf>,
    /// Automatically detected Skyrim `Data` directory.
    pub detected_data_dir: Option<PathBuf>,
    /// How `detected_data_dir` was found, for diagnostics.
    pub detected_by: Option<String>,
}

impl Environment {
    /// Inspects the current process (executable path, registry).
    pub fn current(config: &AppConfig) -> Self {
        let exe = std::env::current_exe().ok();
        let detected = detect::detect_data_dir(config.platform, exe.as_deref());
        Self {
            exe,
            detected_by: detected.as_ref().map(|found| found.method.clone()),
            detected_data_dir: detected.map(|found| found.data_dir),
        }
    }
}

/// Concrete paths derived from the settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Locations {
    /// Value used for the data field (the field itself, or the default when empty).
    pub data_source: String,
    /// What an empty data field resolves to: the detected Skyrim `Data` directory.
    pub default_data_source: String,
    /// How the Skyrim `Data` directory was detected, if it was.
    pub detected_by: Option<String>,
    /// Whether `data_source` addresses MO2 mod folders rather than one `Data` directory.
    pub is_mod_folders: bool,
    /// Directories laid out like `Data` (one per matched mod folder in glob mode).
    pub data_roots: Vec<PathBuf>,
    /// Skyrim `Data` directory passed to the engine with `-d`.
    pub engine_data_dir: Option<PathBuf>,
    /// Engine executable to launch. For the built-in engine this is only a virtual
    /// path; its directory is the engine home (`mods`, `behavior_templates`, ...).
    pub engine_path: PathBuf,
    /// Whether the engine linked into this executable is used (no explicit path).
    pub embedded_engine: bool,
    /// Directories whose sub folders are scanned for `info.ini`.
    pub mod_dirs: Vec<PathBuf>,
}

impl Locations {
    /// Resolves `config` for the current process.
    pub fn resolve(config: &AppConfig) -> Self {
        Self::resolve_with(config, &Environment::current(config))
    }

    /// Resolves `config` against an explicit environment.
    pub fn resolve_with(config: &AppConfig, env: &Environment) -> Self {
        let default_data_source = env
            .detected_data_dir
            .as_ref()
            .map(|dir| dir.display().to_string())
            .unwrap_or_default();

        let field = config.data_dir.trim();
        let data_source = if field.is_empty() {
            default_data_source.clone()
        } else {
            field.to_owned()
        };

        let (data_roots, is_mod_folders) = data_roots(&data_source);
        let engine_data_dir = engine_data_dir(config, &data_source, is_mod_folders, env);
        let engine_path = engine_path(config, engine_data_dir.as_deref());
        let embedded_engine = embedded::AVAILABLE && config.engine_path.trim().is_empty();
        let mod_dirs = mod_dirs(&data_roots, &engine_path);

        Self {
            data_source,
            default_data_source,
            detected_by: env.detected_by.clone(),
            is_mod_folders,
            data_roots,
            engine_data_dir,
            engine_path,
            embedded_engine,
            mod_dirs,
        }
    }

    /// `-d` argument for the engine (empty when nothing could be detected).
    pub fn engine_data_arg(&self) -> String {
        self.engine_data_dir
            .as_ref()
            .map(|dir| dir.display().to_string())
            .unwrap_or_default()
    }
}

/// Expands the data source into `Data`-like roots. Returns whether they are MO2
/// mod folders.
fn data_roots(source: &str) -> (Vec<PathBuf>, bool) {
    if source.is_empty() {
        return (Vec::new(), false);
    }

    if glob::has_wildcard(source) {
        return (glob::expand_dirs(source), true);
    }

    let path = Path::new(source);
    if detect::is_mo2_mods_dir(path) {
        return (
            glob::expand_dirs(&path.join("*").display().to_string()),
            true,
        );
    }

    (vec![path.to_path_buf()], false)
}

/// Picks the `-d` directory: the explicit setting, else the data field when it is
/// a single `Data` directory, else the detected game `Data` directory (which MO2
/// virtualizes to include every enabled mod).
fn engine_data_dir(
    config: &AppConfig,
    data_source: &str,
    is_mod_folders: bool,
    env: &Environment,
) -> Option<PathBuf> {
    let explicit = config.engine_data_dir.trim();
    if !explicit.is_empty() {
        return Some(PathBuf::from(explicit));
    }

    if !is_mod_folders && !data_source.is_empty() {
        return Some(PathBuf::from(data_source));
    }

    env.detected_data_dir.clone()
}

/// The explicit engine path, else `<engine data dir>/Nemesis_Engine/Nemesis_Engine(.exe)`.
fn engine_path(config: &AppConfig, engine_data_dir: Option<&Path>) -> PathBuf {
    let explicit = config.engine_path.trim();
    if !explicit.is_empty() {
        return PathBuf::from(explicit);
    }

    engine_data_dir
        .map(|dir| dir.join("Nemesis_Engine").join(ENGINE_EXE))
        .unwrap_or_else(|| PathBuf::from(ENGINE_EXE))
}

/// Mod directories: `<root>/Nemesis_Engine/mod` for every root (the layout used by
/// published Nemesis mods) and `<engine dir>/mods` (read by this engine),
/// without duplicates.
fn mod_dirs(data_roots: &[PathBuf], engine_path: &Path) -> Vec<PathBuf> {
    let engine_mods = engine_path.parent().map(|dir| dir.join("mods"));
    let mut seen = HashSet::new();

    data_roots
        .iter()
        .map(|root| root.join("Nemesis_Engine").join("mod"))
        .chain(engine_mods)
        // Windows paths are case-insensitive.
        .filter(|dir| seen.insert(dir.to_string_lossy().to_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests;
