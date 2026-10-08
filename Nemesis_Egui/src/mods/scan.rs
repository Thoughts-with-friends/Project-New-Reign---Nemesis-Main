//! Reading `info.ini` files and listing installed mods.

use std::fs;
use std::io;
use std::path::Path;

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

/// Scans `mods_dir` for sub directories containing `info.ini`.
///
/// The result is sorted by mod code so that a fresh list is deterministic.
/// Unreadable mods are skipped rather than aborting the whole scan.
///
/// # Errors
/// Returns an error when `mods_dir` itself cannot be listed.
pub fn scan_mods(mods_dir: &Path) -> io::Result<Vec<ModInfo>> {
    let mut mods: Vec<ModInfo> = fs::read_dir(mods_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|ty| ty.is_dir()))
        .map(|entry| entry.path())
        .filter(|dir| dir.join("info.ini").is_file())
        .filter_map(|dir| read_mod(&dir).ok())
        .collect();

    mods.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(mods)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nemesis_egui_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
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
    fn scans_only_directories_with_info_ini() {
        let root = temp_dir("scan");
        fs::create_dir_all(root.join("TKUC")).unwrap();
        fs::write(root.join("TKUC/info.ini"), "\u{feff}name=Ultimate Combat\n").unwrap();
        fs::create_dir_all(root.join("bcbi")).unwrap();
        fs::write(root.join("bcbi/info.ini"), "name=日本語の名前\nsite=x\n").unwrap();
        fs::create_dir_all(root.join("empty")).unwrap();
        fs::write(root.join("stray.txt"), "").unwrap();

        let mods = scan_mods(&root).unwrap();
        let _ = fs::remove_dir_all(&root);

        let codes: Vec<_> = mods.iter().map(|m| m.code.as_str()).collect();
        assert_eq!(codes, ["bcbi", "tkuc"]);
        assert_eq!(mods[0].name, "日本語の名前");
        assert_eq!(mods[1].name, "Ultimate Combat");
    }

    #[test]
    fn mod_code_matches_engine_stem() {
        assert_eq!(mod_code_of(Path::new("mods/My.Mod")).as_deref(), Some("my"));
        assert_eq!(mod_code_of(Path::new("mods/ABC")).as_deref(), Some("abc"));
    }
}
