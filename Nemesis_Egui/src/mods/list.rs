//! The ordered, checkable list of installed mods shown in the table.

use std::path::PathBuf;

use super::{
    ModEntry, ScanReport, active_mod_codes, apply_saved_order, move_item, scan_dirs, to_saved_order,
};
use crate::config::ModOrderEntry;

/// A change to the mod list requested by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListEdit {
    /// Flip the checkbox of one row.
    Toggle(usize),
    /// Set the checkbox of several rows.
    SetChecked {
        /// Row indices.
        indices: Vec<usize>,
        /// New state.
        checked: bool,
    },
    /// Move a row so that it ends up before `insert_before` (`0..=len`).
    Move {
        /// Index of the moved row.
        from: usize,
        /// Insertion index in the list before the move.
        insert_before: usize,
    },
    /// Sort all rows by display name.
    SortByName,
}

/// Installed mods in merge order (top = merged first), plus the last scan error.
#[derive(Debug, Clone, Default)]
pub struct ModList {
    /// Rows in merge order.
    entries: Vec<ModEntry>,
    /// Why the last scan failed, if it did.
    scan_error: Option<String>,
}

impl ModList {
    /// Scans `dirs` and orders the result according to `saved`.
    ///
    /// Returns the list and the scan report (with its `mods` moved into the list).
    /// [`ModList::scan_error`] is set when nothing was found and the reason is
    /// known: no directories to scan, or directories that could not be read.
    pub fn scan(dirs: &[PathBuf], saved: &[ModOrderEntry], enable_new: bool) -> (Self, ScanReport) {
        let mut report = scan_dirs(dirs);
        let mods = std::mem::take(&mut report.mods);

        let scan_error = if !mods.is_empty() {
            None
        } else if dirs.is_empty() {
            Some(
                "No mod directory to scan. Set the Skyrim Data directory or an MO2 mods pattern."
                    .to_owned(),
            )
        } else {
            let failures: Vec<String> = report
                .dirs
                .iter()
                .filter_map(|dir| {
                    dir.result
                        .as_ref()
                        .err()
                        .map(|err| format!("{}: {err}", dir.dir.display()))
                })
                .collect();
            (!failures.is_empty())
                .then(|| format!("Cannot read mod directories:\n{}", failures.join("\n")))
        };

        let list = Self {
            entries: apply_saved_order(mods, saved, enable_new),
            scan_error,
        };
        (list, report)
    }

    /// All rows in merge order.
    pub fn entries(&self) -> &[ModEntry] {
        &self.entries
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` when no mods are listed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of checked rows.
    pub fn enabled_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.checked).count()
    }

    /// Why the last scan failed, if it did.
    pub fn scan_error(&self) -> Option<&str> {
        self.scan_error.as_deref()
    }

    /// Indices of the rows matching `search` (case-insensitive).
    pub fn visible_indices(&self, search: &str) -> Vec<usize> {
        let needle = search.trim().to_lowercase();
        (0..self.entries.len())
            .filter(|&i| self.entries[i].matches(&needle))
            .collect()
    }

    /// Applies an edit. Out-of-range indices are ignored.
    pub fn apply(&mut self, edit: ListEdit) {
        match edit {
            ListEdit::Toggle(index) => {
                if let Some(entry) = self.entries.get_mut(index) {
                    entry.checked = !entry.checked;
                }
            }
            ListEdit::SetChecked { indices, checked } => {
                for index in indices {
                    if let Some(entry) = self.entries.get_mut(index) {
                        entry.checked = checked;
                    }
                }
            }
            ListEdit::Move {
                from,
                insert_before,
            } => move_item(&mut self.entries, from, insert_before),
            ListEdit::SortByName => self
                .entries
                .sort_by_key(|entry| entry.info.name.to_lowercase()),
        }
    }

    /// Returns the order to persist, keeping `previous` entries of missing mods.
    pub fn saved_order(&self, previous: &[ModOrderEntry]) -> Vec<ModOrderEntry> {
        to_saved_order(&self.entries, previous)
    }

    /// Mod codes to pass to the engine, lowest priority first.
    pub fn active_codes(&self) -> Vec<String> {
        active_mod_codes(&self.entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::ModInfo;

    fn list(codes: &[&str]) -> ModList {
        let entries = codes
            .iter()
            .map(|code| ModEntry {
                info: ModInfo {
                    code: (*code).into(),
                    name: code.to_uppercase(),
                    ..ModInfo::default()
                },
                checked: true,
            })
            .collect();
        ModList {
            entries,
            scan_error: None,
        }
    }

    fn codes(list: &ModList) -> Vec<&str> {
        list.entries()
            .iter()
            .map(|e| e.info.code.as_str())
            .collect()
    }

    #[test]
    fn applies_edits() {
        let mut mods = list(&["b", "c", "a"]);

        mods.apply(ListEdit::Toggle(0));
        assert_eq!(mods.enabled_count(), 2);
        assert_eq!(mods.active_codes(), ["c", "a"]);

        mods.apply(ListEdit::SetChecked {
            indices: vec![0, 1, 99],
            checked: false,
        });
        assert_eq!(mods.active_codes(), ["a"]);

        mods.apply(ListEdit::Move {
            from: 2,
            insert_before: 0,
        });
        assert_eq!(codes(&mods), ["a", "b", "c"]);

        mods.apply(ListEdit::Move {
            from: 0,
            insert_before: 3,
        });
        mods.apply(ListEdit::SortByName);
        assert_eq!(codes(&mods), ["a", "b", "c"]);
    }

    #[test]
    fn filters_by_search() {
        let mods = list(&["tkuc", "bcbi"]);
        assert_eq!(mods.visible_indices("  TK "), [0]);
        assert_eq!(mods.visible_indices(""), [0, 1]);
    }

    #[test]
    fn missing_dirs_are_empty_not_errors() {
        let (mods, report) =
            ModList::scan(&[PathBuf::from("Z:/definitely/missing/mod")], &[], true);
        assert!(mods.is_empty());
        assert!(mods.scan_error().is_none());
        assert_eq!(report.dirs.len(), 1);

        let (none, _) = ModList::scan(&[], &[], true);
        assert!(none.scan_error().is_some());
    }

    #[test]
    fn order_survives_save_load_and_rescan() {
        let root = std::env::temp_dir().join(format!("nemesis_egui_order_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("Nemesis_Engine/mod");
        for code in ["a", "b", "c"] {
            std::fs::create_dir_all(dir.join(code)).unwrap();
            std::fs::write(dir.join(code).join("info.ini"), format!("name={code}")).unwrap();
        }
        let dirs = [dir];

        let (mut mods, _) = ModList::scan(&dirs, &[], true);
        mods.apply(ListEdit::Move {
            from: 2,
            insert_before: 0,
        });
        mods.apply(ListEdit::Toggle(1));

        let config = crate::config::AppConfig {
            mod_order: mods.saved_order(&[]),
            ..crate::config::AppConfig::default()
        };
        let path = root.join("settings.json");
        config.save_to(&path).unwrap();
        let loaded = crate::config::AppConfig::load_from(&path).unwrap();
        let (rescanned, _) = ModList::scan(&dirs, &loaded.mod_order, true);
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(codes(&rescanned), ["c", "a", "b"]);
        assert_eq!(rescanned.active_codes(), ["c", "b"]);
    }
}
