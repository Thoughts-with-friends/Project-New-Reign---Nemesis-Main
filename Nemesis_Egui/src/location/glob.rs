//! A small directory glob supporting `*` and `?` inside path components,
//! e.g. `D:\MO2\mods\*`.
//!
//! The expansion works level by level and collects every `read_dir` into a
//! `Vec` before descending. MO2's USVFS guards file-system calls with a
//! re-entrant per-thread lock, and nested directory iteration on one thread can
//! corrupt it (see D-Merge's `jwalk_glob` notes), so no two directory handles
//! are ever open at the same time here.

use std::fs;
use std::path::{Path, PathBuf};

/// Returns `true` when `pattern` contains a wildcard (`*` or `?`).
pub fn has_wildcard(pattern: &str) -> bool {
    pattern.contains(['*', '?'])
}

/// Expands `pattern` into the directories it matches, sorted by name within each
/// level. Components without wildcards are taken as-is (not checked for
/// existence); unreadable directories simply produce no matches.
pub fn expand_dirs(pattern: &str) -> Vec<PathBuf> {
    let mut bases = vec![PathBuf::new()];

    for component in Path::new(pattern.trim()).components() {
        let text = component.as_os_str().to_string_lossy();

        if !has_wildcard(&text) {
            for base in &mut bases {
                base.push(component);
            }
            continue;
        }

        let mut next = Vec::new();
        for base in &bases {
            next.extend(
                list_subdirs(base)
                    .into_iter()
                    .filter(|name| wildcard_match(&text, name))
                    .map(|name| base.join(name)),
            );
        }
        bases = next;
    }

    bases
}

/// Names of the sub directories of `dir`, sorted. The directory handle is closed
/// before this returns.
fn list_subdirs(dir: &Path) -> Vec<String> {
    let dir = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };

    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|ty| ty.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Case-insensitive wildcard match of a single path component:
/// `*` matches any run of characters, `?` exactly one.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let (mut p, mut t) = (0, 0);
    // Position of the last `*` and the text index it is currently matched up to.
    let mut star: Option<(usize, usize)> = None;

    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if let Some((star_p, star_t)) = star {
            p = star_p + 1;
            t = star_t + 1;
            star = Some((star_p, star_t + 1));
        } else {
            return false;
        }
    }

    pattern[p..].iter().all(|&c| c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_wildcards_case_insensitively() {
        assert!(wildcard_match("*", "anything"));
        assert!(wildcard_match("*", ""));
        assert!(wildcard_match("Nemesis*", "nemesis_engine"));
        assert!(wildcard_match("*engine", "Nemesis_Engine"));
        assert!(wildcard_match("d?ta", "Data"));
        assert!(wildcard_match("*攻撃*", "Attack 攻撃モーション"));
        assert!(!wildcard_match("d?ta", "dta"));
        assert!(!wildcard_match("*.esp", "mod.esm"));
    }

    #[test]
    fn expands_only_matching_directories() {
        let root = std::env::temp_dir().join(format!("nemesis_egui_glob_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["mods/A mod", "mods/B 日本語", "mods/skip/inner"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(root.join("mods/file.txt"), "").unwrap();

        let all = expand_dirs(&format!("{}/mods/*", root.display()));
        let some = expand_dirs(&format!("{}/mods/?ki*/*", root.display()));
        let _ = fs::remove_dir_all(&root);

        let names: Vec<_> = all
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["A mod", "B 日本語", "skip"]);
        assert_eq!(some.len(), 1);
        assert!(some[0].ends_with("skip/inner"));
    }

    #[test]
    fn literal_patterns_are_returned_as_is() {
        assert_eq!(
            expand_dirs("Z:/no/such/dir"),
            [PathBuf::from("Z:/no/such/dir")]
        );
        assert!(expand_dirs("Z:/no/such/*").is_empty());
    }
}
