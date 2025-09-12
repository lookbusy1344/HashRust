//! Progress indication module for file hashing operations
//!
//! This module provides progress tracking for both single files (with spinners for long operations)
//! and multiple files (with progress bars for large sets). It manages thread safety and resource
//! limits to prevent system exhaustion.

use indicatif::{ProgressBar, ProgressStyle};
use std::fmt::Display;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

// Global counter for active progress threads to prevent resource exhaustion
static ACTIVE_PROGRESS_THREADS: AtomicUsize = AtomicUsize::new(0);
const MAX_PROGRESS_THREADS: usize = 4;
const PROGRESS_THRESHOLD_MILLIS: u64 = 200;

/// Handle for a progress indication session
pub struct ProgressHandle {
    sender: Option<mpsc::Sender<()>>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl ProgressHandle {
    /// Signal completion and clean up resources
    pub fn finish(mut self, debug_mode: bool) {
        // Signal completion - this will wake up the progress thread immediately
        if let Some(tx) = self.sender.take() {
            let _ = tx.send(());
        }

        // Wait for progress thread to finish - simple cleanup
        if let Some(handle) = self.thread_handle.take()
            && handle.join().is_err()
            && debug_mode
        {
            eprintln!("Progress thread join failed");
        }
    }
}

/// Manager for progress indication across different operation types
pub struct ProgressManager;

impl ProgressManager {
    /// Create a progress indication for a single file operation
    /// Shows a spinner if the operation takes longer than the threshold
    pub fn create_file_progress<S>(pathstr: S, _debug_mode: bool) -> Option<ProgressHandle>
    where
        S: AsRef<str> + Display + Clone + Send + 'static,
    {
        // Only show progress spinners if we haven't exceeded thread limit
        let should_show_progress =
            ACTIVE_PROGRESS_THREADS.load(Ordering::Relaxed) < MAX_PROGRESS_THREADS;

        if !should_show_progress {
            return None;
        }

        // Create a channel to signal completion
        let (tx, rx) = mpsc::channel();

        // Increment the counter
        ACTIVE_PROGRESS_THREADS.fetch_add(1, Ordering::Relaxed);

        let handle = std::thread::spawn(move || {
            // Wait for either completion signal or threshold timeout
            match rx.recv_timeout(Duration::from_millis(PROGRESS_THRESHOLD_MILLIS)) {
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Threshold passed, show progress spinner
                    let pb = Self::create_progress_spinner(pathstr.as_ref());
                    pb.enable_steady_tick(Duration::from_millis(120));

                    // Wait for completion signal
                    let _ = rx.recv();
                    pb.finish_and_clear();
                }
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Operation completed before threshold or sender dropped
                }
            }

            // Decrement the counter when thread finishes
            ACTIVE_PROGRESS_THREADS.fetch_sub(1, Ordering::Relaxed);
        });

        Some(ProgressHandle {
            sender: Some(tx),
            thread_handle: Some(handle),
        })
    }

    /// Create an overall progress bar for multiple file operations
    pub fn create_overall_progress(
        file_count: usize,
        _debug_mode: bool,
    ) -> Option<Arc<ProgressBar>> {
        // For large file sets, show an overall progress bar instead of per-file spinners
        if file_count < 10 {
            return None;
        }

        let pb = ProgressBar::new(file_count as u64);
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

    /// Create a progress spinner with safe error handling
    fn create_progress_spinner(pathstr: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();

        // Use unwrap_or_else to provide fallback template if parsing fails
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.green} Hashing {msg}...")
            .unwrap_or_else(|_| {
                // Fallback to simpler template if the main one fails
                ProgressStyle::default_spinner()
                    .template("{spinner} Hashing...")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner())
            });

        pb.set_style(style);
        pb.set_message(pathstr.to_string());
        pb
    }

    /// Get the progress threshold in milliseconds
    pub fn threshold_millis() -> u64 {
        PROGRESS_THRESHOLD_MILLIS
    }
}
