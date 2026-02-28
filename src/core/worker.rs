use std::fmt::Display;
use std::time::{Duration, Instant};

use anyhow::Result;
use rayon::prelude::*;

use crate::cli::config::ConfigSettings;
use crate::core::types::BasicHash;
use crate::hash::algorithms::call_hasher;
use crate::io::files::get_required_filenames;
use crate::progress::ProgressCoordinator;

pub fn worker_func(config: &ConfigSettings) -> Result<()> {
    if config.debug_mode {
        show_initial_info(config);
    }

    let paths = get_required_filenames(config)?;

    if paths.is_empty() {
        if config.debug_mode {
            eprintln!("No files found");
        }
        return Ok(());
    }

    if config.debug_mode {
        eprintln!("Files to hash: {paths:?}");
    }

    let had_error = if config.single_thread || paths.len() == 1 {
        file_hashes_st(config, &paths)
    } else {
        file_hashes_mt(config, &paths)
    };

    if had_error {
        std::process::exit(1);
    }

    Ok(())
}

fn show_initial_info(config: &ConfigSettings) {
    crate::cli::args::show_help(false, &mut std::io::stderr());
    eprintln!();
    eprintln!("Config: {config:?}");
    if config.supplied_paths.is_empty() {
        eprintln!("No path specified, reading from stdin");
    } else {
        eprintln!(
            "Paths: {} file path(s) supplied",
            config.supplied_paths.len()
        );
    }
}

fn file_hashes_st<S>(config: &ConfigSettings, paths: &[S]) -> bool
where
    S: AsRef<str> + Display + Send + Sync,
{
    if config.debug_mode {
        eprintln!("Single-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    let coordinator = if config.no_progress {
        None
    } else {
        Some(ProgressCoordinator::new())
    };

    let mut had_error = false;

    for pathstr in paths {
        let file_hash =
            hash_with_progress(config, AsRef::<str>::as_ref(pathstr), coordinator.as_ref());

        match file_hash {
            Ok(basic_hash) => {
                if config.exclude_fn {
                    println!("{basic_hash}");
                } else {
                    println!("{basic_hash} {pathstr}");
                }
            }
            Err(e) => {
                eprintln!("File error for '{pathstr}': {e}");
                had_error = true;
            }
        }
    }

    had_error
}

fn file_hashes_mt<S>(config: &ConfigSettings, paths: &[S]) -> bool
where
    S: AsRef<str> + Sync + Display,
{
    if config.debug_mode {
        eprintln!("Multi-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    let coordinator = if config.no_progress {
        None
    } else {
        Some(ProgressCoordinator::new())
    };

    let overall_progress = coordinator
        .as_ref()
        .and_then(|c| c.create_overall_progress(paths.len()));

    let results: Vec<_> = paths
        .par_iter()
        .map(|pathstr| {
            let file_hash = hash_with_progress(
                config,
                AsRef::<str>::as_ref(pathstr),
                if overall_progress.is_some() {
                    None
                } else {
                    coordinator.as_ref()
                },
            );

            if let Some(ref pb) = overall_progress {
                pb.inc(1);
            }

            (pathstr, file_hash)
        })
        .collect();

    if let Some(pb) = overall_progress {
        pb.finish_with_message("Complete!");
    }

    let mut had_error = false;

    for (pathstr, file_hash) in results {
        match file_hash {
            Ok(basic_hash) => {
                if config.exclude_fn {
                    println!("{basic_hash}");
                } else {
                    println!("{basic_hash} {pathstr}");
                }
            }

            Err(e) => {
                eprintln!("File error for '{pathstr}': {e}");
                had_error = true;
            }
        }
    }

    had_error
}

fn hash_with_progress(
    config: &ConfigSettings,
    pathstr: &str,
    coordinator: Option<&ProgressCoordinator>,
) -> Result<BasicHash> {
    // Create spinner if progress is enabled
    // With MultiProgress, fast operations will just flash briefly which is acceptable
    let spinner = coordinator.map(|coord| coord.create_spinner(pathstr));

    let start_time = Instant::now();
    let result = call_hasher(config.algorithm, config.encoding, pathstr);
    let elapsed = start_time.elapsed();

    // Clean up spinner
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    if config.debug_mode
        && elapsed >= Duration::from_millis(crate::progress::PROGRESS_THRESHOLD_MILLIS)
    {
        eprintln!(
            "File '{}' took {:.2}s to hash",
            pathstr,
            elapsed.as_secs_f64()
        );
    }

    result
}
