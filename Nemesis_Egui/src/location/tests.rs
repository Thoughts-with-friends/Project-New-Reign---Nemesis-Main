//! Tests for [`super::Locations`].

use std::fs;
use std::path::{Path, PathBuf};

use super::{Environment, Locations};
use crate::config::AppConfig;

/// A fake MO2 install: `<root>/ModOrganizer.ini`, `<root>/mods/{a,b,Nemesis_egui}`.
fn mo2_tree(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("nemesis_egui_loc_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for dir in ["mods/a", "mods/b", "mods/Nemesis_egui"] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    fs::write(root.join("ModOrganizer.ini"), "").unwrap();
    root
}

fn env(data: Option<&str>) -> Environment {
    Environment {
        exe: None,
        detected_data_dir: data.map(PathBuf::from),
        detected_by: data.map(|_| "test".to_owned()),
    }
}

#[test]
fn empty_field_uses_the_detected_game_data() {
    let locations = Locations::resolve_with(&AppConfig::default(), &env(Some("G:/Skyrim/Data")));

    assert_eq!(
        locations.data_source,
        Path::new("G:/Skyrim/Data").display().to_string()
    );
    assert_eq!(locations.default_data_source, locations.data_source);
    assert_eq!(locations.detected_by.as_deref(), Some("test"));
    assert!(!locations.is_mod_folders);
    assert_eq!(
        locations.engine_data_dir,
        Some(PathBuf::from("G:/Skyrim/Data"))
    );
    assert_eq!(
        locations.mod_dirs,
        [
            Path::new("G:/Skyrim/Data/Nemesis_Engine/mod").to_path_buf(),
            Path::new("G:/Skyrim/Data/Nemesis_Engine/mods").to_path_buf(),
        ]
    );
}

#[test]
fn explicit_glob_scans_mo2_mod_folders() {
    let root = mo2_tree("glob");
    let config = AppConfig {
        data_dir: root.join("mods").join("*").display().to_string(),
        ..AppConfig::default()
    };
    let locations = Locations::resolve_with(&config, &env(Some("G:/Skyrim/Data")));
    let _ = fs::remove_dir_all(&root);

    assert!(locations.is_mod_folders);
    assert_eq!(locations.data_roots.len(), 3);
    assert!(
        locations
            .mod_dirs
            .contains(&root.join("mods/a").join("Nemesis_Engine").join("mod"))
    );
    // Mod folders cannot be passed to the engine; the game's Data is used instead.
    assert_eq!(
        locations.engine_data_dir,
        Some(PathBuf::from("G:/Skyrim/Data"))
    );
}

#[test]
fn plain_mo2_mods_folder_is_treated_as_glob() {
    let root = mo2_tree("plain");
    let config = AppConfig {
        data_dir: root.join("mods").display().to_string(),
        ..AppConfig::default()
    };
    let locations = Locations::resolve_with(&config, &env(None));
    let _ = fs::remove_dir_all(&root);

    assert!(locations.is_mod_folders);
    assert_eq!(locations.data_roots.len(), 3);
    assert_eq!(locations.engine_data_dir, None);
}

#[test]
fn explicit_data_directory_wins_over_detection() {
    let config = AppConfig {
        data_dir: "G:/Skyrim/Data".into(),
        ..AppConfig::default()
    };
    let locations = Locations::resolve_with(&config, &env(Some("X:/other/Data")));

    assert_eq!(
        locations.engine_data_dir,
        Some(PathBuf::from("G:/Skyrim/Data"))
    );
    assert!(
        locations
            .engine_path
            .starts_with("G:/Skyrim/Data/Nemesis_Engine")
    );
}

#[test]
fn explicit_engine_settings_win() {
    let config = AppConfig {
        data_dir: "G:/MO2/mods/*".into(),
        engine_data_dir: "H:/Data".into(),
        engine_path: "H:/tools/Nemesis_Engine.exe".into(),
        ..AppConfig::default()
    };
    let locations = Locations::resolve_with(&config, &env(Some("X:/Data")));

    assert_eq!(locations.engine_data_arg(), "H:/Data");
    assert_eq!(
        locations.engine_path,
        PathBuf::from("H:/tools/Nemesis_Engine.exe")
    );
    assert!(locations.mod_dirs.contains(&PathBuf::from("H:/tools/mods")));
}
