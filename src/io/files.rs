use std::io::{self, BufRead};
use std::path::Path;

use anyhow::Result;

use crate::cli::config::ConfigSettings;

pub fn file_exists(path: impl AsRef<Path>) -> bool {
    let path_ref = path.as_ref();
    path_ref.exists() && path_ref.is_file()
}

pub fn get_required_filenames(config: &ConfigSettings) -> Result<Vec<String>> {
    let mut paths = if config.supplied_paths.is_empty() {
        get_paths_from_stdin(config)?
    } else {
        get_paths_matching_glob(config)?
    };

    if let Some(limit) = config.limit_num {
        paths.truncate(limit);
    }

    Ok(paths)
}

fn get_paths_from_stdin(config: &ConfigSettings) -> Result<Vec<String>> {
    let stdin = io::stdin();
    let lines = stdin.lock().lines().collect::<Result<Vec<String>, _>>()?;

    Ok(lines
        .into_iter()
        .filter(|line| {
            let is_file = file_exists(line);
            if !is_file && config.debug_mode {
                eprintln!("Not a file: {line}");
            }
            is_file
        })
        .collect())
}

fn get_paths_matching_glob(config: &ConfigSettings) -> Result<Vec<String>> {
    let glob_settings = glob::MatchOptions {
        case_sensitive: config.case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let mut result = Vec::new();

    for pattern in &config.supplied_paths {
        let glob_matches: Vec<_> = glob::glob_with(pattern, glob_settings)?
            .filter_map(|entry| match entry {
                Ok(path) if path.is_file() => Some(path.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();

        if glob_matches.is_empty() {
            if file_exists(pattern) {
                result.push(pattern.clone());
            } else {
                if !pattern.contains(['*', '?', '[', ']']) {
                    let path = std::path::Path::new(pattern);
                    if path.exists() && path.is_dir() {
                        if config.debug_mode {
                            eprintln!("Ignoring directory: {pattern}");
                        }
                    } else {
                        return Err(anyhow::anyhow!("File not found: {}", pattern));
                    }
                }
            }
        } else {
            result.extend(glob_matches);
        }
    }

    Ok(result)
}