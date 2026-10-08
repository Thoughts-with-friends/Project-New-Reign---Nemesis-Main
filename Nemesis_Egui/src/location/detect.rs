//! Detecting where Skyrim and Mod Organizer 2 live.
//!
//! The Skyrim `Data` directory is `<game dir>/Data`, where the game dir is the
//! folder containing the game executable (`SkyrimSE.exe`, ...). Every user
//! installs the game and launches this GUI from different places, so the game
//! dir is looked up, in order:
//!
//! 1. `gamePath` in MO2's `ModOrganizer.ini`, found in an ancestor of the
//!    executable or the working directory (the GUI lives in `<MO2>/mods/<mod>/`);
//! 2. an ancestor of the executable or the working directory that contains the
//!    game executable (the GUI lives inside the game folder);
//! 3. the install path Steam wrote to the registry.
//!
//! Existence checks such as `Path::exists` / `is_dir` can give false negatives
//! inside MO2's virtual file system, so detection only relies on path names,
//! `read_dir`, reading files and the registry.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Platform;

/// Files that mark the folder containing MO2's `mods` directory.
const MO2_MARKERS: [&str; 2] = ["ModOrganizer.ini", "ModOrganizer.exe"];
/// Game executables that mark a Skyrim install folder (SE/AE, VR, LE).
const GAME_EXES: [&str; 3] = ["SkyrimSE.exe", "SkyrimVR.exe", "TESV.exe"];

/// A detected Skyrim `Data` directory and how it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedData {
    /// `<game dir>/Data`.
    pub data_dir: PathBuf,
    /// Human-readable detection method, for diagnostics.
    pub method: String,
}

/// Finds the Skyrim `Data` directory for this user (see the module docs).
pub fn detect_data_dir(platform: Platform, exe: Option<&Path>) -> Option<DetectedData> {
    let cwd = std::env::current_dir().ok();
    let starts: Vec<&Path> = exe.into_iter().chain(cwd.as_deref()).collect();

    let found = |game_dir: PathBuf, method: String| DetectedData {
        data_dir: game_dir.join("Data"),
        method,
    };

    starts
        .iter()
        .find_map(|start| mo2_game_dir(start))
        .map(|(game_dir, ini)| found(game_dir, format!("MO2 gamePath in {}", ini.display())))
        .or_else(|| {
            starts
                .iter()
                .find_map(|start| game_dir_ancestor(start))
                .map(|game_dir| found(game_dir, "game executable next to this program".to_owned()))
        })
        .or_else(|| {
            registry_game_dir(platform).map(|game_dir| found(game_dir, "registry".to_owned()))
        })
}

/// Returns `true` when `dir` is an MO2 `mods` folder: it is named `mods` and its
/// parent contains `ModOrganizer.ini` or `ModOrganizer.exe`.
pub fn is_mo2_mods_dir(dir: &Path) -> bool {
    let named_mods = dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("mods"));

    named_mods
        && dir
            .parent()
            .is_some_and(|parent| contains_any(parent, &MO2_MARKERS))
}

/// Looks for `ModOrganizer.ini` in `start` and its ancestors and returns the game
/// dir it configures, together with the ini path.
fn mo2_game_dir(start: &Path) -> Option<(PathBuf, PathBuf)> {
    start.ancestors().find_map(|dir| {
        let ini = dir.join("ModOrganizer.ini");
        let bytes = fs::read(&ini).ok()?;
        parse_mo2_game_path(&String::from_utf8_lossy(&bytes)).map(|game| (game, ini))
    })
}

/// Extracts `gamePath` from the text of `ModOrganizer.ini`.
///
/// Qt writes it as `gamePath=@ByteArray(D:\\Steam\\...\\Skyrim Special Edition)`,
/// with backslashes doubled and non-ASCII bytes escaped as `\xHH`.
pub fn parse_mo2_game_path(ini: &str) -> Option<PathBuf> {
    let value = ini
        .lines()
        .find_map(|line| line.trim().strip_prefix("gamePath="))?
        .trim();
    let value = value
        .strip_prefix("@ByteArray(")
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(value);

    let path = unescape_qt(value);
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Decodes Qt ini escapes: `\\` → `\`, `\xHH` → byte; the bytes are UTF-8.
fn unescape_qt(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'x'
                && let Some(byte) = value
                    .get(i + 2..i + 4)
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                out.push(byte);
                i += 4;
                continue;
            }

            out.push(bytes[i + 1]);
            i += 2;
            continue;
        }

        out.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}

/// The closest ancestor of `start` (inclusive) that contains a game executable.
fn game_dir_ancestor(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .filter(|dir| !dir.as_os_str().is_empty())
        .find(|dir| contains_any(dir, &GAME_EXES))
        .map(Path::to_path_buf)
}

/// Reads the game install path that Steam wrote to the registry.
#[cfg(windows)]
fn registry_game_dir(platform: Platform) -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let game = match platform {
        Platform::Amd64 => "Skyrim Special Edition",
        Platform::Win32 => "Skyrim",
        Platform::Ps3 | Platform::Ps4 | Platform::Xb360 => return None,
    };

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    [
        "SOFTWARE\\WOW6432Node\\Bethesda Softworks",
        "SOFTWARE\\Bethesda Softworks",
    ]
    .iter()
    .find_map(|root| {
        let key = hklm.open_subkey(format!("{root}\\{game}")).ok()?;
        key.get_value::<String, _>("Installed Path").ok()
    })
    .map(|install| PathBuf::from(install.trim()))
}

/// Registry lookup is only available on Windows.
#[cfg(not(windows))]
const fn registry_game_dir(_platform: Platform) -> Option<PathBuf> {
    None
}

/// Returns `true` when `dir` directly contains an entry named like one of `names`.
fn contains_any(dir: &Path, names: &[&str]) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };

    entries.filter_map(Result::ok).any(|entry| {
        names
            .iter()
            .any(|name| entry.file_name().eq_ignore_ascii_case(name))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("nemesis_egui_detect_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn parses_mo2_game_path() {
        let ini = "[General]\r\ngamePath=@ByteArray(D:\\\\SteamLibrary\\\\steamapps\\\\common\\\\Skyrim Special Edition)\r\n";
        assert_eq!(
            parse_mo2_game_path(ini),
            Some(PathBuf::from(
                r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition"
            ))
        );

        // Non-ASCII bytes are escaped as UTF-8 `\xHH`: "ゲーム".
        let ini = "gamePath=@ByteArray(E:\\\\\\xe3\\x82\\xb2\\xe3\\x83\\xbc\\xe3\\x83\\xa0)";
        assert_eq!(parse_mo2_game_path(ini), Some(PathBuf::from(r"E:\ゲーム")));

        assert_eq!(
            parse_mo2_game_path("gamePath=C:/Games/Skyrim"),
            Some(PathBuf::from("C:/Games/Skyrim"))
        );
        assert_eq!(parse_mo2_game_path("[General]\nother=1"), None);
    }

    #[test]
    fn finds_game_dir_from_mo2_ini_above_the_executable() {
        let root = temp_root("mo2");
        fs::create_dir_all(root.join("mods/Nemesis_egui")).unwrap();
        fs::write(
            root.join("ModOrganizer.ini"),
            "[General]\ngamePath=@ByteArray(G:\\\\Games\\\\Skyrim Special Edition)\n",
        )
        .unwrap();

        let found = mo2_game_dir(&root.join("mods/Nemesis_egui/Nemesis_egui.exe"));
        let _ = fs::remove_dir_all(&root);

        let (game_dir, ini) = found.unwrap();
        assert_eq!(game_dir, PathBuf::from(r"G:\Games\Skyrim Special Edition"));
        assert!(ini.ends_with("ModOrganizer.ini"));
    }

    #[test]
    fn finds_game_dir_containing_the_game_executable() {
        let root = temp_root("game");
        fs::create_dir_all(root.join("Data/Nemesis_Engine")).unwrap();
        fs::write(root.join("SkyrimSE.exe"), "").unwrap();

        let from_inside = game_dir_ancestor(&root.join("Data/Nemesis_Engine/Nemesis_Egui.exe"));
        let elsewhere = game_dir_ancestor(&std::env::temp_dir().join("no/game/here.exe"));
        let _ = fs::remove_dir_all(&root);

        assert_eq!(from_inside, Some(root));
        assert_eq!(elsewhere, None);
    }

    #[test]
    fn detects_mo2_mods_folder() {
        let root = temp_root("mods");
        fs::create_dir_all(root.join("mods/x")).unwrap();
        fs::write(root.join("ModOrganizer.exe"), "").unwrap();

        let is_mods = is_mo2_mods_dir(&root.join("mods"));
        let is_mod = is_mo2_mods_dir(&root.join("mods/x"));
        let _ = fs::remove_dir_all(&root);

        assert!(is_mods);
        assert!(!is_mod);
    }
}
