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

/// Chooses the progress display for a run.
///
/// Progress is drawn on stderr, so it needs a terminal there. Large sets use one
/// overall bar, because each spinner runs its own tick thread. Output on the same
/// terminal is written through [`suspended`], so it does not land inside the bar.
#[must_use]
pub fn progress_mode(
    no_progress: bool,
    stderr_is_terminal: bool,
    file_count: usize,
) -> ProgressMode {
    if no_progress || !stderr_is_terminal {
        ProgressMode::Off
    } else if file_count < OVERALL_BAR_FILE_THRESHOLD {
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

    /// Runs `f` with all progress output cleared, then redraws it.
    pub fn suspend<R>(&self, f: impl FnOnce() -> R) -> R {
        self.multi.suspend(f)
    }

    /// Marks the overall bar complete.
    pub fn finish(&self) {
        if let Some(pb) = &self.overall {
            pb.finish_with_message("Complete!");
        }
    }
}

/// Runs `f` with progress cleared when there is any, so its output does not land inside a bar.
pub fn suspended<R>(coordinator: Option<&ProgressCoordinator>, f: impl FnOnce() -> R) -> R {
    match coordinator {
        Some(c) => c.suspend(f),
        None => f(),
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

    const LARGE: usize = OVERALL_BAR_FILE_THRESHOLD;
    const SMALL: usize = OVERALL_BAR_FILE_THRESHOLD - 1;

    #[test]
    fn test_progress_off_when_requested() {
        assert_eq!(progress_mode(true, true, SMALL), ProgressMode::Off);
        assert_eq!(progress_mode(true, true, LARGE), ProgressMode::Off);
    }

    #[test]
    fn test_progress_off_when_stderr_not_terminal() {
        assert_eq!(progress_mode(false, false, SMALL), ProgressMode::Off);
        assert_eq!(progress_mode(false, false, LARGE), ProgressMode::Off);
    }

    #[test]
    fn test_progress_spinners_for_small_sets() {
        assert_eq!(progress_mode(false, true, SMALL), ProgressMode::Spinners);
    }

    // Each spinner runs its own tick thread, so large sets always use one bar.
    #[test]
    fn test_progress_overall_bar_for_large_sets() {
        assert_eq!(progress_mode(false, true, LARGE), ProgressMode::Overall);
    }
}
