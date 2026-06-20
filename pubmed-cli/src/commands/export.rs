use std::io::Write;

use anyhow::Result;
use clap::Args;
use pubmed_client::ExportFormat;

use super::{CitationFormat, ClientContext};

#[derive(Args, Debug)]
pub struct Export {
    /// PubMed IDs to export
    #[arg(required = true)]
    pub pmids: Vec<String>,

    /// Export format (bibtex, ris, csl-json, nbib)
    #[arg(short, long, value_enum, default_value_t = CitationFormat::Bibtex)]
    pub format: CitationFormat,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
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

        if let Some(ref output_path) = self.output {
            std::fs::write(output_path, &result)?;
            tracing::info!(
                path = %output_path.display(),
                articles = articles.len(),
                "Exported {} articles to {}",
                articles.len(),
                output_path.display()
            );
        } else {
            write!(std::io::stdout(), "{}", result)?;
        }

        Ok(())
    }
}
