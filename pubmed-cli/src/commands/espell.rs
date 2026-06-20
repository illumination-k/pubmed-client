use std::io::Write;

use anyhow::{Result, bail};
use clap::Args;

use super::{ClientContext, OutputFormat};

#[derive(Args, Debug)]
pub struct ESpell {
    /// Search term to spell-check
    #[arg(required = true)]
    pub term: String,

    /// NCBI database to check against (default: pubmed)
    #[arg(long, default_value = "pubmed")]
    pub db: String,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

impl ESpell {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.pubmed_client();

        tracing::info!(term = %self.term, db = %self.db, "Checking spelling");

        let result = client.spell_check_db(&self.term, &self.db).await?;

        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&result)?;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            OutputFormat::Text => {
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
                bail!(
                    "Unsupported format '{}' for spell-check. Use 'text' or 'json'.",
                    self.format
                );
            }
        }

        Ok(())
    }
}
