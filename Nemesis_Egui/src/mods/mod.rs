//! Nemesis mod patches: discovery and merge order.
//!
//! A Nemesis mod is a sub directory of `<engine dir>/mods` that contains an `info.ini`
//! file. The folder name (lower-cased, without extension) is the *mod code* that the
//! engine expects after `-m`.
//!
//! * [`scan`]: reading `info.ini` files and listing installed mods;
//! * [`order`]: restoring, saving and applying the user's mod order;
//! * [`list`]: the editable list shown in the mod table.

mod list;
mod order;
mod scan;

use std::path::PathBuf;

pub use list::{ListEdit, ModList};
pub use order::{active_mod_codes, apply_saved_order, move_item, to_saved_order};
pub use scan::scan_mods;

/// Metadata of a single mod read from its `info.ini`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModInfo {
    /// Lower-case mod code passed to the engine.
    pub code: String,
    /// Display name (`name=`). Falls back to the mod code when empty.
    pub name: String,
    /// Author (`author=`).
    pub author: String,
    /// Web site (`site=`).
    pub site: String,
    /// Reference file used for automatic activation (`auto=`).
    pub auto_ref: String,
    /// Directory containing the mod.
    pub dir: PathBuf,
}

/// A mod row in the GUI: its metadata plus whether it is enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModEntry {
    /// Metadata read from disk.
    pub info: ModInfo,
    /// Whether the mod will be passed to the engine.
    pub checked: bool,
}

impl ModEntry {
    /// Returns `true` when name, code, author or site contains `needle`.
    ///
    /// `needle` must already be lower-case; an empty needle matches everything.
    pub fn matches(&self, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }

        let info = &self.info;
        [&info.name, &info.code, &info.author, &info.site]
            .iter()
            .any(|field| field.to_lowercase().contains(needle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_any_field_case_insensitively() {
        let entry = ModEntry {
            info: ModInfo {
                code: "tkuc".into(),
                name: "Ultimate Combat".into(),
                author: "作者".into(),
                ..ModInfo::default()
            },
            checked: true,
        };

        assert!(entry.matches(""));
        assert!(entry.matches("combat"));
        assert!(entry.matches("tkuc"));
        assert!(entry.matches("作者"));
        assert!(!entry.matches("dodge"));
    }
}
