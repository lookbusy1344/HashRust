use std::ffi::OsStr;
use std::io::{self, BufRead};
use std::path::{Component, Path, PathBuf};

use crate::cli::config::ConfigSettings;

const GLOB_WILDCARDS: [char; 4] = ['*', '?', '[', ']'];
const CARRIAGE_RETURN: u8 = b'\r';
const NEWLINE: u8 = b'\n';

/// Fails when no paths were given and stdin is a terminal, which would wait for typed input.
pub fn check_path_source(config: &ConfigSettings, stdin_is_terminal: bool) -> anyhow::Result<()> {
    if config.supplied_paths.is_empty() && stdin_is_terminal {
        anyhow::bail!("No files specified. Pass file paths or globs, or pipe paths on stdin");
    }
    Ok(())
}

/// Paths to hash, and whether collecting them hit an error already reported on stderr.
#[derive(Debug, Default)]
pub struct InputPaths {
    pub paths: Vec<PathBuf>,
    pub had_error: bool,
}

pub fn get_required_filenames(config: &ConfigSettings) -> InputPaths {
    let InputPaths {
        mut paths,
        had_error,
    } = if config.supplied_paths.is_empty() {
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

    InputPaths { paths, had_error }
}

/// Path without `.` components, so `./a.txt` and `a.txt` compare equal.
fn dedup_key(path: &Path) -> PathBuf {
    path.components()
        .filter(|c| *c != Component::CurDir)
        .collect()
}

/// Reads one path per line. Stops at the first read error, which can repeat forever.
fn get_paths_from_stdin(config: &ConfigSettings) -> InputPaths {
    let mut paths = Vec::new();
    for line in io::stdin().lock().split(NEWLINE) {
        match line {
            Ok(line) => {
                let path = stdin_line_to_path(line);
                if !path.as_os_str().is_empty() && is_hashable(&path, config.debug_mode) {
                    paths.push(path);
                }
            }
            Err(e) => {
                eprintln!("Error reading from stdin: {e}");
                return InputPaths {
                    paths,
                    had_error: true,
                };
            }
        }
    }
    InputPaths {
        paths,
        had_error: false,
    }
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

fn get_paths_matching_glob(config: &ConfigSettings) -> InputPaths {
    let glob_settings = glob::MatchOptions {
        case_sensitive: config.case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    config
        .supplied_paths
        .iter()
        .map(|pattern| expand_pattern(pattern, glob_settings, config.debug_mode))
        .fold(InputPaths::default(), |mut acc, expanded| {
            acc.paths.extend(expanded.paths);
            acc.had_error |= expanded.had_error;
            acc
        })
}

/// Expands one command-line argument into file paths.
///
/// An existing path is taken literally, even when its name holds glob characters,
/// and kept only if [`is_hashable`]. Glob matching needs UTF-8, so other arguments
/// are taken literally. A glob that cannot read a directory, or matches no file,
/// is reported and sets `had_error`.
fn expand_pattern(pattern: &OsStr, options: glob::MatchOptions, debug_mode: bool) -> InputPaths {
    let path = Path::new(pattern);
    let literal = || InputPaths {
        paths: vec![path.to_path_buf()],
        had_error: false,
    };

    if path.exists() {
        return if is_hashable(path, debug_mode) {
            literal()
        } else {
            InputPaths::default()
        };
    }

    let Some(glob_pattern) = pattern.to_str().filter(|p| p.contains(GLOB_WILDCARDS)) else {
        return literal();
    };

    // Invalid glob pattern: keep it as a literal so the missing file is reported
    let Ok(entries) = glob::glob_with(glob_pattern, options) else {
        return literal();
    };

    let mut expanded = InputPaths::default();
    for entry in entries {
        match entry {
            Ok(path) if path.is_file() => expanded.paths.push(path),
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading '{}': {}", e.path().display(), e.error());
                expanded.had_error = true;
            }
        }
    }
    if expanded.paths.is_empty() {
        eprintln!("No files match: {glob_pattern}");
        expanded.had_error = true;
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{HashAlgorithm, OutputEncoding};

    fn config_with_paths(supplied_paths: Vec<std::ffi::OsString>) -> ConfigSettings {
        ConfigSettings {
            debug_mode: false,
            exclude_fn: false,
            single_thread: false,
            case_sensitive: false,
            no_progress: false,
            algorithm: HashAlgorithm::SHA3_256,
            encoding: OutputEncoding::Hex,
            limit_num: None,
            supplied_paths,
        }
    }

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

    #[test]
    fn test_no_paths_with_terminal_stdin_is_error() {
        let err = check_path_source(&config_with_paths(Vec::new()), true).unwrap_err();
        assert!(err.to_string().contains("No files"), "got: {err}");
    }

    #[test]
    fn test_no_paths_with_piped_stdin_is_ok() {
        assert!(check_path_source(&config_with_paths(Vec::new()), false).is_ok());
    }

    #[test]
    fn test_paths_with_terminal_stdin_is_ok() {
        assert!(check_path_source(&config_with_paths(vec!["a.txt".into()]), true).is_ok());
    }
}
