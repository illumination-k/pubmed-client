use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::fs;
use tracing::{error, info, warn};

use crate::commands::create_pmc_client;
use crate::Cli;

pub async fn execute(
    pmcids: Vec<String>,
    output_dir: PathBuf,
    failed_output: Option<PathBuf>,
    cli: &Cli,
) -> Result<()> {
    // Ensure output directory exists
    fs::create_dir_all(&output_dir).await?;

    // Initialize the PMC client
    let client = create_pmc_client(cli.api_key.as_deref(), cli.email.as_deref(), &cli.tool)?;

    let mut failed_pmcids = Vec::new();

    // Process each PMCID
    for pmcid in &pmcids {
        info!(pmcid = %pmcid, "Processing article");

        if let Err(e) = process_article(&client, pmcid, &output_dir).await {
            error!(pmcid = %pmcid, error = %e, "Failed to process article");
            failed_pmcids.push(pmcid.clone());
            continue;
        }

        info!(pmcid = %pmcid, "Successfully processed article");
    }

    if !failed_pmcids.is_empty() {
        error!(
            failed_count = failed_pmcids.len(),
            failed_pmcids = ?failed_pmcids,
            "Failed to process some PMC IDs"
        );

        // Save failed PMC IDs to file if output path is specified
        if let Some(failed_path) = failed_output {
            match save_failed_pmcids(&failed_pmcids, &failed_path).await {
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
    output_base: &Path,
) -> Result<()> {
    // Prepare output directory path for this article (but don't create it yet)
    let article_dir = output_base.join(pmcid);

    // First, try to extract figures to a temporary location
    // We'll use the final directory path, but only create it if extraction succeeds
    info!("Extracting figures and matching with captions");

    // Create a temporary directory for extraction using tempfile crate
    // This ensures automatic cleanup on drop and avoids conflicts
    let temp_dir_handle = TempDir::new_in(output_base)?;
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
        info!(pmcid = %pmcid, "No figures found in article");
        return Ok(());
    }

    // Now that we have figures, move temp directory to final location
    info!(directory = %article_dir.display(), "Creating output directory");
    if article_dir.exists() {
        fs::remove_dir_all(&article_dir).await?;
    }

    // Move the temporary directory to the final location
    // We need to persist the temp directory to prevent automatic cleanup
    let temp_path = temp_dir_handle.keep();
    fs::rename(&temp_path, &article_dir).await?;

    info!(pmcid = %pmcid, figure_count = figures.len(), "Found figures");

    // Process each figure
    let mut figure_metadata = Vec::new();

    for (index, extracted_figure) in figures.iter().enumerate() {
        let figure_num = index + 1;
        info!(
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
        // The extraction creates a PMC subdirectory inside the temp directory
        // So we need to find the file in the moved directory structure
        let extracted_path = Path::new(&extracted_figure.extracted_file_path);

        // Check if the file exists directly in article_dir (for flat structure)
        let filename = extracted_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

        let mut actual_file_path = article_dir.join(filename);

        // If not found, check in the PMC subdirectory (nested structure)
        if !actual_file_path.exists() {
            actual_file_path = article_dir.join(pmcid).join(filename);
        }

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
            let new_path = article_dir.join(&new_filename);

            if let Err(e) = fs::copy(&actual_file_path, &new_path).await {
                warn!(
                    error = %e,
                    source = %actual_file_path.display(),
                    target = %new_path.display(),
                    "Could not copy figure"
                );
            } else {
                info!(filename = %new_filename, "Saved figure");

                let caption_preview = if extracted_figure.figure.caption.len() > 80 {
                    format!("{}...", &extracted_figure.figure.caption[..80])
                } else {
                    extracted_figure.figure.caption.clone()
                };
                info!(caption = %caption_preview, "Figure caption");

                if let Some(dimensions) = extracted_figure.dimensions {
                    info!(
                        width = dimensions.0,
                        height = dimensions.1,
                        "Figure dimensions"
                    );
                }

                if let Some(size) = extracted_figure.file_size {
                    info!(size_kb = size / 1024, "Figure size");
                }
            }
        }

        figure_metadata.push(metadata);
    }

    // Save metadata as JSON
    let json_filename = format!("{}_figures_metadata.json", pmcid);
    let json_path = article_dir.join(&json_filename);

    let json_content = serde_json::to_string_pretty(&figure_metadata)?;
    fs::write(&json_path, json_content).await?;

    info!(filename = %json_filename, "Saved metadata");

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct FigureMetadata {
    pmcid: String,
    figureid: String,
    label: Option<String>,
    caption: String,
}

async fn save_failed_pmcids(failed_pmcids: &[String], path: &Path) -> Result<()> {
    // Join PMC IDs with newlines, one per line
    let content = failed_pmcids.join("\n");

    // Write to file
    fs::write(path, content).await?;

    Ok(())
}
