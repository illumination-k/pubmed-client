use anyhow::Result;
use clap::Args;

use super::create_pubmed_client;

#[derive(Args, Debug)]
pub struct ESpell {
    /// Search term to spell-check
    #[arg(required = true)]
    pub term: String,

    /// NCBI database to check against (default: pubmed)
    #[arg(long, default_value = "pubmed")]
    pub db: String,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    pub format: String,
}

impl ESpell {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

        tracing::info!(term = %self.term, db = %self.db, "Checking spelling");

        let result = client.spell_check_db(&self.term, &self.db).await?;

        match self.format.as_str() {
            "json" => {
                let json = serde_json::to_string_pretty(&result)?;
                use std::io::Write;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            "text" => {
                use std::io::Write;
                let mut stdout = std::io::stdout();
                writeln!(stdout, "Database: {}", result.database)?;
                writeln!(stdout, "Original:  \"{}\"", result.query)?;
                writeln!(stdout, "Corrected: \"{}\"", result.corrected_query)?;

                if result.has_corrections() {
                    let replacements = result.replacements();
                    writeln!(stdout)?;
                    writeln!(stdout, "Corrections: {}", replacements.join(", "))?;
                } else {
                    writeln!(stdout)?;
                    writeln!(stdout, "No spelling corrections needed.")?;
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
