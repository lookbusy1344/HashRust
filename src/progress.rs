//! Progress indication module for file hashing operations
//!
//! This module provides progress tracking for both single files (with spinners for long operations)
//! and multiple files (with progress bars for large sets). Uses `indicatif::MultiProgress` for
//! coordinated rendering without manual thread management.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Duration;

pub const PROGRESS_THRESHOLD_MILLIS: u64 = 200;

/// Coordinator for all progress indication
///
/// Wraps `MultiProgress` to provide application-specific progress display logic.
pub struct ProgressCoordinator {
    multi: MultiProgress,
}

impl ProgressCoordinator {
    /// Create a new progress coordinator
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
        }
    }

    /// Create an overall progress bar for multiple file operations
    ///
    /// Returns `None` for small file sets (< 10 files) where per-file spinners are more appropriate.
    pub fn create_overall_progress(&self, file_count: usize) -> Option<Arc<ProgressBar>> {
        if file_count < 10 {
            return None;
        }

        let pb = self.multi.add(ProgressBar::new(file_count as u64));
        let style = ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos}/{len} files ({percent}%) {msg}")
            .unwrap_or_else(|_| {
                ProgressStyle::default_bar()
                    .template("{bar:40} {pos}/{len} files")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
            });
        pb.set_style(style);
        pb.set_message("Processing...");
        Some(Arc::new(pb))
    }

    /// Create a spinner for a single file operation
    ///
    /// The spinner is added to the `MultiProgress` for coordinated rendering.
    pub fn create_spinner(&self, pathstr: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new_spinner());
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.green} Hashing {msg}...")
            .unwrap_or_else(|_| {
                ProgressStyle::default_spinner()
                    .template("{spinner} Hashing...")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner())
            });

        pb.set_style(style);
        pb.set_message(pathstr.to_string());
        pb.enable_steady_tick(Duration::from_millis(350));
        pb
    }
}

impl Default for ProgressCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
