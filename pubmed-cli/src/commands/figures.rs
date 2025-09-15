use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tracing::{debug, error, info, warn};

use crate::commands::create_pmc_client_with_timeout;
use crate::commands::storage::{create_storage_backend, StorageBackend};
use crate::Cli;

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
    let storage =
        create_storage_backend(options.output_dir, options.s3_path, options.s3_region).await?;

    // Initialize the PMC client with timeout (default to 180 seconds for figure extraction)
    let timeout = options.timeout_seconds.unwrap_or(180);
    let client = create_pmc_client_with_timeout(
        cli.api_key.as_deref(),
        cli.email.as_deref(),
        &cli.tool,
        Some(timeout),
    )?;

    let mut failed_pmcids = Vec::new();

    // Process each PMCID
    for pmcid in &options.pmcids {
        info!(pmcid = %pmcid, "Processing article");

        if let Err(e) = process_article(&client, pmcid, storage.as_ref(), options.overwrite).await {
            error!(pmcid = %pmcid, error = %e, "Failed to process article");
            failed_pmcids.push(pmcid.clone());
            continue;
        }

        debug!(pmcid = %pmcid, "Successfully processed article");
    }

    if !failed_pmcids.is_empty() {
        error!(
            failed_count = failed_pmcids.len(),
            failed_pmcids = ?failed_pmcids,
            "Failed to process some PMC IDs"
        );

        // Save failed PMC IDs to file if output path is specified
        if let Some(failed_path) = options.failed_output {
            match save_failed_pmcids(&failed_pmcids, &failed_path, storage.as_ref()).await {
                Ok(_) => {
                    info!(
                        path = %failed_path.display(),
                        count = failed_pmcids.len(),
                        "Saved failed PMC IDs to file"
                    );
                }
                Err(e) => {
                    error!(
                        path = %failed_path.display(),
                        error = %e,
                        "Failed to save failed PMC IDs to file"
                    );
                }
            }
        }
    }

    Ok(())
}

async fn process_article(
    client: &pubmed_client_rs::PmcClient,
    pmcid: &str,
    storage: &dyn StorageBackend,
    overwrite: bool,
) -> Result<()> {
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
    let temp_dir_handle = TempDir::new()?;
    let temp_dir = temp_dir_handle.path();

    let figures = match client.extract_figures_with_captions(pmcid, temp_dir).await {
        Ok(figures) => figures,
        Err(e) => {
            // Temporary directory will be automatically cleaned up when temp_dir_handle is dropped
            error!(
                pmcid = %pmcid,
                error = %e,
                "Failed to download TAR archive or extract figures"
            );
            return Err(anyhow::anyhow!(
                "Failed to download TAR archive for PMC ID {}: {}",
                pmcid,
                e
            ));
        }
    };

    if figures.is_empty() {
        // Temporary directory will be automatically cleaned up when temp_dir_handle is dropped
        return Err(anyhow::anyhow!("No figures found in article"));
    }

    // Now that we have figures, ensure the storage directory exists
    storage.ensure_directory(article_dir_str).await?;
    debug!(directory = %storage.get_full_path(article_dir_str), "Created storage directory");

    // Keep the temp directory from being auto-deleted
    let _temp_path = temp_dir_handle.keep();

    debug!(pmcid = %pmcid, figure_count = figures.len(), "Found figures");

    // Process each figure
    let mut figure_metadata = Vec::new();

    for (index, extracted_figure) in figures.iter().enumerate() {
        let figure_num = index + 1;
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
            if let Err(e) = storage.copy_file(actual_file_path, &storage_path).await {
                warn!(
                    error = %e,
                    source = %actual_file_path.display(),
                    target = %storage_path,
                    "Could not copy figure to storage"
                );
            } else {
                info!(
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

        figure_metadata.push(metadata);
    }

    // Save metadata as JSON to storage (always save metadata if we got this far)
    let json_content = serde_json::to_string_pretty(&figure_metadata)?;
    storage
        .write_file(&json_storage_path, json_content.as_bytes())
        .await?;

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

async fn save_failed_pmcids(
    failed_pmcids: &[String],
    path: &Path,
    storage: &dyn StorageBackend,
) -> Result<()> {
    // Join PMC IDs with newlines, one per line
    let content = failed_pmcids.join("\n");

    // Get just the filename from the path
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid failed output path"))?;

    // Write to storage
    storage.write_file(filename, content.as_bytes()).await?;

    Ok(())
}
