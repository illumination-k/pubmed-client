use anyhow::{Context as _, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info};

use crate::commands::create_pmc_client_with_timeout;
use crate::Cli;

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("Failed to fetch metadata for PMC ID {pmcid}: {source}")]
    FetchFailed {
        pmcid: String,
        #[source]
        source: anyhow::Error,
    },

    #[error("Article not found: {pmcid}")]
    ArticleNotFound { pmcid: String },

    #[error("Network timeout while processing {pmcid} (timeout: {timeout_seconds}s)")]
    NetworkTimeout { pmcid: String, timeout_seconds: u64 },

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
    FetchFailed {
        message: String,
        is_timeout: bool,
        is_network_error: bool,
    },
    ArticleNotFound,
    NetworkTimeout {
        timeout_seconds: u64,
    },
    Other(String),
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchFailed {
                message,
                is_timeout,
                is_network_error,
            } => {
                write!(
                    f,
                    "Metadata fetch failed: {}{}{}",
                    message,
                    if *is_timeout { " (timeout)" } else { "" },
                    if *is_network_error {
                        " (network error)"
                    } else {
                        ""
                    }
                )
            }
            Self::ArticleNotFound => write!(f, "Article not found"),
            Self::NetworkTimeout { timeout_seconds } => {
                write!(f, "Network timeout after {} seconds", timeout_seconds)
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

pub struct MetadataOptions {
    pub pmcids: Vec<String>,
    pub output_file: Option<PathBuf>,
    pub failed_output: Option<PathBuf>,
    pub timeout_seconds: Option<u64>,
    pub append: bool,
}

pub async fn execute(options: MetadataOptions, cli: &Cli) -> Result<()> {
    // Determine output file path
    let output_path = options
        .output_file
        .unwrap_or_else(|| PathBuf::from("metadata.jsonl"));

    // Initialize the PMC client with timeout (default to 60 seconds for metadata extraction)
    let timeout = options.timeout_seconds.unwrap_or(60);
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

    // Open output file for writing (append mode if specified)
    let mut output_file = if options.append && output_path.exists() {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&output_path)
            .await
            .context("Failed to open output file for appending")?
    } else {
        tokio::fs::File::create(&output_path)
            .await
            .context("Failed to create output file")?
    };

    // Process each PMCID
    for pmcid in &options.pmcids {
        main_pb.set_message(format!("Processing {}", pmcid));
        debug!(pmcid = %pmcid, "Processing article metadata");

        match fetch_article_metadata(&client, pmcid, &multi_progress).await {
            Ok(metadata) => {
                // Write metadata as a single line of JSON
                let json_line =
                    serde_json::to_string(&metadata).context("Failed to serialize metadata")?;

                output_file
                    .write_all(json_line.as_bytes())
                    .await
                    .context("Failed to write metadata to file")?;
                output_file
                    .write_all(b"\n")
                    .await
                    .context("Failed to write newline")?;

                debug!(pmcid = %pmcid, "Successfully processed article metadata");
                main_pb.set_message(format!("Completed {}", pmcid));
            }
            Err(e) => {
                error!(pmcid = %pmcid, error = %e, "Failed to process article metadata");

                // Determine the failure reason based on the error
                let (reason, error_details) = categorize_failure_from_metadata_error(&e, pmcid);

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

    // Ensure all data is written
    output_file
        .flush()
        .await
        .context("Failed to flush output file")?;

    main_pb.finish_with_message(format!(
        "Processed {} articles ({} failed)",
        total_pmcids,
        failed_pmcids.len()
    ));

    info!(
        path = %output_path.display(),
        count = options.pmcids.len() - failed_pmcids.len(),
        "Saved metadata to JSONL file"
    );

    if !failed_pmcids.is_empty() {
        // Save failed PMC IDs to file if output path is specified
        if let Some(failed_path) = options.failed_output {
            match save_failed_pmcids_json(&failed_pmcids, &failed_path).await {
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

async fn fetch_article_metadata(
    client: &pubmed_client::PmcClient,
    pmcid: &str,
    multi_progress: &MultiProgress,
) -> Result<ArticleMetadata, MetadataError> {
    // Create progress bar for downloading
    let download_pb = multi_progress.add(ProgressBar::new_spinner());
    download_pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .map_err(|e| MetadataError::Other {
                pmcid: pmcid.to_string(),
                source: anyhow::anyhow!(e),
            })?,
    );
    download_pb.enable_steady_tick(Duration::from_millis(100));
    download_pb.set_message(format!("Fetching metadata for {}", pmcid));

    // Fetch the full text metadata
    let full_text = match client.fetch_full_text(pmcid).await {
        Ok(text) => {
            download_pb.finish_with_message(format!("Fetched metadata for {}", pmcid));
            text
        }
        Err(e) => {
            download_pb.finish_with_message(format!("Failed to fetch {}", pmcid));
            error!(
                pmcid = %pmcid,
                error = %e,
                "Failed to fetch PMC article metadata"
            );

            // Check if it's a timeout error
            let error_str = e.to_string();
            if error_str.contains("timeout") || error_str.contains("timed out") {
                return Err(MetadataError::NetworkTimeout {
                    pmcid: pmcid.to_string(),
                    timeout_seconds: client.get_pmc_config().timeout.as_secs(),
                });
            }

            // Check if article not found
            if error_str.contains("not found") || error_str.contains("404") {
                return Err(MetadataError::ArticleNotFound {
                    pmcid: pmcid.to_string(),
                });
            }

            return Err(MetadataError::FetchFailed {
                pmcid: pmcid.to_string(),
                source: anyhow::anyhow!(e),
            });
        }
    };

    // Extract abstract from sections if available
    let abstract_content = full_text
        .sections
        .iter()
        .find(|s| s.section_type == "abstract")
        .map(|s| s.content.clone());

    // Create metadata structure
    let metadata = ArticleMetadata {
        pmcid: full_text.pmcid.clone(),
        pmid: full_text.pmid.clone(),
        doi: full_text.doi.clone(),
        title: full_text.title.clone(),
        r#abstract: abstract_content,
        authors: full_text.authors.clone(),
        journal: Some(full_text.journal.clone()),
        publication_date: Some(full_text.pub_date.clone()),
        keywords: full_text.keywords.clone(),
        funding: full_text.funding.clone(),
        references: full_text.references.clone(),
    };

    Ok(metadata)
}

#[derive(Serialize, Deserialize)]
struct ArticleMetadata {
    pmcid: String,
    pmid: Option<String>,
    doi: Option<String>,
    title: String,
    r#abstract: Option<String>,
    authors: Vec<pubmed_client::pmc::Author>,
    journal: Option<pubmed_client::pmc::JournalInfo>,
    publication_date: Option<String>,
    keywords: Vec<String>,
    funding: Vec<pubmed_client::pmc::FundingInfo>,
    references: Vec<pubmed_client::pmc::Reference>,
}

fn categorize_failure_from_metadata_error(
    error: &MetadataError,
    _pmcid: &str,
) -> (FailureReason, String) {
    let error_str = error.to_string();
    let error_chain = format!("{:#}", error); // Include full error chain for debugging

    let reason = match error {
        MetadataError::FetchFailed { .. } => {
            let is_timeout = error_str.contains("timeout") || error_str.contains("timed out");
            let is_network = error_str.contains("network") || error_str.contains("connection");
            FailureReason::FetchFailed {
                message: error_str.clone(),
                is_timeout,
                is_network_error: is_network,
            }
        }
        MetadataError::ArticleNotFound { .. } => FailureReason::ArticleNotFound,
        MetadataError::NetworkTimeout {
            timeout_seconds, ..
        } => FailureReason::NetworkTimeout {
            timeout_seconds: *timeout_seconds,
        },
        MetadataError::Other { .. } => FailureReason::Other(error_str.clone()),
    };

    (reason, error_chain)
}

async fn save_failed_pmcids_json(failed_pmcids: &[FailedPmcId], path: &Path) -> Result<()> {
    // Convert to JSON with pretty formatting
    let json_content = serde_json::to_string_pretty(failed_pmcids)?;

    // Write to file
    tokio::fs::write(path, json_content)
        .await
        .context("Failed to write failed PMC IDs to file")?;

    Ok(())
}
