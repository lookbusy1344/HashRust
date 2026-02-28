use std::io::{self, BufRead};
use std::path::Path;

use anyhow::Result;

use crate::cli::config::ConfigSettings;

pub fn get_required_filenames(config: &ConfigSettings) -> Result<Vec<String>> {
    let mut paths = if config.supplied_paths.is_empty() {
        get_paths_from_stdin(config)
    } else {
        get_paths_matching_glob(config)?
    };

    if let Some(limit) = config.limit_num {
        paths.truncate(limit);
    }

    Ok(paths)
}

fn get_paths_from_stdin(config: &ConfigSettings) -> Vec<String> {
    let stdin = io::stdin();
    stdin
        .lock()
        .lines()
        .filter_map(|line_result| match line_result {
            Ok(line) => {
                let is_file = Path::new(&line).is_file();
                if !is_file && config.debug_mode {
                    eprintln!("Not a file: {line}");
                }
                is_file.then_some(line)
            }
            Err(e) => {
                if config.debug_mode {
                    eprintln!("Skipping unreadable stdin line: {e}");
                }
                None
            }
        })
        .collect()
}

fn get_paths_matching_glob(config: &ConfigSettings) -> Result<Vec<String>> {
    let glob_settings = glob::MatchOptions {
        case_sensitive: config.case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let mut result = Vec::new();

    for pattern in &config.supplied_paths {
        let glob_result = glob::glob_with(pattern, glob_settings);

        match glob_result {
            Ok(paths) => {
                let glob_matches: Vec<_> = paths
                    .filter_map(|entry| match entry {
                        Ok(path) if path.is_file() => Some(path.to_string_lossy().into_owned()),
                        _ => None,
                    })
                    .collect();

                if glob_matches.is_empty() {
                    if Path::new(pattern).is_file() {
                        result.push(pattern.clone());
                    } else if !pattern.contains(['*', '?', '[', ']']) {
                        let path = std::path::Path::new(pattern);
                        if path.exists() && path.is_dir() {
                            if config.debug_mode {
                                eprintln!("Ignoring directory: {pattern}");
                            }
                        } else {
                            return Err(anyhow::anyhow!("File not found: {pattern}"));
                        }
                    }
                } else {
                    result.extend(glob_matches);
                }
            }
            Err(_) => {
                // If glob fails (e.g. invalid pattern), treat as literal file
                if Path::new(pattern).is_file() {
                    result.push(pattern.clone());
                } else {
                    return Err(anyhow::anyhow!("File not found or invalid glob: {pattern}"));
                }
            }
        }
    }

    Ok(result)
}
