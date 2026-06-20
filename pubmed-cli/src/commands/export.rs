use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use pubmed_client::ExportFormat;

use super::{CitationFormat, ClientContext};
use crate::commands::storage::create_file_storage;

#[derive(Args, Debug)]
pub struct Export {
    /// PubMed IDs to export
    #[arg(required = true)]
    pub pmids: Vec<String>,

    /// Export format (bibtex, ris, csl-json, nbib)
    #[arg(short, long, value_enum, default_value_t = CitationFormat::Bibtex)]
    pub format: CitationFormat,

    /// Output file (default: stdout)
    #[arg(short, long, conflicts_with = "s3_path")]
    pub output: Option<PathBuf>,

    /// S3 path for the exported file (e.g., s3://bucket/prefix/citations.bib)
    #[arg(long, conflicts_with = "output")]
    pub s3_path: Option<String>,

    /// AWS region for S3 (optional, uses default AWS config if not specified)
    #[arg(long, requires = "s3_path")]
    pub s3_region: Option<String>,
}

impl Export {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.pubmed_client();

        tracing::info!(
            pmids_count = self.pmids.len(),
            format = %self.format,
            "Exporting citations"
        );

        let pmid_refs: Vec<&str> = self.pmids.iter().map(|s| s.as_str()).collect();
        let articles = client.fetch_articles(&pmid_refs).await?;

        if articles.is_empty() {
            tracing::warn!("No articles found for the given PMIDs");
            return Ok(());
        }

        let result = match self.format {
            CitationFormat::Bibtex => pubmed_client::export::articles_to_bibtex(&articles),
            CitationFormat::Ris => pubmed_client::export::articles_to_ris(&articles),
            CitationFormat::CslJson => {
                let json = pubmed_client::export::articles_to_csl_json(&articles);
                serde_json::to_string_pretty(&json)?
            }
            CitationFormat::Nbib => articles
                .iter()
                .map(|a| a.to_nbib())
                .collect::<Vec<_>>()
                .join("\n\n"),
        };

        // No destination given: write to stdout (default behavior).
        if self.output.is_none() && self.s3_path.is_none() {
            write!(std::io::stdout(), "{}", result)?;
            return Ok(());
        }

        let (storage, key) = create_file_storage(
            self.output.clone(),
            self.s3_path.clone(),
            self.s3_region.clone(),
            &self.default_filename(),
        )
        .await?;

        storage.write_file(&key, result.as_bytes()).await?;
        tracing::info!(
            path = %storage.get_full_path(&key),
            articles = articles.len(),
            "Exported {} articles",
            articles.len(),
        );

        Ok(())
    }

    fn default_filename(&self) -> String {
        let ext = match self.format {
            CitationFormat::Bibtex => "bib",
            CitationFormat::Ris => "ris",
            CitationFormat::CslJson => "json",
            CitationFormat::Nbib => "nbib",
        };
        format!("citations.{}", ext)
    }
}
