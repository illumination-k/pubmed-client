//! Shared building blocks for batch CLI commands (figures, metadata, ...).
//!
//! Batch commands all follow the same shape: take a list of IDs, process each
//! one independently, show a progress bar, collect per-item failures, and
//! optionally dump those failures as JSON. This module factors out that
//! framework so individual commands only need to describe how to process a
//! single item.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::commands::storage::StorageBackend;

/// Categorized reason a single batch item failed.
///
/// Shared across batch commands so error handling improvements apply uniformly.
/// `message` on [`BatchItemError`] carries the human-readable detail; this enum
/// captures the machine-readable category.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// A network request timed out.
    NetworkTimeout {
        /// The configured timeout, in seconds.
        timeout_seconds: u64,
    },
    /// A network or connection error that was not a timeout.
    NetworkError,
    /// The requested article/resource was not found.
    NotFound,
    /// The article was retrieved but contained no usable content
    /// (e.g. no figures to extract).
    Empty,
    /// Fetching or downloading the source data failed.
    FetchFailed,
    /// Writing the output to storage failed.
    StorageError {
        /// The storage operation that failed (e.g. `write_metadata`).
        operation: String,
    },
    /// Any other, uncategorized error.
    Other,
}

impl fmt::Display for FailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkTimeout { timeout_seconds } => {
                write!(f, "network timeout after {} seconds", timeout_seconds)
            }
            Self::NetworkError => write!(f, "network error"),
            Self::NotFound => write!(f, "not found"),
            Self::Empty => write!(f, "no content"),
            Self::FetchFailed => write!(f, "fetch failed"),
            Self::StorageError { operation } => write!(f, "storage error during {}", operation),
            Self::Other => write!(f, "unexpected error"),
        }
    }
}

/// A single failed batch item, serialized into the `--failed-output` JSON file.
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchItemError {
    /// The ID (e.g. PMC ID) of the item that failed.
    pub id: String,
    /// The categorized failure reason.
    pub kind: FailureKind,
    /// Human-readable detail (typically the full error chain).
    pub message: String,
}

impl BatchItemError {
    /// Create a new batch item error.
    pub fn new(id: impl Into<String>, kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for BatchItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.id, self.kind, self.message)
    }
}

/// Inspect an error string and classify it as a timeout, not-found, or generic
/// network failure. Helper for the common case of mapping an opaque client
/// error into a [`FailureKind`].
pub fn classify_fetch_error(error_str: &str, timeout_seconds: u64) -> FailureKind {
    let lower = error_str.to_lowercase();
    if lower.contains("timeout") || lower.contains("timed out") {
        FailureKind::NetworkTimeout { timeout_seconds }
    } else if lower.contains("not found") || lower.contains("404") {
        FailureKind::NotFound
    } else if lower.contains("network") || lower.contains("connection") {
        FailureKind::NetworkError
    } else {
        FailureKind::FetchFailed
    }
}

/// Drives a batch command: owns the progress bars and accumulates failures.
///
/// Construct with [`BatchProcessor::new`], run items via [`BatchProcessor::run`]
/// (or manually with [`BatchProcessor::record`]), then [`BatchProcessor::finish`]
/// and inspect [`BatchProcessor::failures`].
pub struct BatchProcessor {
    multi_progress: MultiProgress,
    main_pb: ProgressBar,
    total: usize,
    failures: Vec<BatchItemError>,
}

impl BatchProcessor {
    /// Create a processor for `total` items, setting up the main progress bar.
    pub fn new(total: usize) -> Result<Self> {
        let multi_progress = MultiProgress::new();
        let main_pb = multi_progress.add(ProgressBar::new(total as u64));
        main_pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} articles ({msg})")
                .context("Failed to set progress bar style")?
                .progress_chars("#>-"),
        );
        main_pb.set_message("Processing PMC articles");
        Ok(Self {
            multi_progress,
            main_pb,
            total,
            failures: Vec::new(),
        })
    }

    /// Process every ID in `ids` sequentially.
    ///
    /// `process` receives the shared [`MultiProgress`] and an item ID, performs
    /// all work for that item (including writing its output), and returns
    /// `Ok(())` on success or a [`BatchItemError`] on failure. Failures are
    /// collected and processing continues with the next item.
    pub async fn run<F>(&mut self, ids: &[String], process: F)
    where
        F: AsyncFn(&MultiProgress, &str) -> Result<(), BatchItemError>,
    {
        for id in ids {
            self.main_pb.set_message(format!("Processing {}", id));
            debug!(id = %id, "Processing batch item");
            let result = process(&self.multi_progress, id).await;
            self.record(id, result);
        }
    }

    /// Process every ID in `ids` with up to `concurrency` items in flight.
    ///
    /// Behaves like [`BatchProcessor::run`] but overlaps the per-item work so
    /// network-bound commands (e.g. downloading an article's files from the PMC
    /// OA Cloud) don't idle waiting on each other. NCBI E-utilities requests
    /// inside `process` remain governed by the client's shared rate limiter, so
    /// raising concurrency never exceeds the NCBI quota — it only pipelines the
    /// non-eutils (S3) work and hides per-request latency.
    ///
    /// Results are recorded in completion order; the main progress bar advances
    /// as each item finishes. A `concurrency` of 0 or 1 runs sequentially.
    pub async fn run_concurrent<F>(&mut self, ids: &[String], concurrency: usize, process: F)
    where
        F: AsyncFn(&MultiProgress, &str) -> Result<(), BatchItemError>,
    {
        use futures_util::stream::{self, StreamExt};

        // Clone the progress handles so the streaming borrow does not conflict
        // with the `&mut self` needed to record failures afterwards. Both are
        // cheap Arc-backed handles.
        let multi_progress = self.multi_progress.clone();
        let main_pb = self.main_pb.clone();

        // Borrow `process` and the progress handle so each spawned future copies
        // a reference instead of moving the (non-`Copy`) closure.
        let process = &process;
        let multi_progress = &multi_progress;

        let mut collected: Vec<BatchItemError> = Vec::new();
        {
            let mut in_flight = stream::iter(ids.iter())
                .map(|id| async move {
                    debug!(id = %id, "Processing batch item");
                    let result = process(multi_progress, id).await;
                    (id.as_str(), result)
                })
                .buffer_unordered(concurrency.max(1));

            while let Some((id, result)) = in_flight.next().await {
                match result {
                    Ok(()) => {
                        debug!(id = %id, "Successfully processed batch item");
                        main_pb.set_message(format!("Completed {}", id));
                    }
                    Err(e) => {
                        error!(id = %id, kind = %e.kind, message = %e.message, "Failed to process batch item");
                        main_pb.set_message(format!("Failed {}", id));
                        collected.push(e);
                    }
                }
                main_pb.inc(1);
            }
        }

        self.failures.extend(collected);
    }

    /// Record the outcome of processing a single item, updating the progress
    /// bar and the failure list. Useful when a command needs a custom loop
    /// instead of [`BatchProcessor::run`].
    pub fn record(&mut self, id: &str, result: Result<(), BatchItemError>) {
        match result {
            Ok(()) => {
                debug!(id = %id, "Successfully processed batch item");
                self.main_pb.set_message(format!("Completed {}", id));
            }
            Err(e) => {
                error!(id = %id, kind = %e.kind, message = %e.message, "Failed to process batch item");
                self.main_pb.set_message(format!("Failed {}", id));
                self.failures.push(e);
            }
        }
        self.main_pb.inc(1);
    }

    /// Finish the main progress bar with a summary message.
    pub fn finish(&self) {
        self.main_pb.finish_with_message(format!(
            "Processed {} articles ({} failed)",
            self.total,
            self.failures.len()
        ));
    }

    /// The collected failures.
    pub fn failures(&self) -> &[BatchItemError] {
        &self.failures
    }
}

/// Handle the failures collected by a [`BatchProcessor`].
///
/// If `failed_output` is set, the failures are serialized to JSON and written
/// to that path (filename only, normalized to a `.json` extension) inside the
/// given `storage` backend. Otherwise they are logged. Storage write errors are
/// logged but not propagated, so a failure to record failures never masks the
/// primary command result.
pub async fn report_failures(
    failures: &[BatchItemError],
    failed_output: Option<PathBuf>,
    storage: &dyn StorageBackend,
) -> Result<()> {
    if failures.is_empty() {
        return Ok(());
    }

    let Some(failed_path) = failed_output else {
        error!(
            failed_count = failures.len(),
            failures = ?failures,
            "Failed to process some IDs"
        );
        return Ok(());
    };

    let json_content = serde_json::to_string_pretty(failures)?;
    let json_filename = failed_output_filename(&failed_path)?;

    match storage
        .write_file(&json_filename, json_content.as_bytes())
        .await
    {
        Ok(()) => info!(
            path = %storage.get_full_path(&json_filename),
            count = failures.len(),
            "Saved failed IDs to JSON file"
        ),
        Err(e) => error!(
            path = %failed_path.display(),
            error = %e,
            "Failed to save failed IDs to JSON file"
        ),
    }

    Ok(())
}

/// Extract the filename from `path` and ensure it ends with `.json`.
fn failed_output_filename(path: &Path) -> Result<String> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid failed output path: {}", path.display()))?;

    Ok(if filename.ends_with(".json") {
        filename.to_string()
    } else {
        format!("{}.json", filename.trim_end_matches('.'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_fetch_error() {
        assert!(matches!(
            classify_fetch_error("request timed out", 30),
            FailureKind::NetworkTimeout {
                timeout_seconds: 30
            }
        ));
        assert!(matches!(
            classify_fetch_error("HTTP 404 returned", 30),
            FailureKind::NotFound
        ));
        assert!(matches!(
            classify_fetch_error("Article not found", 30),
            FailureKind::NotFound
        ));
        assert!(matches!(
            classify_fetch_error("connection reset", 30),
            FailureKind::NetworkError
        ));
        assert!(matches!(
            classify_fetch_error("something odd", 30),
            FailureKind::FetchFailed
        ));
    }

    #[test]
    fn test_failed_output_filename() {
        assert_eq!(
            failed_output_filename(Path::new("/tmp/failed.json")).unwrap(),
            "failed.json"
        );
        assert_eq!(
            failed_output_filename(Path::new("failed")).unwrap(),
            "failed.json"
        );
        assert_eq!(
            failed_output_filename(Path::new("dir/failed.")).unwrap(),
            "failed.json"
        );
    }

    #[tokio::test]
    async fn test_run_concurrent_processes_all_and_collects_failures() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let ids: Vec<String> = (0..10).map(|i| format!("PMC{i}")).collect();
        let processed = AtomicUsize::new(0);

        let mut processor = BatchProcessor::new(ids.len()).unwrap();
        processor
            .run_concurrent(&ids, 4, async |_mp, id| {
                processed.fetch_add(1, Ordering::SeqCst);
                // Fail every item whose numeric suffix is even.
                let n: usize = id.trim_start_matches("PMC").parse().unwrap();
                if n.is_multiple_of(2) {
                    Err(BatchItemError::new(id, FailureKind::Empty, "even"))
                } else {
                    Ok(())
                }
            })
            .await;

        // Every id runs exactly once regardless of concurrency.
        assert_eq!(processed.load(Ordering::SeqCst), ids.len());
        // The 5 even ids (0,2,4,6,8) are recorded as failures.
        assert_eq!(processor.failures().len(), 5);
        assert!(
            processor
                .failures()
                .iter()
                .all(|f| matches!(f.kind, FailureKind::Empty))
        );
    }

    #[test]
    fn test_failure_kind_serialization() {
        let err = BatchItemError::new(
            "PMC123",
            FailureKind::NetworkTimeout {
                timeout_seconds: 60,
            },
            "boom",
        );
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"id\":\"PMC123\""));
        assert!(json.contains("network_timeout"));
        assert!(json.contains("\"timeout_seconds\":60"));
    }
}
