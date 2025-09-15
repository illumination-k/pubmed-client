use anyhow::Result;
use clap::{Parser, Subcommand};
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
        /// Output directory for extracted figures (local storage)
        #[arg(short, long, conflicts_with = "s3_path")]
        output_dir: Option<PathBuf>,
        /// S3 path for extracted figures (e.g., s3://bucket/prefix)
        #[arg(long, conflicts_with = "output_dir")]
        s3_path: Option<String>,
        /// AWS region for S3 (optional, uses default AWS config if not specified)
        #[arg(long, requires = "s3_path")]
        s3_region: Option<String>,
        /// Path to save failed PMC IDs (if not specified, failures are logged only)
        #[arg(short, long)]
        failed_output: Option<PathBuf>,
        /// HTTP request timeout in seconds (default: 120)
        #[arg(short, long)]
        timeout: Option<u64>,
        /// Overwrite existing files (default: skip existing files)
        #[arg(long)]
        overwrite: bool,
    },
    /// Convert PMC articles to Markdown format
    Markdown(commands::markdown::Markdown),
    /// Convert PMID to PMCID
    #[command(name = "pmid-to-pmcid")]
    PmidToPmcid(Box<commands::convert::Convert>),
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
        .with_writer(std::io::stderr)
        .init();

    // Execute command
    match &cli.command {
        Commands::Search(cmd) => {
            let api_key = cli.api_key.as_deref();
            let email = cli.email.as_deref();
            let tool = &cli.tool;
            cmd.execute_with_config(api_key, email, tool).await
        }
        Commands::Figures {
            pmcids,
            output_dir,
            s3_path,
            s3_region,
            failed_output,
            timeout,
            overwrite,
        } => {
            let options = commands::figures::FiguresOptions {
                pmcids: pmcids.clone(),
                output_dir: output_dir.clone(),
                s3_path: s3_path.clone(),
                s3_region: s3_region.clone(),
                failed_output: failed_output.clone(),
                timeout_seconds: *timeout,
                overwrite: *overwrite,
            };
            commands::figures::execute(options, &cli).await
        }
        Commands::Markdown(cmd) => {
            let api_key = cli.api_key.as_deref();
            let email = cli.email.as_deref();
            let tool = &cli.tool;
            cmd.execute_with_config(api_key, email, tool).await
        }
        Commands::PmidToPmcid(cmd) => {
            let api_key = cli.api_key.as_deref();
            let email = cli.email.as_deref();
            let tool = &cli.tool;
            cmd.execute_with_config(api_key, email, tool).await
        }
    }
}
