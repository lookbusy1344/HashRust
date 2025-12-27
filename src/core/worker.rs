use std::fmt::Display;
use std::time::{Duration, Instant};

use anyhow::Result;
use rayon::prelude::*;

use crate::cli::config::ConfigSettings;
use crate::core::types::BasicHash;
use crate::hash::algorithms::call_hasher;
use crate::io::files::get_required_filenames;
use crate::progress::ProgressManager;

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

    if config.single_thread || paths.len() == 1 {
        file_hashes_st(config, &paths);
    } else {
        file_hashes_mt(config, &paths);
    }

    Ok(())
}

fn show_initial_info(config: &ConfigSettings) {
    crate::cli::args::show_help(false);
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

fn file_hashes_st<S>(config: &ConfigSettings, paths: &[S])
where
    S: AsRef<str> + Display + Send + Sync,
{
    if config.debug_mode {
        eprintln!("Single-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    for pathstr in paths {
        let file_hash = hash_with_progress(config, pathstr.as_ref().to_string(), false);

        match file_hash {
            Ok(basic_hash) => {
                if config.exclude_fn {
                    println!("{basic_hash}");
                } else {
                    println!("{basic_hash} {pathstr}");
                }
            }
            Err(e) => eprintln!("File error for '{pathstr}': {e}"),
        }
    }
}

fn file_hashes_mt<S>(config: &ConfigSettings, paths: &[S])
where
    S: AsRef<str> + Sync + Display,
{
    if config.debug_mode {
        eprintln!("Multi-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    let overall_progress = if config.no_progress {
        None
    } else {
        ProgressManager::create_overall_progress(paths.len(), config.debug_mode)
    };

    paths.par_iter().for_each(|pathstr| {
        let file_hash = hash_with_progress(
            config,
            pathstr.as_ref().to_string(),
            overall_progress.is_some(),
        );

        if let Some(ref pb) = overall_progress {
            pb.inc(1);
        }

        match file_hash {
            Ok(basic_hash) => {
                if config.exclude_fn {
                    println!("{basic_hash}");
                } else {
                    println!("{basic_hash} {pathstr}");
                }
            }

            Err(e) => eprintln!("File error for '{pathstr}': {e}"),
        }
    });

    if let Some(pb) = overall_progress {
        pb.finish_with_message("Complete!");
    }
}

fn hash_with_progress<S>(
    config: &ConfigSettings,
    pathstr: S,
    suppress_spinner: bool,
) -> Result<BasicHash>
where
    S: AsRef<str> + Display + Clone + Send + 'static,
{
    let pathstr_clone = pathstr.clone();

    let progress_handle = if config.no_progress || suppress_spinner {
        None
    } else {
        ProgressManager::create_file_progress(pathstr_clone.clone(), config.debug_mode)
    };

    let start_time = Instant::now();
    let result = call_hasher(config.algorithm, config.encoding, pathstr);
    let elapsed = start_time.elapsed();

    if let Some(handle) = progress_handle {
        handle.finish(config.debug_mode);
    }

    if config.debug_mode && elapsed >= Duration::from_millis(ProgressManager::threshold_millis()) {
        eprintln!(
            "File '{}' took {:.2}s to hash",
            pathstr_clone,
            elapsed.as_secs_f64()
        );
    }

    result
}
