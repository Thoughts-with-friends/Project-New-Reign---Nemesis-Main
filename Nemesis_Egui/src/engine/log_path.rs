//! Location of the engine's `log.txt`.

use std::path::{Path, PathBuf};

/// Returns where the engine writes `log.txt`.
///
/// The engine writes `<engine dir>/log.txt`, but when an output directory differs
/// from the data directory and the engine lives inside the data directory, the
/// path is redirected into the output directory (`NemesisInfo::PatchOutputPath`).
pub fn log_file_path(engine_path: &Path, data_dir: &str, output_dir: &str) -> PathBuf {
    let log = engine_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("log.txt");
    let data_dir = data_dir.trim();
    let output_dir = output_dir.trim();

    if output_dir.is_empty() || data_dir.is_empty() {
        return log;
    }

    let absolute = |path: &Path| std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let data = absolute(Path::new(data_dir));
    let output = absolute(Path::new(output_dir));

    if data == output {
        return log;
    }

    let log_abs = absolute(&log);
    let log_str = log_abs.to_string_lossy();
    let data_str = data.to_string_lossy();

    // The engine compares case-insensitively and skips one separator after the prefix.
    if log_str.len() <= data_str.len() + 1
        || !log_str.is_char_boundary(data_str.len())
        || !log_str[..data_str.len()].eq_ignore_ascii_case(&data_str)
    {
        return log;
    }

    output.join(&log_str[data_str.len() + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_path_follows_engine_redirect_rule() {
        let engine = Path::new("C:/Game/Data/nemesis_engine/Nemesis_Engine.exe");

        assert_eq!(
            log_file_path(engine, "", ""),
            Path::new("C:/Game/Data/nemesis_engine/log.txt")
        );
        assert_eq!(
            log_file_path(engine, "C:/Game/Data", "C:/Game/Data"),
            Path::new("C:/Game/Data/nemesis_engine/log.txt")
        );

        let redirected = log_file_path(engine, "C:/Game/Data", "D:/Out");
        let expected = std::path::absolute("D:/Out")
            .unwrap()
            .join("nemesis_engine")
            .join("log.txt");
        assert_eq!(redirected, expected);
    }
}
