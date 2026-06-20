use std::io::Write;

use anyhow::{Result, bail};
use clap::Args;

use super::{ClientContext, OutputFormat};

#[derive(Args, Debug)]
pub struct Related {
    /// PubMed IDs to find related articles for
    #[arg(required = true)]
    pub pmids: Vec<u32>,

    /// Maximum number of related articles to display
    #[arg(short, long, default_value = "20")]
    pub max: usize,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

impl Related {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.pubmed_client();

        tracing::info!(pmids = ?self.pmids, "Finding related articles");

        let related = client.get_related_articles(&self.pmids).await?;

        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&related)?;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            OutputFormat::Text => {
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
                bail!(
                    "Unsupported format '{}' for related. Use 'text' or 'json'.",
                    self.format
                );
            }
        }

        Ok(())
    }
}
