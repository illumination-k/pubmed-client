use anyhow::Result;
use clap::Args;
use pubmed_client::ExportFormat;

use super::create_pubmed_client;

#[derive(Args, Debug)]
pub struct Export {
    /// PubMed IDs to export
    #[arg(required = true)]
    pub pmids: Vec<String>,

    /// Export format (bibtex, ris, csl-json, nbib)
    #[arg(short, long, default_value = "bibtex")]
    pub format: String,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,
}

impl Export {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

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

        let result = match self.format.as_str() {
            "bibtex" | "bib" => pubmed_client::export::articles_to_bibtex(&articles),
            "ris" => pubmed_client::export::articles_to_ris(&articles),
            "csl-json" | "csl" => {
                let json = pubmed_client::export::articles_to_csl_json(&articles);
                serde_json::to_string_pretty(&json)?
            }
            "nbib" | "medline" => articles
                .iter()
                .map(|a| a.to_nbib())
                .collect::<Vec<_>>()
                .join("\n\n"),
            _ => {
                tracing::error!(
                    "Unsupported format '{}'. Use 'bibtex', 'ris', 'csl-json', or 'nbib'.",
                    self.format
                );
                std::process::exit(1);
            }
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
            use std::io::Write;
            write!(std::io::stdout(), "{}", result)?;
        }

        Ok(())
    }
}
