use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context as _, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::commands::ClientContext;
use crate::commands::batch::{
    BatchItemError, BatchProcessor, FailureKind, classify_fetch_error, report_failures,
};
use crate::commands::storage::create_file_storage;

pub struct MetadataOptions {
    pub pmcids: Vec<String>,
    pub output_file: Option<PathBuf>,
    pub s3_path: Option<String>,
    pub s3_region: Option<String>,
    pub failed_output: Option<PathBuf>,
    pub timeout_seconds: Option<u64>,
    pub append: bool,
}

pub async fn execute(options: MetadataOptions, ctx: &ClientContext<'_>) -> Result<()> {
    let (storage, output_key) = create_file_storage(
        options.output_file,
        options.s3_path,
        options.s3_region,
        "metadata.jsonl",
    )
    .await
    .context("Failed to create storage backend")?;

    let timeout = options.timeout_seconds.unwrap_or(60);
    let client = ctx.pmc_client_with_timeout(Some(timeout));

    let mut processor = BatchProcessor::new(options.pmcids.len())?;

    // Successful metadata lines are accumulated here, then written in one shot so
    // the same code path works for both local and S3 storage backends.
    let buffer: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    processor
        .run(&options.pmcids, async |multi_progress, pmcid| {
            let metadata = fetch_article_metadata(&client, pmcid, multi_progress).await?;
            let json_line = serde_json::to_string(&metadata)
                .map_err(|e| BatchItemError::new(pmcid, FailureKind::Other, e.to_string()))?;
            let mut buf = buffer.lock().unwrap_or_else(|e| e.into_inner());
            buf.extend_from_slice(json_line.as_bytes());
            buf.push(b'\n');
            Ok(())
        })
        .await;

    processor.finish();

    let mut content = buffer.into_inner().unwrap_or_else(|e| e.into_inner());

    // Append mode: prepend any existing content before writing the combined file.
    if options.append
        && let Some(mut existing) = storage.read_file(&output_key).await.unwrap_or(None)
    {
        if !existing.is_empty() && !existing.ends_with(b"\n") {
            existing.push(b'\n');
        }
        existing.append(&mut content);
        content = existing;
    }

    storage
        .write_file(&output_key, &content)
        .await
        .context("Failed to write metadata output")?;

    let succeeded = options.pmcids.len() - processor.failures().len();
    info!(
        path = %storage.get_full_path(&output_key),
        count = succeeded,
        "Saved metadata to JSONL file"
    );

    report_failures(
        processor.failures(),
        options.failed_output,
        storage.as_ref(),
    )
    .await
}

async fn fetch_article_metadata(
    client: &pubmed_client::PmcClient,
    pmcid: &str,
    multi_progress: &MultiProgress,
) -> Result<ArticleMetadata, BatchItemError> {
    // Create progress bar for downloading
    let download_pb = multi_progress.add(ProgressBar::new_spinner());
    download_pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .map_err(|e| BatchItemError::new(pmcid, FailureKind::Other, e.to_string()))?,
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
            let error_str = e.to_string();
            let timeout_seconds = client.get_pmc_config().timeout.as_secs();
            return Err(BatchItemError::new(
                pmcid,
                classify_fetch_error(&error_str, timeout_seconds),
                format!("Metadata fetch failed: {:#}", anyhow::anyhow!(e)),
            ));
        }
    };

    // Extract abstract from sections if available
    let abstract_content = full_text
        .sections()
        .iter()
        .find(|s| s.section_type.as_deref() == Some("abstract"))
        .map(|s| s.content.clone());

    // Create metadata structure
    let metadata = ArticleMetadata {
        pmcid: full_text.pmcid().to_string(),
        pmid: full_text.pmid().as_ref().map(|p| p.to_string()),
        doi: full_text.doi().map(str::to_string),
        title: full_text.title().unwrap_or("Untitled").to_string(),
        r#abstract: abstract_content,
        authors: full_text.authors().to_vec(),
        journal: Some(full_text.journal().clone()),
        publication_date: full_text.pub_dates().first().map(|d| {
            let mut s = String::new();
            if let Some(y) = d.year {
                s.push_str(&y.to_string());
            }
            if let Some(m) = d.month {
                s.push_str(&format!("-{:02}", m));
            }
            if let Some(day) = d.day {
                s.push_str(&format!("-{:02}", day));
            }
            s
        }),
        keywords: full_text.keywords().to_vec(),
        funding: full_text.funding().to_vec(),
        references: full_text.references().to_vec(),
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
    journal: Option<pubmed_client::pmc::JournalMeta>,
    publication_date: Option<String>,
    keywords: Vec<String>,
    funding: Vec<pubmed_client::pmc::FundingInfo>,
    references: Vec<pubmed_client::pmc::Reference>,
}
