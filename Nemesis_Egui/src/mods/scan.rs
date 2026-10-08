//! Reading `info.ini` files and listing installed mods.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::ModInfo;

/// Parsed `key=value` fields of an `info.ini` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InfoFields {
    /// Value of `name=`.
    pub name: String,
    /// Value of `author=`.
    pub author: String,
    /// Value of `site=`.
    pub site: String,
    /// Value of `auto=`.
    pub auto_ref: String,
}

/// Parses the contents of an `info.ini` file.
///
/// Mirrors the engine's parser: only lines starting with `name=`, `author=`, `site=`
/// or `auto=` are recognized, and the last occurrence wins. A UTF-8 BOM and
/// surrounding whitespace are ignored.
pub fn parse_info_ini(text: &str) -> InfoFields {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut fields = InfoFields::default();

    for line in text.lines().map(str::trim) {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let slot = match key {
            "name" => &mut fields.name,
            "author" => &mut fields.author,
            "site" => &mut fields.site,
            "auto" => &mut fields.auto_ref,
            _ => continue,
        };

        *slot = value.to_owned();
    }

    fields
}

/// Derives the mod code from a mod directory, matching the engine's
/// `to_lower(path.stem())`.
pub fn mod_code_of(dir: &Path) -> Option<String> {
    dir.file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
}

/// Reads a single mod directory.
///
/// # Errors
/// Returns an error when `info.ini` cannot be read or the directory has no name.
pub fn read_mod(dir: &Path) -> io::Result<ModInfo> {
    let bytes = fs::read(dir.join("info.ini"))?;
    let fields = parse_info_ini(&String::from_utf8_lossy(&bytes));
    let code = mod_code_of(dir)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "mod directory has no name"))?;

    Ok(ModInfo {
        name: if fields.name.is_empty() {
            code.clone()
        } else {
            fields.name
        },
        code,
        author: fields.author,
        site: fields.site,
        auto_ref: fields.auto_ref,
        dir: dir.to_path_buf(),
    })
}

/// Result of scanning one mod directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirReport {
    /// The scanned directory.
    pub dir: PathBuf,
    /// Number of mods found, or why the directory could not be read.
    pub result: Result<usize, String>,
}

/// Result of [`scan_dirs`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanReport {
    /// Mods found, without duplicate codes, in scan order.
    pub mods: Vec<ModInfo>,
    /// Per-directory outcome (missing directories count as `Ok(0)`).
    pub dirs: Vec<DirReport>,
    /// Codes found more than once; the first occurrence was kept.
    pub duplicates: Vec<String>,
}

/// Scans each directory in `dirs` for sub directories containing `info.ini`.
///
/// Within a directory, mods are sorted by code; a code seen in an earlier
/// directory wins. Missing directories are normal (most MO2 mods have no
/// Nemesis patch) and count as empty.
///
/// No existence pre-checks are made, because they can give false negatives inside
/// MO2's virtual file system. Each listing is collected before any file is read,
/// so only one directory handle is open at a time.
pub fn scan_dirs(dirs: &[PathBuf]) -> ScanReport {
    let mut report = ScanReport::default();
    let mut seen = HashSet::new();

    for dir in dirs {
        let result = match list_subdirs(dir) {
            Ok(candidates) => {
                let mut found: Vec<ModInfo> = candidates
                    .iter()
                    .filter_map(|candidate| read_mod(candidate).ok())
                    .collect();
                found.sort_by(|a, b| a.code.cmp(&b.code));
                let count = found.len();

                for info in found {
                    if seen.insert(info.code.clone()) {
                        report.mods.push(info);
                    } else {
                        report.duplicates.push(info.code);
                    }
                }

                Ok(count)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(err) => Err(err.to_string()),
        };

        report.dirs.push(DirReport {
            dir: dir.clone(),
            result,
        });
    }

    report
}

/// Lists the sub directories of `dir`; the handle is closed before returning.
fn list_subdirs(dir: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|ty| ty.is_dir()))
        .map(|entry| entry.path())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nemesis_egui_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_mod(dir: &Path, code: &str, ini: &str) {
        fs::create_dir_all(dir.join(code)).unwrap();
        fs::write(dir.join(code).join("info.ini"), ini).unwrap();
    }

    #[test]
    fn parses_info_ini_with_bom_and_crlf() {
        let fields = parse_info_ini(
            "\u{feff}name=攻撃モーション\r\nauthor=Someone\r\nsite=https://example.com\r\nauto=foo.hkx\r\n",
        );

        assert_eq!(fields.name, "攻撃モーション");
        assert_eq!(fields.author, "Someone");
        assert_eq!(fields.site, "https://example.com");
        assert_eq!(fields.auto_ref, "foo.hkx");
    }

    #[test]
    fn keeps_equals_signs_inside_values() {
        let fields = parse_info_ini("site=https://example.com/?a=b\n");
        assert_eq!(fields.site, "https://example.com/?a=b");
    }

    #[test]
    fn scans_both_layouts_and_skips_missing_dirs() {
        let root = temp_dir("scan");
        let mod_dir = root.join("mods/Attack/Nemesis_Engine/mod");
        let engine_mods = root.join("Data/Nemesis_Engine/mods");
        write_mod(&mod_dir, "TKUC", "\u{feff}name=Ultimate Combat\n");
        fs::create_dir_all(mod_dir.join("empty")).unwrap();
        fs::write(mod_dir.join("stray.txt"), "").unwrap();
        write_mod(&engine_mods, "bcbi", "name=日本語の名前\nsite=x\n");
        write_mod(&engine_mods, "tkuc", "name=Duplicate\n");

        let missing = root.join("mods/NoPatch/Nemesis_Engine/mod");
        let report = scan_dirs(&[mod_dir.clone(), missing, engine_mods]);
        let _ = fs::remove_dir_all(&root);

        let codes: Vec<_> = report.mods.iter().map(|m| m.code.as_str()).collect();
        assert_eq!(codes, ["tkuc", "bcbi"]);
        assert_eq!(report.mods[0].name, "Ultimate Combat");
        assert_eq!(report.mods[0].dir, mod_dir.join("TKUC"));
        assert_eq!(report.mods[1].name, "日本語の名前");
        assert_eq!(report.duplicates, ["tkuc"]);

        let counts: Vec<_> = report.dirs.iter().map(|d| d.result.clone()).collect();
        assert_eq!(counts, [Ok(1), Ok(0), Ok(2)]);
    }

    #[test]
    fn mod_code_matches_engine_stem() {
        assert_eq!(mod_code_of(Path::new("mods/My.Mod")).as_deref(), Some("my"));
        assert_eq!(mod_code_of(Path::new("mods/ABC")).as_deref(), Some("abc"));
    }
}
