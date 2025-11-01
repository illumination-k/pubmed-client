use anyhow::{Context as _, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::commands::create_pmc_client_with_timeout;
use crate::commands::storage::{create_storage_backend, StorageBackend};
use crate::Cli;

#[derive(Error, Debug)]
pub enum FiguresError {
    #[error("Failed to download TAR archive for PMC ID {pmcid}: {source}")]
    TarDownloadFailed {
        pmcid: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("No figures found in article {pmcid}")]
    NoFiguresFound { pmcid: String },

    #[error("Failed to save metadata for {pmcid}: {source}")]
    MetadataSaveFailed {
        pmcid: String,
        path: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Network timeout while processing {pmcid} (timeout: {timeout_seconds}s)")]
    NetworkTimeout { pmcid: String, timeout_seconds: u64 },

    #[error("Storage backend error for {pmcid}: {source}")]
    StorageError {
        pmcid: String,
        operation: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Unexpected error processing {pmcid}: {source}")]
    Other {
        pmcid: String,
        #[source]
        source: anyhow::Error,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    TarDownloadFailed {
        message: String,
        is_timeout: bool,
        is_network_error: bool,
    },
    NoFiguresFound,
    MetadataSaveFailed(String),
    NetworkTimeout {
        timeout_seconds: u64,
    },
    StorageError {
        operation: String,
        message: String,
    },
    Other(String),
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TarDownloadFailed {
                message,
                is_timeout,
                is_network_error,
            } => {
                write!(
                    f,
                    "TAR download failed: {}{}{}]",
                    message,
                    if *is_timeout { " (timeout)" } else { "" },
                    if *is_network_error {
                        " (network error)"
                    } else {
                        ""
                    }
                )
            }
            Self::NoFiguresFound => write!(f, "No figures found in article"),
            Self::MetadataSaveFailed(msg) => write!(f, "Metadata save failed: {}", msg),
            Self::NetworkTimeout { timeout_seconds } => {
                write!(f, "Network timeout after {} seconds", timeout_seconds)
            }
            Self::StorageError { operation, message } => {
                write!(f, "Storage error during {}: {}", operation, message)
            }
            Self::Other(msg) => write!(f, "Unexpected error: {}", msg),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FailedPmcId {
    pub pmcid: String,
    pub reason: FailureReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<String>,
}

pub struct FiguresOptions {
    pub pmcids: Vec<String>,
    pub output_dir: Option<PathBuf>,
    pub s3_path: Option<String>,
    pub s3_region: Option<String>,
    pub failed_output: Option<PathBuf>,
    pub timeout_seconds: Option<u64>,
    pub overwrite: bool,
}

pub async fn execute(options: FiguresOptions, cli: &Cli) -> Result<()> {
    // Create the appropriate storage backend
    let storage = create_storage_backend(options.output_dir, options.s3_path, options.s3_region)
        .await
        .context("Failed to create storage backend")?;

    // Initialize the PMC client with timeout (default to 180 seconds for figure extraction)
    let timeout = options.timeout_seconds.unwrap_or(180);
    let client = create_pmc_client_with_timeout(
        cli.api_key.as_deref(),
        cli.email.as_deref(),
        &cli.tool,
        Some(timeout),
    )
    .context("Failed to create PMC client")?;

    let mut failed_pmcids: Vec<FailedPmcId> = Vec::new();

    // Create progress bars
    let multi_progress = MultiProgress::new();
    let total_pmcids = options.pmcids.len();

    let main_pb = multi_progress.add(ProgressBar::new(total_pmcids as u64));
    main_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} articles ({msg})")
            .context("Failed to set progress bar style")?
            .progress_chars("#>-"),
    );
    main_pb.set_message("Processing PMC articles");

    // Process each PMCID
    for pmcid in &options.pmcids {
        main_pb.set_message(format!("Processing {}", pmcid));
        debug!(pmcid = %pmcid, "Processing article");

        match process_article_with_progress(
            &client,
            pmcid,
            storage.as_ref(),
            options.overwrite,
            &multi_progress,
        )
        .await
        {
            Ok(_) => {
                debug!(pmcid = %pmcid, "Successfully processed article");
                main_pb.set_message(format!("Completed {}", pmcid));
            }
            Err(e) => {
                error!(pmcid = %pmcid, error = %e, "Failed to process article");

                // Determine the failure reason based on the error
                let (reason, error_details) = categorize_failure_from_figures_error(&e, pmcid);

                failed_pmcids.push(FailedPmcId {
                    pmcid: pmcid.clone(),
                    reason,
                    error_details: Some(error_details),
                });
                main_pb.set_message(format!("Failed {}", pmcid));
                continue;
            }
        }

        main_pb.inc(1);
    }

    main_pb.finish_with_message(format!(
        "Processed {} articles ({} failed)",
        total_pmcids,
        failed_pmcids.len()
    ));

    if !failed_pmcids.is_empty() {
        // Save failed PMC IDs to file if output path is specified
        if let Some(failed_path) = options.failed_output {
            match save_failed_pmcids_json(&failed_pmcids, &failed_path, storage.as_ref()).await {
                Ok(_) => {
                    info!(
                        path = %failed_path.display(),
                        count = failed_pmcids.len(),
                        "Saved failed PMC IDs to JSON file"
                    );
                }
                Err(e) => {
                    error!(
                        path = %failed_path.display(),
                        error = %e,
                        "Failed to save failed PMC IDs to JSON file"
                    );
                }
            }
        } else {
            error!(
                failed_count = failed_pmcids.len(),
                failed_pmcids = ?failed_pmcids,
                "Failed to process some PMC IDs"
            );
        }
    }

    Ok(())
}

async fn process_article_with_progress(
    client: &pubmed_client::PmcClient,
    pmcid: &str,
    storage: &dyn StorageBackend,
    overwrite: bool,
    multi_progress: &MultiProgress,
) -> Result<(), FiguresError> {
    // Check if metadata file already exists and skip early if overwrite is false
    let article_dir_str = pmcid;
    let json_filename = format!("{}_figures_metadata.json", pmcid);
    let json_storage_path = format!("{}/{}", article_dir_str, json_filename);

    if !overwrite
        && storage
            .file_exists(&json_storage_path)
            .await
            .unwrap_or(false)
    {
        info!(pmcid = %pmcid, "Article already processed, skipping download (use --overwrite to force)");
        return Ok(());
    }

    // Create a temporary directory for extraction using tempfile crate
    // This ensures automatic cleanup on drop and avoids conflicts
    let temp_dir_handle = TempDir::new().map_err(|e| FiguresError::Other {
        pmcid: pmcid.to_string(),
        source: anyhow::anyhow!(e),
    })?;
    let temp_dir = temp_dir_handle.path();

    // Create progress bar for downloading
    let download_pb = multi_progress.add(ProgressBar::new_spinner());
    download_pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .map_err(|e| FiguresError::Other {
                pmcid: pmcid.to_string(),
                source: anyhow::anyhow!(e),
            })?,
    );
    download_pb.enable_steady_tick(Duration::from_millis(100));
    download_pb.set_message(format!("Downloading and extracting figures from {}", pmcid));

    let figures = match client.extract_figures_with_captions(pmcid, temp_dir).await {
        Ok(figures) => {
            download_pb.finish_with_message(format!(
                "Downloaded {} figures from {}",
                figures.len(),
                pmcid
            ));
            figures
        }
        Err(e) => {
            download_pb.finish_with_message(format!("Failed to download {}", pmcid));
            // Temporary directory will be automatically cleaned up when temp_dir_handle is dropped
            error!(
                pmcid = %pmcid,
                error = %e,
                "Failed to download TAR archive or extract figures"
            );

            // Check if it's a timeout error
            let error_str = e.to_string();
            if error_str.contains("timeout") || error_str.contains("timed out") {
                return Err(FiguresError::NetworkTimeout {
                    pmcid: pmcid.to_string(),
                    timeout_seconds: client.get_tar_client_config().timeout.as_secs(),
                });
            }

            return Err(FiguresError::TarDownloadFailed {
                pmcid: pmcid.to_string(),
                source: anyhow::anyhow!(e),
            });
        }
    };

    if figures.is_empty() {
        // Temporary directory will be automatically cleaned up when temp_dir_handle is dropped
        return Err(FiguresError::NoFiguresFound {
            pmcid: pmcid.to_string(),
        });
    }

    // Now that we have figures, ensure the storage directory exists
    storage
        .ensure_directory(article_dir_str)
        .await
        .map_err(|e| FiguresError::StorageError {
            pmcid: pmcid.to_string(),
            operation: "ensure_directory".to_string(),
            source: e,
        })?;
    debug!(directory = %storage.get_full_path(article_dir_str), "Created storage directory");

    // Keep the temp directory from being auto-deleted
    let _temp_path = temp_dir_handle.keep();

    debug!(pmcid = %pmcid, figure_count = figures.len(), "Found figures");

    // Create progress bar for figure processing
    let figure_pb = multi_progress.add(ProgressBar::new(figures.len() as u64));
    figure_pb.set_style(
        ProgressStyle::default_bar()
            .template("  {spinner:.green} Saving figures [{bar:30.cyan/blue}] {pos}/{len} {msg}")
            .map_err(|e| FiguresError::Other {
                pmcid: pmcid.to_string(),
                source: anyhow::anyhow!(e),
            })?
            .progress_chars("#>-"),
    );
    figure_pb.set_message(pmcid.to_string());

    // Process each figure
    let mut figure_metadata = Vec::new();

    for (index, extracted_figure) in figures.iter().enumerate() {
        let figure_num = index + 1;
        figure_pb.set_message(format!("{}: {}", pmcid, extracted_figure.figure.id));
        debug!(
            figure_number = figure_num,
            figure_id = %extracted_figure.figure.id,
            "Processing figure"
        );

        // Create metadata for this figure
        let metadata = FigureMetadata {
            pmcid: pmcid.to_string(),
            figureid: extracted_figure.figure.id.clone(),
            label: extracted_figure.figure.label.clone(),
            caption: extracted_figure.figure.caption.clone(),
        };

        // The extracted_file_path contains the full path from extraction
        // The file should be in the temp directory we created
        let actual_file_path = Path::new(&extracted_figure.extracted_file_path);

        // Copy figure to a standardized filename
        if let (Some(extension), Some(_filename)) =
            (actual_file_path.extension(), actual_file_path.file_stem())
        {
            let new_filename = format!(
                "{}_{}.{}",
                pmcid,
                extracted_figure.figure.id,
                extension.to_string_lossy()
            );
            let storage_path = format!("{}/{}", article_dir_str, new_filename);

            // Copy figure to storage
            match storage.copy_file(actual_file_path, &storage_path).await {
                Err(e) => {
                    warn!(
                        error = %e,
                        source = %actual_file_path.display(),
                        target = %storage_path,
                        figure_id = %extracted_figure.figure.id,
                        "Could not copy figure to storage"
                    );
                    // Note: We continue processing other figures even if one fails
                }
                Ok(_) => {
                    debug!(
                        filename = %new_filename,
                        location = %storage.get_full_path(&storage_path),
                        "Saved figure"
                    );

                    if let Some(dimensions) = extracted_figure.dimensions {
                        debug!(
                            width = dimensions.0,
                            height = dimensions.1,
                            "Figure dimensions"
                        );
                    }

                    if let Some(size) = extracted_figure.file_size {
                        debug!(size_kb = size / 1024, "Figure size (KB)");
                    }
                }
            }
        }

        figure_metadata.push(metadata);
        figure_pb.inc(1);
    }

    figure_pb.finish_with_message(format!("{}: Saved {} figures", pmcid, figures.len()));

    // Save metadata as JSON to storage (always save metadata if we got this far)
    let json_content = serde_json::to_string_pretty(&figure_metadata).map_err(|e| {
        FiguresError::MetadataSaveFailed {
            pmcid: pmcid.to_string(),
            path: json_storage_path.clone(),
            source: anyhow::anyhow!(e),
        }
    })?;

    storage
        .write_file(&json_storage_path, json_content.as_bytes())
        .await
        .map_err(|e| FiguresError::MetadataSaveFailed {
            pmcid: pmcid.to_string(),
            path: json_storage_path.clone(),
            source: e,
        })?;

    let metadata_action = if storage
        .file_exists(&json_storage_path)
        .await
        .unwrap_or(false)
        && overwrite
    {
        "Overwritten"
    } else {
        "Saved"
    };
    debug!(filename = %json_filename, location = %storage.get_full_path(&json_storage_path), "{} metadata", metadata_action);

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct FigureMetadata {
    pmcid: String,
    figureid: String,
    label: Option<String>,
    caption: String,
}

fn categorize_failure_from_figures_error(
    error: &FiguresError,
    _pmcid: &str,
) -> (FailureReason, String) {
    let error_str = error.to_string();
    let error_chain = format!("{:#}", error); // Include full error chain for debugging

    let reason = match error {
        FiguresError::TarDownloadFailed { .. } => {
            let is_timeout = error_str.contains("timeout") || error_str.contains("timed out");
            let is_network = error_str.contains("network") || error_str.contains("connection");
            FailureReason::TarDownloadFailed {
                message: error_str.clone(),
                is_timeout,
                is_network_error: is_network,
            }
        }
        FiguresError::NoFiguresFound { .. } => FailureReason::NoFiguresFound,
        FiguresError::MetadataSaveFailed { path, .. } => FailureReason::MetadataSaveFailed(
            format!("Failed to save metadata to {}: {}", path, error_str),
        ),
        FiguresError::NetworkTimeout {
            timeout_seconds, ..
        } => FailureReason::NetworkTimeout {
            timeout_seconds: *timeout_seconds,
        },
        FiguresError::StorageError { operation, .. } => FailureReason::StorageError {
            operation: operation.clone(),
            message: error_str.clone(),
        },
        FiguresError::Other { .. } => FailureReason::Other(error_str.clone()),
    };

    (reason, error_chain)
}

async fn save_failed_pmcids_json(
    failed_pmcids: &[FailedPmcId],
    path: &Path,
    storage: &dyn StorageBackend,
) -> Result<()> {
    // Convert to JSON with pretty formatting
    let json_content = serde_json::to_string_pretty(failed_pmcids)?;

    // Get just the filename from the path
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid failed output path"))?;

    // Ensure the filename has .json extension
    let json_filename = if !filename.ends_with(".json") {
        format!("{}.json", filename.trim_end_matches('.'))
    } else {
        filename.to_string()
    };

    // Write to storage
    storage
        .write_file(&json_filename, json_content.as_bytes())
        .await?;

    Ok(())
}
