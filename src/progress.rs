//! Progress indication module for file hashing operations
//!
//! This module provides progress tracking for both single files (with spinners for long operations)
//! and multiple files (with progress bars for large sets). Uses `indicatif::MultiProgress` for
//! coordinated rendering without manual thread management.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;
use std::time::Duration;

pub const PROGRESS_THRESHOLD_MILLIS: u64 = 200;

/// Minimum number of files required to show an overall progress bar.
/// Below this threshold, per-file spinners are used instead.
const OVERALL_BAR_FILE_THRESHOLD: usize = 10;

/// Tick interval for spinner animations, in milliseconds.
const SPINNER_TICK_MS: u64 = 350;

/// How progress is shown for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    /// No progress output.
    Off,
    /// One spinner per file being hashed.
    Spinners,
    /// One bar counting completed files.
    Overall,
}

/// Which standard streams are terminals.
#[derive(Debug, Clone, Copy)]
pub struct Terminals {
    pub stdout: bool,
    pub stderr: bool,
}

/// Chooses the progress display for a run.
///
/// Progress is drawn on stderr, so it needs a terminal there. Large sets use one
/// overall bar, because each spinner runs its own tick thread. When hash lines
/// stream to the same terminal, a bar would interleave with them, so spinners are
/// used: each one finishes before its line is written.
#[must_use]
pub fn progress_mode(
    no_progress: bool,
    streaming_output: bool,
    terminals: Terminals,
    file_count: usize,
) -> ProgressMode {
    if no_progress || !terminals.stderr {
        ProgressMode::Off
    } else if file_count < OVERALL_BAR_FILE_THRESHOLD || (streaming_output && terminals.stdout) {
        ProgressMode::Spinners
    } else {
        ProgressMode::Overall
    }
}

/// Draws progress for one run, as chosen by [`progress_mode`].
pub struct ProgressCoordinator {
    multi: MultiProgress,
    overall: Option<ProgressBar>,
}

impl ProgressCoordinator {
    /// Returns `None` when `mode` is [`ProgressMode::Off`].
    pub fn new(mode: ProgressMode, file_count: usize) -> Option<Self> {
        let multi = MultiProgress::new();
        let overall = match mode {
            ProgressMode::Off => return None,
            ProgressMode::Spinners => None,
            ProgressMode::Overall => Some(multi.add(overall_bar(file_count))),
        };
        Some(Self { multi, overall })
    }

    /// Starts a spinner for one file, unless the overall bar is shown.
    pub fn start_file(&self, path: &Path) -> Option<ProgressBar> {
        self.overall
            .is_none()
            .then(|| self.multi.add(spinner(path)))
    }

    /// Clears the file's spinner and advances the overall bar.
    pub fn finish_file(&self, spinner: Option<ProgressBar>) {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
        if let Some(pb) = &self.overall {
            pb.inc(1);
        }
    }

    /// Marks the overall bar complete.
    pub fn finish(&self) {
        if let Some(pb) = &self.overall {
            pb.finish_with_message("Complete!");
        }
    }
}

fn overall_bar(file_count: usize) -> ProgressBar {
    let style = ProgressStyle::default_bar()
        .template("{bar:40.cyan/blue} {pos}/{len} files ({percent}%) {msg}")
        .unwrap_or_else(|_| {
            ProgressStyle::default_bar()
                .template("{bar:40} {pos}/{len} files")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
        });
    let pb = ProgressBar::new(file_count as u64).with_style(style);
    pb.set_message("Processing...");
    pb
}

fn spinner(path: &Path) -> ProgressBar {
    let style = ProgressStyle::default_spinner()
        .template("{spinner:.green} Hashing {msg}...")
        .unwrap_or_else(|_| {
            ProgressStyle::default_spinner()
                .template("{spinner} Hashing...")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
        });
    let pb = ProgressBar::new_spinner().with_style(style);
    pb.set_message(path.display().to_string());
    pb.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MS));
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: Terminals = Terminals {
        stdout: true,
        stderr: true,
    };
    const STDERR_ONLY: Terminals = Terminals {
        stdout: false,
        stderr: true,
    };
    const NEITHER: Terminals = Terminals {
        stdout: false,
        stderr: false,
    };
    const LARGE: usize = OVERALL_BAR_FILE_THRESHOLD;
    const SMALL: usize = OVERALL_BAR_FILE_THRESHOLD - 1;

    #[test]
    fn test_progress_off_when_requested() {
        assert_eq!(progress_mode(true, false, BOTH, SMALL), ProgressMode::Off);
    }

    #[test]
    fn test_progress_off_when_stderr_not_terminal() {
        assert_eq!(
            progress_mode(false, false, NEITHER, SMALL),
            ProgressMode::Off
        );
        assert_eq!(
            progress_mode(false, false, NEITHER, LARGE),
            ProgressMode::Off
        );
    }

    #[test]
    fn test_progress_spinners_for_small_sets() {
        assert_eq!(
            progress_mode(false, false, BOTH, SMALL),
            ProgressMode::Spinners
        );
        assert_eq!(
            progress_mode(false, true, BOTH, SMALL),
            ProgressMode::Spinners
        );
    }

    #[test]
    fn test_progress_overall_bar_for_large_collected_sets() {
        assert_eq!(
            progress_mode(false, false, BOTH, LARGE),
            ProgressMode::Overall
        );
    }

    #[test]
    fn test_progress_overall_bar_when_streaming_to_redirected_stdout() {
        assert_eq!(
            progress_mode(false, true, STDERR_ONLY, LARGE),
            ProgressMode::Overall
        );
    }

    // Streamed lines sit in a buffered writer, so they do not show progress.
    // Spinners finish before each line is written, so they never interleave.
    #[test]
    fn test_progress_spinners_when_streaming_large_set_to_terminal() {
        assert_eq!(
            progress_mode(false, true, BOTH, LARGE),
            ProgressMode::Spinners
        );
    }
}
