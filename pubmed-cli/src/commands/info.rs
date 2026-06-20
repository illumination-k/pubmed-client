use std::io::Write;

use anyhow::{Result, bail};
use clap::Args;

use super::{ClientContext, OutputFormat};

#[derive(Args, Debug)]
pub struct Info {
    /// Database name to get info for (omit to list all databases)
    pub database: Option<String>,

    /// Show searchable fields (when querying a specific database)
    #[arg(long)]
    pub fields: bool,

    /// Show cross-database links (when querying a specific database)
    #[arg(long)]
    pub links: bool,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

impl Info {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.pubmed_client();

        if let Some(ref database) = self.database {
            self.show_database_info(&client, database).await
        } else {
            self.list_databases(&client).await
        }
    }

    async fn show_database_info(
        &self,
        client: &pubmed_client::PubMedClient,
        database: &str,
    ) -> Result<()> {
        tracing::info!(database = %database, "Getting database info");

        let db_info = client.get_database_info(database).await?;

        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&db_info)?;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                writeln!(stdout, "Database: {} ({})", db_info.name, db_info.menu_name)?;
                writeln!(stdout, "Description: {}", db_info.description)?;
                if let Some(count) = db_info.count {
                    writeln!(stdout, "Records: {}", count)?;
                }
                if let Some(ref update) = db_info.last_update {
                    writeln!(stdout, "Last updated: {}", update)?;
                }

                if self.fields && !db_info.fields.is_empty() {
                    writeln!(stdout, "\nSearchable fields ({}):", db_info.fields.len())?;
                    for field in &db_info.fields {
                        if !field.is_hidden {
                            writeln!(
                                stdout,
                                "  [{}] {} - {}",
                                field.name, field.full_name, field.description
                            )?;
                        }
                    }
                }

                if self.links && !db_info.links.is_empty() {
                    writeln!(stdout, "\nCross-database links ({}):", db_info.links.len())?;
                    for link in &db_info.links {
                        writeln!(
                            stdout,
                            "  {} -> {} ({})",
                            link.name, link.target_db, link.description
                        )?;
                    }
                }
            }
            _ => {
                bail!(
                    "Unsupported format '{}' for info. Use 'text' or 'json'.",
                    self.format
                );
            }
        }

        Ok(())
    }

    async fn list_databases(&self, client: &pubmed_client::PubMedClient) -> Result<()> {
        tracing::info!("Listing all NCBI databases");

        let databases = client.get_database_list().await?;

        match self.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&databases)?;
                writeln!(std::io::stdout(), "{}", json)?;
            }
            OutputFormat::Text => {
                let mut stdout = std::io::stdout();
                writeln!(stdout, "Available NCBI databases ({}):", databases.len())?;
                writeln!(stdout)?;
                for db in &databases {
                    writeln!(stdout, "  {}", db)?;
                }
            }
            _ => {
                bail!(
                    "Unsupported format '{}' for info. Use 'text' or 'json'.",
                    self.format
                );
            }
        }

        Ok(())
    }
}
