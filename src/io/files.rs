use std::ffi::OsStr;
use std::io::{self, BufRead};
use std::path::{Component, Path, PathBuf};

use crate::cli::config::ConfigSettings;

const GLOB_WILDCARDS: [char; 4] = ['*', '?', '[', ']'];
const CARRIAGE_RETURN: u8 = b'\r';
const NEWLINE: u8 = b'\n';

pub fn get_required_filenames(config: &ConfigSettings) -> Vec<PathBuf> {
    let mut paths = if config.supplied_paths.is_empty() {
        get_paths_from_stdin(config)
    } else {
        get_paths_matching_glob(config)
    };

    // De-duplicate while preserving first-seen order.
    // Duplicates arise when overlapping glob patterns or repeated literal
    // arguments produce the same path more than once.
    {
        let mut seen = std::collections::HashSet::new();
        paths.retain(|p| seen.insert(dedup_key(p)));
    }

    if let Some(limit) = config.limit_num {
        paths.truncate(limit);
    }

    paths
}

/// Path without `.` components, so `./a.txt` and `a.txt` compare equal.
fn dedup_key(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| *c != Component::CurDir)
        .collect()
}

fn get_paths_from_stdin(config: &ConfigSettings) -> Vec<PathBuf> {
    let stdin = io::stdin();
    stdin
        .lock()
        .split(NEWLINE)
        .filter_map(|line_result| match line_result {
            Ok(line) => {
                let path = stdin_line_to_path(line);
                (!path.as_os_str().is_empty() && is_hashable(&path, config.debug_mode))
                    .then_some(path)
            }
            Err(e) => {
                // Always report stdin I/O errors so the user isn't silently left
                // with fewer files hashed than expected.
                eprintln!("Error reading from stdin: {e}");
                None
            }
        })
        .collect()
}

/// Converts one stdin line (without its newline) to a path, dropping a trailing `\r`.
pub fn stdin_line_to_path(mut line: Vec<u8>) -> PathBuf {
    if line.last() == Some(&CARRIAGE_RETURN) {
        line.pop();
    }
    bytes_to_path(line)
}

#[cfg(unix)]
fn bytes_to_path(bytes: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(bytes))
}

#[cfg(not(unix))]
fn bytes_to_path(bytes: Vec<u8>) -> PathBuf {
    let text = String::from_utf8(bytes)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());
    PathBuf::from(text)
}

/// Regular files and missing paths are hashed; a missing path is reported then.
/// Directories, FIFOs and devices are skipped, because reading them can block.
fn is_hashable(path: &Path, debug_mode: bool) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => true,
        Ok(_) => {
            if debug_mode {
                eprintln!("Ignoring non-regular file: {}", path.display());
            }
            false
        }
        Err(_) => true,
    }
}

fn get_paths_matching_glob(config: &ConfigSettings) -> Vec<PathBuf> {
    let glob_settings = glob::MatchOptions {
        case_sensitive: config.case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    config
        .supplied_paths
        .iter()
        .flat_map(|pattern| expand_pattern(pattern, glob_settings, config.debug_mode))
        .collect()
}

/// Expands one command-line argument into file paths.
///
/// An existing path is taken literally, even when its name holds glob characters,
/// and kept only if [`is_hashable`]. Glob matching needs UTF-8, so other arguments
/// are taken literally.
fn expand_pattern(pattern: &OsStr, options: glob::MatchOptions, debug_mode: bool) -> Vec<PathBuf> {
    let path = Path::new(pattern);

    if path.exists() {
        return if is_hashable(path, debug_mode) {
            vec![path.to_path_buf()]
        } else {
            Vec::new()
        };
    }

    let Some(glob_pattern) = pattern.to_str().filter(|p| p.contains(GLOB_WILDCARDS)) else {
        return vec![path.to_path_buf()];
    };

    match glob::glob_with(glob_pattern, options) {
        Ok(entries) => {
            let matches: Vec<_> = entries
                .filter_map(|entry| match entry {
                    Ok(path) if path.is_file() => Some(path),
                    Ok(_) => None,
                    Err(e) => {
                        eprintln!("Error reading '{}': {}", e.path().display(), e.error());
                        None
                    }
                })
                .collect();
            if matches.is_empty() && debug_mode {
                eprintln!("No matches: {glob_pattern}");
            }
            matches
        }
        // Invalid glob pattern: keep it as a literal so the missing file is reported
        Err(_) => vec![path.to_path_buf()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn test_fifo_is_not_hashable() {
        let dir = tempfile::TempDir::new().unwrap();
        let fifo = dir.path().join("fifo");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap();
        assert!(status.success());

        assert!(!is_hashable(&fifo, false));
    }

    #[test]
    #[cfg(unix)]
    fn test_character_device_is_not_hashable() {
        assert!(!is_hashable(Path::new("/dev/null"), false));
    }

    #[test]
    fn test_directory_is_not_hashable() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(!is_hashable(dir.path(), false));
    }

    #[test]
    fn test_regular_file_is_hashable() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(is_hashable(file.path(), false));
    }

    #[test]
    fn test_missing_path_is_hashable_so_it_is_reported() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(is_hashable(&dir.path().join("missing.txt"), false));
    }
}
