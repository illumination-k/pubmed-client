use std::io::Write;

use anyhow::{Result, bail};
use clap::Args;

use super::{ClientContext, OutputFormat};
use pubmed_client::CitationQuery;

#[derive(Args, Debug)]
pub struct CitMatch {
    /// Citation strings in format: "journal|year|volume|first_page|author_name"
    /// Example: "proc natl acad sci u s a|1991|88|3248|mann bj"
    #[arg(required = true)]
    pub citations: Vec<String>,

    /// Output format (json, csv, or txt)
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,
}

impl CitMatch {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.pubmed_client();

        let queries: Vec<CitationQuery> = self
            .citations
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let parts: Vec<&str> = s.split('|').collect();
                if parts.len() < 5 {
                    bail!(
                        "Invalid citation format: '{}'. Expected: journal|year|volume|first_page|author_name",
                        s
                    );
                }
                let key = if parts.len() >= 6 {
                    parts[5].to_string()
                } else {
                    format!("ref{}", i + 1)
                };
                Ok(CitationQuery::new(parts[0], parts[1], parts[2], parts[3], parts[4], &key))
            })
            .collect::<Result<Vec<_>>>()?;

        tracing::info!(
            citation_count = queries.len(),
            "Matching citations to PMIDs"
        );

        let results = client.match_citations(&queries).await?;

        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&results)?;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            OutputFormat::Csv => {
                let mut stdout = std::io::stdout();
                writeln!(
                    stdout,
                    "key,journal,year,volume,first_page,author_name,pmid,status"
                )?;
                for m in &results.matches {
                    let status = match m.status {
                        pubmed_client::CitationMatchStatus::Found => "found",
                        pubmed_client::CitationMatchStatus::NotFound => "not_found",
                        pubmed_client::CitationMatchStatus::Ambiguous => "ambiguous",
                    };
                    writeln!(
                        stdout,
                        "{},{},{},{},{},{},{},{}",
                        m.key,
                        m.journal,
                        m.year,
                        m.volume,
                        m.first_page,
                        m.author_name,
                        m.pmid.as_deref().unwrap_or(""),
                        status
                    )?;
                }
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                for m in &results.matches {
                    let status = match m.status {
                        pubmed_client::CitationMatchStatus::Found => "Found",
                        pubmed_client::CitationMatchStatus::NotFound => "Not Found",
                        pubmed_client::CitationMatchStatus::Ambiguous => "Ambiguous",
                    };
                    if let Some(ref pmid) = m.pmid {
                        writeln!(stdout, "{}: PMID {} ({})", m.key, pmid, status)?;
                    } else {
                        writeln!(stdout, "{}: {} ({}, {})", m.key, status, m.journal, m.year)?;
                    }
                }
                writeln!(
                    stdout,
                    "\nTotal: {} matched out of {}",
                    results.found_count(),
                    results.matches.len()
                )?;
            }
            _ => {
                bail!(
                    "Unsupported format '{}' for citmatch. Use 'json', 'csv', or 'txt'.",
                    self.format
                );
            }
        }

        Ok(())
    }
}
