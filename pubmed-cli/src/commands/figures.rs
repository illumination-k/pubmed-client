use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use tracing::{debug, info, warn};

use crate::commands::ClientContext;
use crate::commands::batch::{
    BatchItemError, BatchProcessor, FailureKind, classify_fetch_error, report_failures,
};
use crate::commands::storage::{StorageBackend, create_storage_backend};

pub struct FiguresOptions {
    pub pmcids: Vec<String>,
    pub output_dir: Option<PathBuf>,
    pub s3_path: Option<String>,
    pub s3_region: Option<String>,
    pub failed_output: Option<PathBuf>,
    pub timeout_seconds: Option<u64>,
    pub overwrite: bool,
}

pub async fn execute(options: FiguresOptions, ctx: &ClientContext<'_>) -> Result<()> {
    let storage = create_storage_backend(options.output_dir, options.s3_path, options.s3_region)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create storage backend: {}", e))?;

    let timeout = options.timeout_seconds.unwrap_or(180);
    let client = ctx.pmc_client_with_timeout(Some(timeout));

    let mut processor = BatchProcessor::new(options.pmcids.len())?;

    processor
        .run(&options.pmcids, async |multi_progress, pmcid| {
            process_article(
                &client,
                pmcid,
                storage.as_ref(),
                options.overwrite,
                multi_progress,
            )
            .await
        })
        .await;

    processor.finish();

    report_failures(
        processor.failures(),
        options.failed_output,
        storage.as_ref(),
    )
    .await
}

async fn process_article(
    client: &pubmed_client::PmcClient,
    pmcid: &str,
    storage: &dyn StorageBackend,
    overwrite: bool,
    multi_progress: &MultiProgress,
) -> Result<(), BatchItemError> {
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
    let temp_dir_handle = TempDir::new()
        .map_err(|e| BatchItemError::new(pmcid, FailureKind::Other, e.to_string()))?;
    let temp_dir = temp_dir_handle.path();

    // Create progress bar for downloading
    let download_pb = multi_progress.add(ProgressBar::new_spinner());
    download_pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .map_err(|e| BatchItemError::new(pmcid, FailureKind::Other, e.to_string()))?,
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
            let error_str = e.to_string();
            let timeout_seconds = client.get_cloud_client_config().timeout.as_secs();
            return Err(BatchItemError::new(
                pmcid,
                classify_fetch_error(&error_str, timeout_seconds),
                format!("Cloud download failed: {:#}", anyhow::anyhow!(e)),
            ));
        }
    };

    if figures.is_empty() {
        // Temporary directory will be automatically cleaned up when temp_dir_handle is dropped
        return Err(BatchItemError::new(
            pmcid,
            FailureKind::Empty,
            "No figures found in article",
        ));
    }

    // Now that we have figures, ensure the storage directory exists
    storage
        .ensure_directory(article_dir_str)
        .await
        .map_err(|e| {
            BatchItemError::new(
                pmcid,
                FailureKind::StorageError {
                    operation: "ensure_directory".to_string(),
                },
                format!("{:#}", e),
            )
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
            .map_err(|e| BatchItemError::new(pmcid, FailureKind::Other, e.to_string()))?
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
        BatchItemError::new(
            pmcid,
            FailureKind::StorageError {
                operation: "serialize_metadata".to_string(),
            },
            e.to_string(),
        )
    })?;

    storage
        .write_file(&json_storage_path, json_content.as_bytes())
        .await
        .map_err(|e| {
            BatchItemError::new(
                pmcid,
                FailureKind::StorageError {
                    operation: "write_metadata".to_string(),
                },
                format!("Failed to save metadata to {}: {:#}", json_storage_path, e),
            )
        })?;

    debug!(
        filename = %json_filename,
        location = %storage.get_full_path(&json_storage_path),
        "Saved metadata"
    );

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct FigureMetadata {
    pmcid: String,
    figureid: String,
    label: Option<String>,
    caption: Option<String>,
}
