use anyhow::Result;
use clap::Args;

use super::create_pubmed_client;
use pubmed_client::CitationQuery;

#[derive(Args, Debug)]
pub struct CitMatch {
    /// Citation strings in format: "journal|year|volume|first_page|author_name"
    /// Example: "proc natl acad sci u s a|1991|88|3248|mann bj"
    #[arg(required = true)]
    pub citations: Vec<String>,

    /// Output format (json, csv, or txt)
    #[arg(long, default_value = "json")]
    pub format: String,
}

impl CitMatch {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

        // Parse citation strings
        let queries: Vec<CitationQuery> = self
            .citations
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let parts: Vec<&str> = s.split('|').collect();
                if parts.len() < 5 {
                    tracing::error!(
                        "Invalid citation format: '{}'. Expected: journal|year|volume|first_page|author_name",
                        s
                    );
                    std::process::exit(1);
                }
                let key = if parts.len() >= 6 {
                    parts[5].to_string()
                } else {
                    format!("ref{}", i + 1)
                };
                CitationQuery::new(parts[0], parts[1], parts[2], parts[3], parts[4], &key)
            })
            .collect();

        tracing::info!(
            citation_count = queries.len(),
            "Matching citations to PMIDs"
        );

        let results = client.match_citations(&queries).await?;

        match self.format.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&results)?;
                // Using write! to stdout since println! is disallowed by clippy in this workspace
                use std::io::Write;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            "csv" => {
                use std::io::Write;
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
            "txt" => {
                use std::io::Write;
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
                tracing::error!(
                    "Unsupported format '{}'. Use 'json', 'csv', or 'txt'.",
                    self.format
                );
                std::process::exit(1);
            }
        }

        Ok(())
    }
}
