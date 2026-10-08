//! Detecting where Skyrim and Mod Organizer 2 live.
//!
//! Existence checks such as `Path::exists` / `is_dir` can give false negatives
//! inside MO2's virtual file system, so detection only relies on path names,
//! `read_dir` and the registry.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Platform;

/// Files that mark the folder containing MO2's `mods` directory.
const MO2_MARKERS: [&str; 2] = ["ModOrganizer.ini", "ModOrganizer.exe"];

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

/// The default mod source when the GUI runs from inside an MO2 mod folder
/// (`<MO2>/mods/<this mod>/Nemesis_Egui.exe`): `<MO2>/mods/*`.
pub fn mo2_mods_pattern(exe: &Path) -> Option<String> {
    let mods_dir = exe.parent()?.parent()?;
    is_mo2_mods_dir(mods_dir).then(|| mods_dir.join("*").display().to_string())
}

/// The closest ancestor of `path` named `Data` (case-insensitive).
pub fn data_ancestor(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|dir| {
            dir.file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("data"))
        })
        .map(Path::to_path_buf)
}

/// Finds the Skyrim `Data` directory: first an ancestor of the executable or the
/// working directory named `Data`, then the game's install path in the registry.
pub fn detect_data_dir(platform: Platform, exe: Option<&Path>) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok();

    exe.and_then(data_ancestor)
        .or_else(|| cwd.as_deref().and_then(data_ancestor))
        .or_else(|| registry_data_dir(platform))
}

/// Reads `<install path>/Data` from the registry entry written by Steam.
#[cfg(windows)]
pub fn registry_data_dir(platform: Platform) -> Option<PathBuf> {
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
    .map(|install| PathBuf::from(install.trim()).join("Data"))
}

/// Registry lookup is only available on Windows.
#[cfg(not(windows))]
pub const fn registry_data_dir(_platform: Platform) -> Option<PathBuf> {
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

    fn mo2_tree(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("nemesis_egui_mo2_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("mods/Nemesis_egui")).unwrap();
        fs::write(root.join("ModOrganizer.ini"), "").unwrap();
        root
    }

    #[test]
    fn detects_mo2_mods_folder_and_default_pattern() {
        let root = mo2_tree("detect");
        let exe = root.join("mods/Nemesis_egui/Nemesis_egui.exe");

        let is_mods = is_mo2_mods_dir(&root.join("mods"));
        let is_mod = is_mo2_mods_dir(&root.join("mods/Nemesis_egui"));
        let pattern = mo2_mods_pattern(&exe);
        let _ = fs::remove_dir_all(&root);

        assert!(is_mods);
        assert!(!is_mod);
        assert_eq!(
            pattern,
            Some(root.join("mods").join("*").display().to_string())
        );
    }

    #[test]
    fn no_pattern_outside_mo2() {
        let exe = std::env::temp_dir().join("a/b/Nemesis_egui.exe");
        assert_eq!(mo2_mods_pattern(&exe), None);
    }

    #[test]
    fn finds_data_ancestor() {
        let path = Path::new("D:/Skyrim Special Edition/Data/Nemesis_Engine/Nemesis_Egui.exe");
        assert_eq!(
            data_ancestor(path),
            Some(PathBuf::from("D:/Skyrim Special Edition/Data"))
        );
        assert_eq!(data_ancestor(Path::new("D:/MO2/mods/x/a.exe")), None);
    }
}
