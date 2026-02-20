use anyhow::Result;
use clap::Args;

use super::create_pubmed_client;

#[derive(Args, Debug)]
pub struct GQuery {
    /// Search term to query across all NCBI databases
    #[arg(required = true)]
    pub term: String,

    /// Output format (json, table, or csv)
    #[arg(long, default_value = "table")]
    pub format: String,

    /// Only show databases with matching records (count > 0)
    #[arg(long)]
    pub non_zero: bool,
}

impl GQuery {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

        tracing::info!(term = %self.term, "Querying all NCBI databases");

        let results = client.global_query(&self.term).await?;

        let display_results: Vec<_> = if self.non_zero {
            results.results.iter().filter(|r| r.count > 0).collect()
        } else {
            results.results.iter().collect()
        };

        match self.format.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&results)?;
                use std::io::Write;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            "table" => {
                use std::io::Write;
                let mut stdout = std::io::stdout();
                writeln!(stdout, "Query: \"{}\"", results.term)?;
                writeln!(stdout)?;

                // Find max widths for formatting
                let max_name_len = display_results
                    .iter()
                    .map(|r| r.menu_name.len())
                    .max()
                    .unwrap_or(10)
                    .max(10);
                let max_count_len = display_results
                    .iter()
                    .map(|r| r.count.to_string().len())
                    .max()
                    .unwrap_or(5)
                    .max(5);

                writeln!(
                    stdout,
                    "{:<width_name$}  {:>width_count$}  Status",
                    "Database",
                    "Count",
                    width_name = max_name_len,
                    width_count = max_count_len
                )?;
                writeln!(
                    stdout,
                    "{:-<width_name$}  {:-<width_count$}  {:-<10}",
                    "",
                    "",
                    "",
                    width_name = max_name_len,
                    width_count = max_count_len
                )?;

                for db in &display_results {
                    writeln!(
                        stdout,
                        "{:<width_name$}  {:>width_count$}  {}",
                        db.menu_name,
                        db.count,
                        db.status,
                        width_name = max_name_len,
                        width_count = max_count_len
                    )?;
                }

                writeln!(stdout)?;
                writeln!(
                    stdout,
                    "Total databases: {} (showing {})",
                    results.results.len(),
                    display_results.len()
                )?;
            }
            "csv" => {
                use std::io::Write;
                let mut stdout = std::io::stdout();
                writeln!(stdout, "db_name,menu_name,count,status")?;
                for db in &display_results {
                    writeln!(
                        stdout,
                        "{},{},{},{}",
                        db.db_name, db.menu_name, db.count, db.status
                    )?;
                }
            }
            _ => {
                tracing::error!(
                    "Unsupported format '{}'. Use 'json', 'table', or 'csv'.",
                    self.format
                );
                std::process::exit(1);
            }
        }

        Ok(())
    }
}
