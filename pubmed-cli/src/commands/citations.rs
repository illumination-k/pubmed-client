use anyhow::Result;
use clap::Args;

use super::create_pubmed_client;

#[derive(Args, Debug)]
pub struct Citations {
    /// PubMed IDs to find citing articles for
    #[arg(required = true)]
    pub pmids: Vec<u32>,

    /// Maximum number of citing articles to display
    #[arg(short, long, default_value = "50")]
    pub max: usize,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: String,
}

impl Citations {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

        tracing::info!(pmids = ?self.pmids, "Finding citing articles");

        let citations = client.get_citations(&self.pmids).await?;

        match self.format.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&citations)?;
                use std::io::Write;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            "text" => {
                use std::io::Write;
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
