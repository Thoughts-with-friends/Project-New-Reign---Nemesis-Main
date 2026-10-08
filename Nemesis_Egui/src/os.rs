//! Interaction with the operating system: opening files, folders and web links.

use std::path::Path;
use std::process::Command;

/// Opens a file or directory with the platform's default application.
///
/// # Errors
/// Returns a message when the path does not exist or no handler could be started.
pub fn open_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    #[cfg(windows)]
    let mut command = Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");

    command
        .arg(path)
        .spawn()
        .map(drop)
        .map_err(|err| format!("Failed to open {}: {err}", path.display()))
}

/// Turns an `info.ini` `site=` value into a URL, adding `https://` to bare domains
/// such as `www.nexusmods.com/...`.
pub fn web_url(site: &str) -> String {
    let site = site.trim();
    if site.contains("://") {
        site.to_owned()
    } else {
        format!("https://{site}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bare_domains() {
        assert_eq!(
            web_url("www.patreon.com/Distar"),
            "https://www.patreon.com/Distar"
        );
        assert_eq!(
            web_url(" https://www.nexusmods.com "),
            "https://www.nexusmods.com"
        );
    }

    #[test]
    fn missing_path_is_an_error() {
        assert!(open_path(Path::new("Z:/definitely/missing/file.txt")).is_err());
    }
}
