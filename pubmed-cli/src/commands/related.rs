use anyhow::Result;
use clap::Args;

use super::create_pubmed_client;

#[derive(Args, Debug)]
pub struct Related {
    /// PubMed IDs to find related articles for
    #[arg(required = true)]
    pub pmids: Vec<u32>,

    /// Maximum number of related articles to display
    #[arg(short, long, default_value = "20")]
    pub max: usize,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: String,
}

impl Related {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

        tracing::info!(pmids = ?self.pmids, "Finding related articles");

        let related = client.get_related_articles(&self.pmids).await?;

        match self.format.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&related)?;
                use std::io::Write;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            "text" => {
                use std::io::Write;
                let mut stdout = std::io::stdout();
                writeln!(
                    stdout,
                    "Found {} related articles for PMIDs {:?}",
                    related.related_pmids.len(),
                    related.source_pmids
                )?;
                writeln!(stdout)?;

                for (i, pmid) in related.related_pmids.iter().take(self.max).enumerate() {
                    writeln!(stdout, "  {}. PMID: {}", i + 1, pmid)?;
                }

                if related.related_pmids.len() > self.max {
                    writeln!(
                        stdout,
                        "\n  ... and {} more (use --max to show more)",
                        related.related_pmids.len() - self.max
                    )?;
                }
            }
            _ => {
                tracing::error!(
                    "Unsupported format '{}'. Use 'text' or 'json'.",
                    self.format
                );
                std::process::exit(1);
            }
        }

        Ok(())
    }
}
