//! Restoring, saving and applying the user's mod order.

use std::collections::{HashMap, HashSet};

use super::{ModEntry, ModInfo};
use crate::config::ModOrderEntry;

/// Combines freshly scanned mods with the saved order.
///
/// Mods present in `saved` keep their saved position and checked state. Mods that are
/// not in `saved` are appended at the bottom with `checked = enable_new`. Saved entries
/// whose mod no longer exists are dropped.
pub fn apply_saved_order(
    mods: Vec<ModInfo>,
    saved: &[ModOrderEntry],
    enable_new: bool,
) -> Vec<ModEntry> {
    let rank: HashMap<&str, (usize, bool)> = saved
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.code.as_str(), (index, entry.checked)))
        .collect();

    let mut known = Vec::new();
    let mut fresh = Vec::new();

    for info in mods {
        match rank.get(info.code.as_str()) {
            Some(&(index, checked)) => known.push((index, ModEntry { info, checked })),
            None => fresh.push(ModEntry {
                info,
                checked: enable_new,
            }),
        }
    }

    known.sort_by_key(|(index, _)| *index);
    known
        .into_iter()
        .map(|(_, entry)| entry)
        .chain(fresh)
        .collect()
}

/// Converts the current list back into its persisted form.
///
/// Entries of `previous` whose mod is not in the current list (for example because
/// the engine path was temporarily wrong) are kept at the end, so their state is not
/// lost.
pub fn to_saved_order(entries: &[ModEntry], previous: &[ModOrderEntry]) -> Vec<ModOrderEntry> {
    let current: HashSet<&str> = entries
        .iter()
        .map(|entry| entry.info.code.as_str())
        .collect();

    entries
        .iter()
        .map(|entry| ModOrderEntry {
            code: entry.info.code.clone(),
            checked: entry.checked,
        })
        .chain(
            previous
                .iter()
                .filter(|entry| !current.contains(entry.code.as_str()))
                .cloned(),
        )
        .collect()
}

/// Returns the mod codes to pass after `-m`.
///
/// The list is merged from the top row to the bottom row: the engine treats the
/// left-most code as the lowest priority and the right-most as the highest, so a
/// mod placed lower in the list is applied later and wins conflicts. This is the
/// same mapping the original Nemesis launcher and D-Merge use.
pub fn active_mod_codes(entries: &[ModEntry]) -> Vec<String> {
    let mut seen = HashSet::new();

    entries
        .iter()
        .filter(|entry| entry.checked)
        .map(|entry| entry.info.code.clone())
        .filter(|code| seen.insert(code.clone()))
        .collect()
}

/// Moves the element at `from` so that it ends up just before the element that was
/// at `insert_before` (an index in the *original* list, `0..=len`).
///
/// Out-of-range `from` values are ignored.
pub fn move_item<T>(items: &mut Vec<T>, from: usize, insert_before: usize) {
    if from >= items.len() {
        return;
    }

    let insert_before = insert_before.min(items.len());
    let target = if insert_before > from {
        insert_before - 1
    } else {
        insert_before
    };

    if target != from {
        let item = items.remove(from);
        items.insert(target, item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(code: &str) -> ModInfo {
        ModInfo {
            code: code.into(),
            name: code.into(),
            ..ModInfo::default()
        }
    }

    fn saved(code: &str, checked: bool) -> ModOrderEntry {
        ModOrderEntry {
            code: code.into(),
            checked,
        }
    }

    #[test]
    fn saved_order_is_restored_and_new_mods_are_appended() {
        let saved = [saved("c", false), saved("gone", true), saved("a", true)];
        let entries = apply_saved_order(vec![info("a"), info("b"), info("c")], &saved, true);

        let codes: Vec<_> = entries
            .iter()
            .map(|e| (e.info.code.as_str(), e.checked))
            .collect();
        assert_eq!(codes, [("c", false), ("a", true), ("b", true)]);
    }

    #[test]
    fn saving_keeps_entries_of_missing_mods() {
        let previous = [saved("missing", false), saved("a", false)];
        let entries = vec![ModEntry {
            info: info("a"),
            checked: true,
        }];

        assert_eq!(
            to_saved_order(&entries, &previous),
            [saved("a", true), saved("missing", false)]
        );
    }

    #[test]
    fn active_codes_follow_list_order_top_to_bottom() {
        let entries = vec![
            ModEntry {
                info: info("first"),
                checked: true,
            },
            ModEntry {
                info: info("skip"),
                checked: false,
            },
            ModEntry {
                info: info("last"),
                checked: true,
            },
        ];

        assert_eq!(active_mod_codes(&entries), ["first", "last"]);
    }

    #[test]
    fn move_item_handles_both_directions() {
        let mut v = vec![0, 1, 2, 3];
        move_item(&mut v, 0, 3);
        assert_eq!(v, [1, 2, 0, 3]);

        let mut v = vec![0, 1, 2, 3];
        move_item(&mut v, 3, 0);
        assert_eq!(v, [3, 0, 1, 2]);

        let mut v = vec![0, 1, 2, 3];
        move_item(&mut v, 1, 4);
        assert_eq!(v, [0, 2, 3, 1]);

        let mut v = vec![0, 1, 2];
        move_item(&mut v, 1, 1);
        move_item(&mut v, 1, 2);
        move_item(&mut v, 9, 0);
        assert_eq!(v, [0, 1, 2]);
    }
}
