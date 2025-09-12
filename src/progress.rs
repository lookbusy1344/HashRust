//! Progress indication module for file hashing operations
//! 
//! This module provides progress tracking for both single files (with spinners for long operations)
//! and multiple files (with progress bars for large sets). It manages thread safety and resource
//! limits to prevent system exhaustion.

use std::sync::{mpsc, Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;
use std::fmt::Display;
use indicatif::{ProgressBar, ProgressStyle};

// Global counter for active progress threads to prevent resource exhaustion
static ACTIVE_PROGRESS_THREADS: AtomicUsize = AtomicUsize::new(0);
const MAX_PROGRESS_THREADS: usize = 4;
const PROGRESS_THRESHOLD_SECS: u64 = 1;

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
        if let Some(handle) = self.thread_handle.take() {
            if handle.join().is_err() && debug_mode {
                eprintln!("Progress thread join failed");
            }
        }
    }
}

/// Manager for progress indication across different operation types
pub struct ProgressManager;

impl ProgressManager {
    /// Create a progress indication for a single file operation
    /// Shows a spinner if the operation takes longer than the threshold
    pub fn create_file_progress<S>(pathstr: S, debug_mode: bool) -> Option<ProgressHandle>
    where
        S: AsRef<str> + Display + Clone + Send + 'static,
    {
        // Only show progress spinners if not in debug mode and we haven't exceeded thread limit
        let should_show_progress = !debug_mode 
            && ACTIVE_PROGRESS_THREADS.load(Ordering::Relaxed) < MAX_PROGRESS_THREADS;
        
        if !should_show_progress {
            return None;
        }
        
        // Create a channel to signal completion
        let (tx, rx) = mpsc::channel();
        
        // Increment the counter
        ACTIVE_PROGRESS_THREADS.fetch_add(1, Ordering::Relaxed);
        
        let handle = std::thread::spawn(move || {
            // Wait for either completion signal or threshold timeout
            match rx.recv_timeout(Duration::from_secs(PROGRESS_THRESHOLD_SECS)) {
                Ok(()) => {
                    // Operation completed before threshold, no progress needed
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Threshold passed, show progress spinner
                    let pb = Self::create_progress_spinner(&pathstr);
                    if let Some(pb) = pb {
                        pb.enable_steady_tick(Duration::from_millis(120));
                        
                        // Wait for completion signal
                        let _ = rx.recv();
                        pb.finish_and_clear();
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Sender dropped, operation completed
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
    pub fn create_overall_progress(file_count: usize, debug_mode: bool) -> Option<Arc<ProgressBar>> {
        // For large file sets, show an overall progress bar instead of per-file spinners
        if debug_mode || file_count < 10 {
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
    fn create_progress_spinner(pathstr: &str) -> Option<ProgressBar> {
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
        Some(pb)
    }
    
    /// Get the progress threshold in seconds
    pub fn threshold_secs() -> u64 {
        PROGRESS_THRESHOLD_SECS
    }
}