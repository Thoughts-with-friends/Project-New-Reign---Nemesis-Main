//! Human-readable diagnostics written to the log panel.
//!
//! Running under Mod Organizer 2 changes how paths resolve (virtual file system,
//! working directory, executable location), so the GUI reports what it actually
//! sees. Users can copy these lines when something is not detected.

use std::path::Path;

use crate::config::{AppConfig, LoadedFrom};
use crate::location::Locations;
use crate::mods::ScanReport;

/// Maximum number of per-directory lines listed after a scan.
const MAX_LISTED_DIRS: usize = 30;

/// Lines describing the process and where the settings came from.
pub fn startup_lines(loaded_from: &LoadedFrom) -> Vec<String> {
    let show = |path: Option<&Path>| {
        path.map_or_else(|| "(unknown)".to_owned(), |p| p.display().to_string())
    };
    let exe = std::env::current_exe().ok();
    let cwd = std::env::current_dir().ok();

    let settings = match loaded_from {
        LoadedFrom::File(path) => format!("Settings: {}", path.display()),
        LoadedFrom::Legacy(path) => format!(
            "Settings: migrated from {} to {}",
            path.display(),
            AppConfig::file_path().display()
        ),
        LoadedFrom::Defaults => format!(
            "Settings: defaults (will be saved to {})",
            AppConfig::file_path().display()
        ),
    };

    vec![
        format!("Executable: {}", show(exe.as_deref())),
        format!("Working directory: {}", show(cwd.as_deref())),
        settings,
    ]
}

/// Lines describing the resolved paths and the result of a mod scan.
///
/// Only directories that contain mods or failed to read are listed, because a
/// glob over MO2's `mods\*` usually matches hundreds of folders without patches.
pub fn scan_lines(locations: &Locations, report: &ScanReport) -> Vec<String> {
    let source_kind = if locations.is_mod_folders {
        format!("{} MO2 mod folders", locations.data_roots.len())
    } else {
        "Skyrim Data directory".to_owned()
    };
    let engine_data = locations
        .engine_data_dir
        .as_ref()
        .map_or_else(|| "(not found)".to_owned(), |dir| dir.display().to_string());

    let detected = locations.detected_by.as_deref().map_or_else(
        || "Skyrim Data auto-detection: not found".to_owned(),
        |method| format!("Skyrim Data auto-detected via {method}"),
    );

    let mut lines = vec![
        detected,
        format!(
            "Mod source: {} ({source_kind})",
            or_none(&locations.data_source)
        ),
        format!("Skyrim Data for the engine (-d): {engine_data}"),
        engine_line(locations),
    ];

    let interesting: Vec<String> = report
        .dirs
        .iter()
        .filter_map(|dir| match &dir.result {
            Ok(0) => None,
            Ok(count) => Some(format!("  {count:>3} mods  {}", dir.dir.display())),
            Err(err) => Some(format!("  error    {}: {err}", dir.dir.display())),
        })
        .collect();

    let total: usize = report
        .dirs
        .iter()
        .filter_map(|dir| dir.result.as_ref().ok())
        .sum();
    lines.push(format!(
        "Scanned {} directories, found {total} mods{}",
        report.dirs.len(),
        if interesting.is_empty() { "" } else { ":" }
    ));

    let hidden = interesting.len().saturating_sub(MAX_LISTED_DIRS);
    lines.extend(interesting.into_iter().take(MAX_LISTED_DIRS));
    if hidden > 0 {
        lines.push(format!("  … and {hidden} more"));
    }

    if !report.duplicates.is_empty() {
        lines.push(format!(
            "Duplicate mod codes (first one kept): {}",
            report.duplicates.join(", ")
        ));
    }

    lines
}

/// Which engine runs: the built-in one (with its home directory) or an external exe.
fn engine_line(locations: &Locations) -> String {
    if locations.embedded_engine {
        let home = locations.engine_path.parent().unwrap_or(Path::new(""));
        format!("Engine: built-in (resources in {})", home.display())
    } else {
        format!("Engine: {}", locations.engine_path.display())
    }
}

/// `(none)` for an empty string.
fn or_none(text: &str) -> &str {
    if text.is_empty() { "(none)" } else { text }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::mods::scan_dirs;

    #[test]
    fn lists_only_directories_with_mods_or_errors() {
        let locations = Locations {
            data_source: "D:/MO2/mods/*".into(),
            is_mod_folders: true,
            data_roots: vec![
                PathBuf::from("D:/MO2/mods/a"),
                PathBuf::from("D:/MO2/mods/b"),
            ],
            ..Locations::default()
        };
        let report = scan_dirs(&[
            PathBuf::from("Z:/missing/one"),
            PathBuf::from("Z:/missing/two"),
        ]);

        let lines = scan_lines(&locations, &report);

        assert!(lines[0].ends_with("not found"));
        assert!(lines[1].contains("2 MO2 mod folders"));
        assert!(lines[2].ends_with("(not found)"));
        assert_eq!(
            lines.last().map(String::as_str),
            Some("Scanned 2 directories, found 0 mods")
        );
    }

    #[test]
    fn startup_reports_settings_origin() {
        let lines = startup_lines(&LoadedFrom::Legacy(PathBuf::from("old.json")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("migrated from old.json"))
        );
    }
}
