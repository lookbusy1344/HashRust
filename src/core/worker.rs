use std::fmt::Display;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use rayon::prelude::*;

use crate::cli::config::ConfigSettings;
use crate::core::types::BasicHash;
use crate::hash::algorithms::call_hasher;
use crate::io::files::{InputPaths, check_path_source, get_required_filenames};
use crate::progress::{ProgressCoordinator, progress_mode, suspended};

/// Returned by `worker_func` when one or more files could not be hashed.
///
/// Distinguished from config/arg errors so `main` can suppress the help banner
/// (individual file errors are already printed to stderr before this is returned).
#[derive(Debug)]
pub(crate) struct FileHashError;

impl std::fmt::Display for FileHashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "One or more files could not be hashed")
    }
}

impl std::error::Error for FileHashError {}

/// When hash lines reach the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineFlush {
    /// At the end, or when the buffer fills.
    Buffered,
    /// After each line, so a terminal shows each result as its file finishes.
    PerLine,
}

impl LineFlush {
    fn for_stdout() -> Self {
        if io::stdout().is_terminal() {
            Self::PerLine
        } else {
            Self::Buffered
        }
    }
}

fn print_hash_line(
    out: &mut impl Write,
    hash: &BasicHash,
    pathstr: &impl Display,
    exclude_fn: bool,
) -> io::Result<()> {
    if exclude_fn {
        writeln!(out, "{hash}")
    } else {
        writeln!(out, "{hash} {pathstr}")
    }
}

pub fn worker_func(config: &ConfigSettings) -> Result<()> {
    if config.debug_mode {
        show_initial_info(config);
    }

    check_path_source(config, io::stdin().is_terminal())?;
    let InputPaths {
        paths,
        had_error: input_error,
    } = get_required_filenames(config);

    if paths.is_empty() {
        if config.debug_mode {
            eprintln!("No files found");
        }
        return if input_error {
            Err(FileHashError.into())
        } else {
            Ok(())
        };
    }

    if config.debug_mode {
        eprintln!("Files to hash: {paths:?}");
    }

    let hash_error = if config.single_thread || paths.len() == 1 {
        file_hashes_st(config, &paths)
    } else {
        file_hashes_mt(config, &paths)
    };

    if input_error || hash_error {
        return Err(FileHashError.into());
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

fn file_hashes_st(config: &ConfigSettings, paths: &[PathBuf]) -> bool {
    if config.debug_mode {
        eprintln!("Single-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    let coordinator = create_coordinator(config, paths.len());
    let mut out = BufWriter::new(io::stdout().lock());

    // Lazy iterator: each file is hashed only when its line is about to be written
    let results = paths.iter().map(|path| {
        let file_hash = hash_with_progress(config, path, coordinator.as_ref());
        (path.display(), file_hash)
    });

    let had_error = write_results(
        &mut out,
        results,
        config.exclude_fn,
        LineFlush::for_stdout(),
        coordinator.as_ref(),
    );
    if let Some(c) = &coordinator {
        c.finish();
    }
    had_error
}

fn file_hashes_mt(config: &ConfigSettings, paths: &[PathBuf]) -> bool {
    if config.debug_mode {
        eprintln!("Multi-threaded mode");
        eprintln!("Algorithm: {:?}", config.algorithm);
    }

    let coordinator = create_coordinator(config, paths.len());

    let results: Vec<_> = paths
        .par_iter()
        .map(|path| {
            let file_hash = hash_with_progress(config, path, coordinator.as_ref());
            (path.display(), file_hash)
        })
        .collect();

    if let Some(c) = &coordinator {
        c.finish();
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    write_results(
        &mut out,
        results,
        config.exclude_fn,
        LineFlush::Buffered,
        coordinator.as_ref(),
    )
}

fn create_coordinator(config: &ConfigSettings, file_count: usize) -> Option<ProgressCoordinator> {
    let mode = progress_mode(config.no_progress, io::stderr().is_terminal(), file_count);
    ProgressCoordinator::new(mode, file_count)
}

/// Writes one line per successful hash and reports failures on stderr.
///
/// Returns `true` if any file failed to hash or any line failed to write.
/// A broken pipe stops output early. Terminal output is written with progress
/// cleared: error reports always, hash lines when flushed per line.
fn write_results<P: Display>(
    out: &mut impl Write,
    results: impl IntoIterator<Item = (P, Result<BasicHash>)>,
    exclude_fn: bool,
    flush: LineFlush,
    coordinator: Option<&ProgressCoordinator>,
) -> bool {
    let report = |message: &dyn Display| suspended(coordinator, || eprintln!("{message}"));
    let mut had_error = false;

    for (pathstr, file_hash) in results {
        let basic_hash = match file_hash {
            Ok(basic_hash) => basic_hash,
            Err(e) => {
                report(&format_args!("File error for '{pathstr}': {e}"));
                had_error = true;
                continue;
            }
        };
        let written = match flush {
            LineFlush::PerLine => suspended(coordinator, || {
                print_hash_line(out, &basic_hash, &pathstr, exclude_fn).and_then(|()| out.flush())
            }),
            LineFlush::Buffered => print_hash_line(out, &basic_hash, &pathstr, exclude_fn),
        };
        match written {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                let _ = out.flush();
                return had_error;
            }
            Err(e) => {
                report(&format_args!("Write error: {e}"));
                had_error = true;
            }
        }
    }

    if let Err(e) = out.flush()
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        report(&format_args!("Write error: {e}"));
        had_error = true;
    }

    had_error
}

fn hash_with_progress(
    config: &ConfigSettings,
    path: &Path,
    coordinator: Option<&ProgressCoordinator>,
) -> Result<BasicHash> {
    let spinner = coordinator.and_then(|coord| coord.start_file(path));

    let start_time = Instant::now();
    let result = call_hasher(config.algorithm, config.encoding, path);
    let elapsed = start_time.elapsed();

    if let Some(coord) = coordinator {
        coord.finish_file(spinner);
    }

    if config.debug_mode
        && elapsed >= Duration::from_millis(crate::progress::PROGRESS_THRESHOLD_MILLIS)
    {
        suspended(coordinator, || {
            eprintln!(
                "File '{}' took {:.2}s to hash",
                path.display(),
                elapsed.as_secs_f64()
            );
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct BrokenPipeWriter;

    impl io::Write for BrokenPipeWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_print_hash_line_propagates_broken_pipe() {
        let hash = BasicHash::new("deadbeef".to_string());
        let result = print_hash_line(&mut BrokenPipeWriter, &hash, &"f.txt", false);
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn test_print_hash_line_includes_filename() {
        let hash = BasicHash::new("abc123".to_string());
        let mut buf = Vec::new();
        print_hash_line(&mut buf, &hash, &"file.txt", false).unwrap();
        assert_eq!(buf, b"abc123 file.txt\n");
    }

    #[test]
    fn test_print_hash_line_excludes_filename_when_requested() {
        let hash = BasicHash::new("abc123".to_string());
        let mut buf = Vec::new();
        print_hash_line(&mut buf, &hash, &"file.txt", true).unwrap();
        assert_eq!(buf, b"abc123\n");
    }

    /// Records which bytes have been flushed, shared with the results iterator.
    #[derive(Clone, Default)]
    struct FlushRecorder {
        pending: Rc<RefCell<Vec<u8>>>,
        flushed: Rc<RefCell<Vec<u8>>>,
    }

    impl io::Write for FlushRecorder {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.pending.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            let pending = std::mem::take(&mut *self.pending.borrow_mut());
            self.flushed.borrow_mut().extend(pending);
            Ok(())
        }
    }

    /// Writes two lines and returns what was flushed when the second result was produced.
    fn flushed_before_second_result(flush: LineFlush) -> Vec<u8> {
        let mut recorder = FlushRecorder::default();
        let observer = recorder.clone();
        let seen = RefCell::new(Vec::new());
        let results = ["a.txt", "b.txt"].into_iter().enumerate().map(|(i, name)| {
            if i == 1 {
                *seen.borrow_mut() = observer.flushed.borrow().clone();
            }
            (name, Ok(BasicHash::new(format!("h{i}"))))
        });

        assert!(!write_results(&mut recorder, results, false, flush, None));
        assert_eq!(*recorder.flushed.borrow(), b"h0 a.txt\nh1 b.txt\n");
        seen.into_inner()
    }

    #[test]
    fn test_write_results_per_line_flushes_before_next_file() {
        assert_eq!(
            flushed_before_second_result(LineFlush::PerLine),
            b"h0 a.txt\n"
        );
    }

    #[test]
    fn test_write_results_buffered_flushes_at_end() {
        assert_eq!(flushed_before_second_result(LineFlush::Buffered), b"");
    }

    #[test]
    fn test_write_results_broken_pipe_keeps_earlier_error() {
        let results = [
            ("bad.txt", Err(anyhow::anyhow!("unreadable"))),
            ("good.txt", Ok(BasicHash::new("abc123".to_string()))),
        ];
        assert!(write_results(
            &mut BrokenPipeWriter,
            results,
            false,
            LineFlush::Buffered,
            None
        ));
    }

    #[test]
    fn test_write_results_broken_pipe_without_error_is_success() {
        let results = [("good.txt", Ok(BasicHash::new("abc123".to_string())))];
        assert!(!write_results(
            &mut BrokenPipeWriter,
            results,
            false,
            LineFlush::Buffered,
            None
        ));
    }
}
