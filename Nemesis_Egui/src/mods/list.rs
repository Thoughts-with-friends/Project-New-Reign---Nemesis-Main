//! The ordered, checkable list of installed mods shown in the table.

use std::path::Path;

use super::{ModEntry, active_mod_codes, apply_saved_order, move_item, scan_mods, to_saved_order};
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
    /// Scans `mods_dir` and orders the result according to `saved`.
    ///
    /// A failed scan yields an empty list with [`ModList::scan_error`] set.
    pub fn scan(mods_dir: &Path, saved: &[ModOrderEntry], enable_new: bool) -> Self {
        match scan_mods(mods_dir) {
            Ok(found) => Self {
                entries: apply_saved_order(found, saved, enable_new),
                scan_error: None,
            },
            Err(err) => Self {
                entries: Vec::new(),
                scan_error: Some(format!(
                    "Cannot read mods directory \"{}\": {err}",
                    mods_dir.display()
                )),
            },
        }
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
    fn failed_scan_reports_error() {
        let mods = ModList::scan(Path::new("Z:/definitely/missing/mods"), &[], true);
        assert!(mods.is_empty());
        assert!(mods.scan_error().is_some());
    }
}
