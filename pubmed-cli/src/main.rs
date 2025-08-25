use anyhow::Result;
use clap::{Parser, Subcommand};
use pubmed_client_rs::{ClientConfig, PubMedClient};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(
    name = "pubmed-cli",
    about = "Command-line interface for PubMed and PMC APIs",
    long_about = "A CLI tool for biomedical research articles from PubMed and PMC databases"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// API key for NCBI E-utilities (increases rate limit)
    #[arg(long, env = "NCBI_API_KEY", global = true)]
    api_key: Option<String>,

    /// Email for NCBI requests (recommended)
    #[arg(long, env = "NCBI_EMAIL", global = true)]
    email: Option<String>,

    /// Tool name for NCBI requests
    #[arg(long, env = "NCBI_TOOL", default_value = "pubmed-cli", global = true)]
    tool: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Search PubMed articles with advanced filtering
    Search(Box<commands::search::Search>),
    /// Extract figures from PMC articles
    Figures {
        /// PMC ID(s) to process (e.g., PMC7906746 or 7906746)
        pmcids: Vec<String>,
        /// Output directory for extracted figures
        #[arg(short, long, default_value = "./extracted_figures")]
        output_dir: PathBuf,
    },
    /// Convert PMC articles to Markdown format
    Markdown(commands::markdown::Markdown),
    /// Convert PMID to PMCID
    #[command(name = "pmid-to-pmcid")]
    PmidToPmcid {
        /// PMID(s) to convert to PMCID
        #[arg(required = true)]
        pmids: Vec<String>,

        /// Output format (json or csv)
        #[arg(long, default_value = "json")]
        format: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();

    // Log startup
    tracing::info!("PMC Tool started");

    // Execute command
    match &cli.command {
        Commands::Search(cmd) => {
            let api_key = cli.api_key.as_deref();
            let email = cli.email.as_deref();
            let tool = &cli.tool;
            cmd.execute_with_config(api_key, email, tool).await
        }
        Commands::Figures { pmcids, output_dir } => {
            commands::figures::execute(pmcids.clone(), output_dir.clone(), &cli).await
        }
        Commands::Markdown(cmd) => {
            let api_key = cli.api_key.as_deref();
            let email = cli.email.as_deref();
            let tool = &cli.tool;
            cmd.execute_with_config(api_key, email, tool).await
        }
        Commands::PmidToPmcid { pmids, format } => execute_pmid_to_pmcid(pmids, format, &cli).await,
    }
}

async fn execute_pmid_to_pmcid(pmids: &[String], format: &str, cli: &Cli) -> Result<()> {
    // Parse PMIDs from strings to u32
    let parsed_pmids: Result<Vec<u32>, _> = pmids.iter().map(|pmid| pmid.parse::<u32>()).collect();

    let parsed_pmids = match parsed_pmids {
        Ok(pmids) => pmids,
        Err(e) => {
            eprintln!(
                "Error: Invalid PMID format. PMIDs must be numeric. Error: {}",
                e
            );
            std::process::exit(1);
        }
    };

    // Create client with configuration
    let mut config = ClientConfig::new();

    if let Some(api_key) = &cli.api_key {
        config = config.with_api_key(api_key);
    }

    if let Some(email) = &cli.email {
        config = config.with_email(email);
    }

    config = config.with_tool(&cli.tool);

    let client = PubMedClient::with_config(config);

    // Get PMC links
    let pmc_links = client.get_pmc_links(&parsed_pmids).await?;

    match format {
        "json" => {
            // Create a more user-friendly JSON output
            let mut result = serde_json::Map::new();
            let mut conversions = Vec::new();

            // Add all source PMIDs with their conversion status
            for pmid in &pmc_links.source_pmids {
                let mut conversion = serde_json::Map::new();
                conversion.insert(
                    "pmid".to_string(),
                    serde_json::Value::Number((*pmid).into()),
                );
                conversion.insert("pmcid".to_string(), serde_json::Value::Null);
                conversions.push(serde_json::Value::Object(conversion));
            }

            // If we have PMCIDs, update the conversions
            // Note: This is a simplified approach - the actual PMID->PMCID mapping
            // requires parsing the ELink response more carefully
            if !pmc_links.pmc_ids.is_empty() {
                result.insert("note".to_string(),
                    serde_json::Value::String("PMCIDs found but mapping to specific PMIDs requires detailed ELink parsing".to_string()));
                result.insert(
                    "available_pmcids".to_string(),
                    serde_json::Value::Array(
                        pmc_links
                            .pmc_ids
                            .iter()
                            .map(|id| serde_json::Value::String(id.clone()))
                            .collect(),
                    ),
                );
            }

            result.insert(
                "conversions".to_string(),
                serde_json::Value::Array(conversions),
            );

            let json_output = serde_json::to_string_pretty(&result)?;
            println!("{}", json_output);
        }
        "csv" => {
            println!("PMID,PMCID_Available,PMCIDs_Found");
            for pmid in &pmc_links.source_pmids {
                let has_pmc = if pmc_links.pmc_ids.is_empty() {
                    "false"
                } else {
                    "true"
                };
                let pmcids_str = if pmc_links.pmc_ids.is_empty() {
                    "".to_string()
                } else {
                    pmc_links.pmc_ids.join(";")
                };
                println!("{},{},{}", pmid, has_pmc, pmcids_str);
            }
        }
        _ => {
            eprintln!(
                "Error: Unsupported format '{}'. Use 'json' or 'csv'.",
                format
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
