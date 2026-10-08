//! Persistent user settings.
//!
//! Settings are stored as JSON in the user's configuration directory
//! (`%APPDATA%\Nemesis_Egui\settings.json` on Windows). That location is outside
//! the folders Mod Organizer 2 virtualizes, so it behaves the same whether the GUI
//! runs inside or outside MO2. Older versions wrote `nemesis_egui.json` next to
//! the executable; that file is migrated on first load.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Directory name inside the user's configuration directory.
const CONFIG_DIR_NAME: &str = "Nemesis_Egui";
/// File name of the settings file.
const CONFIG_FILE_NAME: &str = "settings.json";
/// File name used by older versions, next to the executable.
const LEGACY_FILE_NAME: &str = "nemesis_egui.json";

/// Output platform understood by the engine's `-p` argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// Skyrim Legendary Edition (32-bit Havok).
    #[default]
    Win32,
    /// Skyrim Special/Anniversary Edition (64-bit Havok).
    Amd64,
    /// PlayStation 3.
    Ps3,
    /// PlayStation 4.
    Ps4,
    /// Xbox 360.
    Xb360,
}

impl Platform {
    /// Every supported platform, in the order shown in the settings combo box.
    pub const ALL: [Self; 5] = [Self::Win32, Self::Amd64, Self::Ps3, Self::Ps4, Self::Xb360];

    /// Returns the value passed to the engine after `-p`.
    pub const fn engine_arg(self) -> &'static str {
        match self {
            Self::Win32 => "win32",
            Self::Amd64 => "amd64",
            Self::Ps3 => "ps3",
            Self::Ps4 => "ps4",
            Self::Xb360 => "360",
        }
    }

    /// Returns a human readable label for the UI.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Win32 => "win32 (Skyrim LE)",
            Self::Amd64 => "amd64 (Skyrim SE/AE)",
            Self::Ps3 => "ps3",
            Self::Ps4 => "ps4",
            Self::Xb360 => "xb360",
        }
    }
}

/// Saved position and activation state of a single mod in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModOrderEntry {
    /// Lower-case mod code (the mod's folder name).
    pub code: String,
    /// Whether the mod is enabled for patching.
    pub checked: bool,
}

/// All settings persisted between sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Where mods are read from: a Skyrim `Data` directory or a glob over MO2 mod
    /// folders such as `D:\MO2\mods\*` (see [`crate::location`]). Empty = automatic.
    pub data_dir: String,
    /// Skyrim `Data` directory passed to the engine with `-d`. Empty = automatic.
    pub engine_data_dir: String,
    /// Output (staging) directory passed with `-o`. Empty means "write into the data directory".
    pub output_dir: String,
    /// Explicit path of `Nemesis_Engine(.exe)`. Empty = `<engine data dir>/Nemesis_Engine/`.
    pub engine_path: String,
    /// Output platform passed with `-p`.
    pub platform: Platform,
    /// Passes `-db` (engine debug mode) when enabled.
    pub debug_mode: bool,
    /// Passes `-s` (synchronous processing) when enabled.
    pub synchronous: bool,
    /// Newly discovered mods are enabled by default when `true`.
    pub enable_new_mods: bool,
    /// Uses the dark theme when `true`.
    pub dark_mode: bool,
    /// Mod order from top (first merged) to bottom (last merged, wins conflicts).
    pub mod_order: Vec<ModOrderEntry>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: String::new(),
            engine_data_dir: String::new(),
            output_dir: String::new(),
            engine_path: String::new(),
            platform: Platform::Amd64,
            debug_mode: false,
            synchronous: false,
            enable_new_mods: true,
            dark_mode: true,
            mod_order: Vec::new(),
        }
    }
}

/// Where [`AppConfig::load`] got the settings from, for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadedFrom {
    /// The settings file.
    File(PathBuf),
    /// The legacy file next to the executable (copied to the new location on save).
    Legacy(PathBuf),
    /// No readable file; defaults were used.
    Defaults,
}

impl AppConfig {
    /// Location of the settings file: `<config dir>/Nemesis_Egui/settings.json`.
    ///
    /// The config dir is `%APPDATA%` on Windows and `$XDG_CONFIG_HOME` or
    /// `~/.config` elsewhere; the executable's directory is the last resort.
    pub fn file_path() -> PathBuf {
        config_base_dir()
            .unwrap_or_else(exe_dir)
            .join(CONFIG_DIR_NAME)
            .join(CONFIG_FILE_NAME)
    }

    /// Location of the settings file written by older versions.
    pub fn legacy_file_path() -> PathBuf {
        exe_dir().join(LEGACY_FILE_NAME)
    }

    /// Loads the settings, falling back to the legacy file and then to defaults,
    /// so the GUI can always start.
    pub fn load() -> (Self, LoadedFrom) {
        Self::load_first(&Self::file_path(), &Self::legacy_file_path())
    }

    /// Loads from `path`, else from `legacy`, else returns defaults.
    pub fn load_first(path: &Path, legacy: &Path) -> (Self, LoadedFrom) {
        if let Ok(config) = Self::load_from(path) {
            return (config, LoadedFrom::File(path.to_path_buf()));
        }

        match Self::load_from(legacy) {
            Ok(config) => (config, LoadedFrom::Legacy(legacy.to_path_buf())),
            Err(_) => (Self::default(), LoadedFrom::Defaults),
        }
    }

    /// Loads the configuration from an explicit path.
    ///
    /// # Errors
    /// Returns an error when the file cannot be read or is not valid JSON.
    pub fn load_from(path: &Path) -> io::Result<Self> {
        let text = fs::read_to_string(path)?;
        serde_json::from_str(&text).map_err(io::Error::other)
    }

    /// Saves the configuration to [`AppConfig::file_path`].
    ///
    /// # Errors
    /// Returns an error when the file cannot be written.
    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::file_path())
    }

    /// Saves the configuration to an explicit path, creating its directory.
    ///
    /// # Errors
    /// Returns an error when serialization fails or the file cannot be written.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, text)
    }
}

/// The per-user configuration directory, if the environment provides one.
fn config_base_dir() -> Option<PathBuf> {
    let from_env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());

    if cfg!(windows) {
        from_env("APPDATA").map(PathBuf::from)
    } else {
        from_env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| from_env("HOME").map(|home| PathBuf::from(home).join(".config")))
    }
}

/// Directory of the running executable (empty when unknown).
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("nemesis_egui_cfg_{}", std::process::id()))
            .join(name)
    }

    #[test]
    fn round_trips_through_json_and_creates_the_directory() {
        let path = temp_path("nested/settings.json");
        let config = AppConfig {
            data_dir: "D:/MO2/mods/*".into(),
            output_dir: "D:/MO2/mods/Nemesis Output 用".into(),
            mod_order: vec![ModOrderEntry {
                code: "tkuc".into(),
                checked: false,
            }],
            ..AppConfig::default()
        };

        config.save_to(&path).unwrap();
        let loaded = AppConfig::load_from(&path).unwrap();
        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());

        assert_eq!(config, loaded);
    }

    #[test]
    fn falls_back_to_legacy_file_then_defaults() {
        let path = temp_path("missing/settings.json");
        let legacy = temp_path("legacy.json");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, r#"{ "data_dir": "D:\\MO2\\mods" }"#).unwrap();

        let (config, from) = AppConfig::load_first(&path, &legacy);
        let _ = fs::remove_file(&legacy);
        let (defaults, none) = AppConfig::load_first(&path, &legacy);

        assert_eq!(config.data_dir, "D:\\MO2\\mods");
        assert_eq!(from, LoadedFrom::Legacy(legacy));
        assert_eq!(defaults, AppConfig::default());
        assert_eq!(none, LoadedFrom::Defaults);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let config: AppConfig = serde_json::from_str(r#"{ "data_dir": "X" }"#).unwrap();

        assert_eq!(config.data_dir, "X");
        assert_eq!(config.engine_data_dir, "");
        assert_eq!(config.platform, Platform::Amd64);
        assert!(config.enable_new_mods);
    }

    #[test]
    fn settings_live_outside_the_executable_directory() {
        if config_base_dir().is_some() {
            assert!(!AppConfig::file_path().starts_with(exe_dir()));
        }
    }
}
