use anyhow::Result;
use clap::Args;

use super::create_pubmed_client;

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
    #[arg(long, default_value = "text")]
    pub format: String,
}

impl Info {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pubmed_client(api_key, email, tool)?;

        if let Some(ref database) = self.database {
            // Get info for a specific database
            tracing::info!(database = %database, "Getting database info");

            let db_info = client.get_database_info(database).await?;

            match self.format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&db_info)?;
                    use std::io::Write;
                    writeln!(std::io::stdout(), "{}", json)?;
                }
                "text" => {
                    use std::io::Write;
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
                    tracing::error!(
                        "Unsupported format '{}'. Use 'text' or 'json'.",
                        self.format
                    );
                    std::process::exit(1);
                }
            }
        } else {
            // List all databases
            tracing::info!("Listing all NCBI databases");

            let databases = client.get_database_list().await?;

            match self.format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&databases)?;
                    use std::io::Write;
                    writeln!(std::io::stdout(), "{}", json)?;
                }
                "text" => {
                    use std::io::Write;
                    let mut stdout = std::io::stdout();
                    writeln!(stdout, "Available NCBI databases ({}):", databases.len())?;
                    writeln!(stdout)?;
                    for db in &databases {
                        writeln!(stdout, "  {}", db)?;
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
        }

        Ok(())
    }
}
