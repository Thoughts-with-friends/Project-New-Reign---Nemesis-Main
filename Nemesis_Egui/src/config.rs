//! Persistent user settings.
//!
//! The configuration is stored as pretty-printed JSON next to the GUI executable
//! (`nemesis_egui.json`), so that portable installs keep their settings together
//! with the program.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File name of the configuration file placed next to the executable.
pub const CONFIG_FILE_NAME: &str = "nemesis_egui.json";

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
    /// Skyrim `Data` directory passed to the engine with `-d`.
    pub data_dir: String,
    /// Output (staging) directory passed with `-o`. Empty means "write into the data directory".
    pub output_dir: String,
    /// Explicit path of `Nemesis_Engine(.exe)`. Empty means [`AppConfig::default_engine_path`].
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

impl AppConfig {
    /// Returns the location of the configuration file (next to the running executable).
    pub fn file_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_default()
            .join(CONFIG_FILE_NAME)
    }

    /// Loads the configuration from [`AppConfig::file_path`].
    ///
    /// A missing or malformed file yields the default configuration instead of an error,
    /// because the GUI must always be able to start.
    pub fn load() -> Self {
        Self::load_from(&Self::file_path()).unwrap_or_default()
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

    /// Saves the configuration to an explicit path.
    ///
    /// # Errors
    /// Returns an error when serialization fails or the file cannot be written.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, text)
    }

    /// Default engine location: `<data>/nemesis_engine/Nemesis_Engine(.exe)`,
    /// matching where the original Qt launcher looks for it.
    pub fn default_engine_path(&self) -> PathBuf {
        let exe_name = if cfg!(windows) {
            "Nemesis_Engine.exe"
        } else {
            "Nemesis_Engine"
        };
        Path::new(&self.data_dir)
            .join("nemesis_engine")
            .join(exe_name)
    }

    /// Returns the engine path that will actually be launched.
    pub fn resolved_engine_path(&self) -> PathBuf {
        if self.engine_path.trim().is_empty() {
            self.default_engine_path()
        } else {
            PathBuf::from(self.engine_path.trim())
        }
    }

    /// Directory scanned for mods. The engine only loads patches from `<engine dir>/mods`,
    /// so the GUI derives the list from the same place.
    pub fn mods_dir(&self) -> PathBuf {
        self.resolved_engine_path()
            .parent()
            .map(|dir| dir.join("mods"))
            .unwrap_or_else(|| PathBuf::from("mods"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let path =
            std::env::temp_dir().join(format!("nemesis_egui_cfg_{}.json", std::process::id()));
        let config = AppConfig {
            data_dir: "D:/Skyrim/Data".into(),
            output_dir: "D:/MO2/mods/Nemesis Output 用".into(),
            mod_order: vec![ModOrderEntry {
                code: "tkuc".into(),
                checked: false,
            }],
            ..AppConfig::default()
        };

        config.save_to(&path).unwrap();
        let loaded = AppConfig::load_from(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(config, loaded);
    }

    #[test]
    fn derives_engine_and_mods_dir_from_data_dir() {
        let config = AppConfig {
            data_dir: "C:/Data".into(),
            ..AppConfig::default()
        };
        let engine = config.resolved_engine_path();

        assert!(engine.starts_with("C:/Data/nemesis_engine"));
        assert_eq!(config.mods_dir(), Path::new("C:/Data/nemesis_engine/mods"));
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let config: AppConfig = serde_json::from_str(r#"{ "data_dir": "X" }"#).unwrap();

        assert_eq!(config.data_dir, "X");
        assert_eq!(config.platform, Platform::Amd64);
        assert!(config.enable_new_mods);
    }
}
