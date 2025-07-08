use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{error, info, warn};

use crate::commands::create_pmc_client;
use crate::Cli;

pub async fn execute(pmcids: Vec<String>, output_dir: PathBuf, cli: &Cli) -> Result<()> {
    // Initialize the PMC client
    let client = create_pmc_client(cli.api_key.as_deref(), cli.email.as_deref(), &cli.tool)?;

    // Process each PMCID
    for pmcid in &pmcids {
        info!(pmcid = %pmcid, "Processing article");

        if let Err(e) = process_article(&client, pmcid, &output_dir).await {
            error!(pmcid = %pmcid, error = %e, "Error processing article");
            continue;
        }

        info!(pmcid = %pmcid, "Successfully processed article");
    }

    Ok(())
}

async fn process_article(
    client: &pubmed_client_rs::PmcClient,
    pmcid: &str,
    output_base: &Path,
) -> Result<()> {
    // Create output directory for this article
    let article_dir = output_base.join(pmcid);

    info!(directory = %article_dir.display(), "Creating output directory");
    fs::create_dir_all(&article_dir).await?;

    // Extract figures with captions
    info!("Extracting figures and matching with captions");
    let figures = match client
        .extract_figures_with_captions(pmcid, &article_dir)
        .await
    {
        Ok(figures) => figures,
        Err(e) => {
            warn!(pmcid = %pmcid, error = %e, "Could not extract figures");
            return Ok(()); // Continue with other articles
        }
    };

    if figures.is_empty() {
        info!(pmcid = %pmcid, "No figures found");
        return Ok(());
    }

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
            figure_id: extracted_figure.figure.id.clone(),
            label: extracted_figure.figure.label.clone(),
            caption: extracted_figure.figure.caption.clone(),
            alt_text: extracted_figure.figure.alt_text.clone(),
            fig_type: extracted_figure.figure.fig_type.clone(),
            original_file_path: extracted_figure.extracted_file_path.clone(),
            file_size_bytes: extracted_figure.file_size,
            dimensions: extracted_figure.dimensions,
            extracted_at: chrono::Utc::now().to_rfc3339(),
        };

        // Copy figure to a standardized filename
        let original_path = Path::new(&extracted_figure.extracted_file_path);
        if let (Some(extension), Some(_filename)) =
            (original_path.extension(), original_path.file_stem())
        {
            let new_filename = format!(
                "{}_{}.{}",
                pmcid,
                extracted_figure.figure.id,
                extension.to_string_lossy()
            );
            let new_path = article_dir.join(&new_filename);

            if let Err(e) = fs::copy(&extracted_figure.extracted_file_path, &new_path).await {
                warn!(error = %e, "Could not copy figure");
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

    // Create a summary report
    create_summary_report(&article_dir, pmcid, &figure_metadata).await?;

    Ok(())
}

async fn create_summary_report(
    output_dir: &Path,
    pmcid: &str,
    figures: &[FigureMetadata],
) -> Result<()> {
    let report_path = output_dir.join(format!("{}_summary.txt", pmcid));

    let mut report = String::new();
    report.push_str(&format!("Figure Extraction Summary for {}\n", pmcid));
    report.push_str(&"=".repeat(50));
    report.push_str("\n\n");

    report.push_str(&format!("📊 Total figures extracted: {}\n", figures.len()));
    report.push_str(&format!(
        "📅 Extraction date: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    for (i, figure) in figures.iter().enumerate() {
        report.push_str(&format!("{}. Figure: {}\n", i + 1, figure.figure_id));
        if let Some(label) = &figure.label {
            report.push_str(&format!("   Label: {}\n", label));
        }
        report.push_str(&format!(
            "   Caption: {}\n",
            if figure.caption.len() > 100 {
                format!("{}...", &figure.caption[..100])
            } else {
                figure.caption.clone()
            }
        ));

        if let Some(dimensions) = figure.dimensions {
            report.push_str(&format!(
                "   Dimensions: {}x{}\n",
                dimensions.0, dimensions.1
            ));
        }

        if let Some(size) = figure.file_size_bytes {
            report.push_str(&format!("   Size: {} KB\n", size / 1024));
        }

        report.push('\n');
    }

    fs::write(&report_path, report).await?;
    let summary_filename = format!("{}_summary.txt", pmcid);
    info!(filename = %summary_filename, "Created summary report");

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct FigureMetadata {
    pmcid: String,
    figure_id: String,
    label: Option<String>,
    caption: String,
    alt_text: Option<String>,
    fig_type: Option<String>,
    original_file_path: String,
    file_size_bytes: Option<u64>,
    dimensions: Option<(u32, u32)>,
    extracted_at: String,
}
