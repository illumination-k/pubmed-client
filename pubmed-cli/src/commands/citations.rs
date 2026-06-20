use std::io::Write;

use anyhow::{Result, bail};
use clap::Args;

use super::{ClientContext, OutputFormat};

#[derive(Args, Debug)]
pub struct Citations {
    /// PubMed IDs to find citing articles for
    #[arg(required = true)]
    pub pmids: Vec<u32>,

    /// Maximum number of citing articles to display
    #[arg(short, long, default_value = "50")]
    pub max: usize,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

impl Citations {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.pubmed_client();

        tracing::info!(pmids = ?self.pmids, "Finding citing articles");

        let citations = client.get_citations(&self.pmids).await?;

        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&citations)?;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                writeln!(
                    stdout,
                    "Found {} citing articles in PubMed for PMIDs {:?}",
                    citations.citing_pmids.len(),
                    citations.source_pmids
                )?;
                writeln!(stdout)?;

                for (i, pmid) in citations.citing_pmids.iter().take(self.max).enumerate() {
                    writeln!(stdout, "  {}. PMID: {}", i + 1, pmid)?;
                }

                if citations.citing_pmids.len() > self.max {
                    writeln!(
                        stdout,
                        "\n  ... and {} more (use --max to show more)",
                        citations.citing_pmids.len() - self.max
                    )?;
                }

                writeln!(
                    stdout,
                    "\nNote: Counts reflect PubMed-indexed articles only."
                )?;
            }
            _ => {
                bail!(
                    "Unsupported format '{}' for citations. Use 'text' or 'json'.",
                    self.format
                );
            }
        }

        Ok(())
    }
}
