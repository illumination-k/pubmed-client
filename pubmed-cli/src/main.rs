use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod commands;

use commands::ClientContext;

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

impl Cli {
    fn client_context(&self) -> ClientContext<'_> {
        ClientContext {
            api_key: self.api_key.as_deref(),
            email: self.email.as_deref(),
            tool: &self.tool,
        }
    }
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
        /// Number of articles to download concurrently (default: 4)
        #[arg(short = 'j', long)]
        concurrency: Option<usize>,
        /// Overwrite existing files (default: skip existing files)
        #[arg(long)]
        overwrite: bool,
    },
    /// Convert PMC articles to Markdown format
    Markdown(commands::markdown::Markdown),
    /// Extract metadata from PMC articles and save as JSONL
    Metadata {
        /// PMC ID(s) to process (e.g., PMC7906746 or 7906746)
        pmcids: Vec<String>,
        /// Output JSONL file path (default: metadata.jsonl, local storage)
        #[arg(short, long, conflicts_with = "s3_path")]
        output: Option<PathBuf>,
        /// S3 object path for the JSONL output (e.g., s3://bucket/prefix/metadata.jsonl)
        #[arg(long, conflicts_with = "output")]
        s3_path: Option<String>,
        /// AWS region for S3 (optional, uses default AWS config if not specified)
        #[arg(long, requires = "s3_path")]
        s3_region: Option<String>,
        /// Path to save failed PMC IDs (if not specified, failures are logged only)
        #[arg(short, long)]
        failed_output: Option<PathBuf>,
        /// HTTP request timeout in seconds (default: 60)
        #[arg(short, long)]
        timeout: Option<u64>,
        /// Number of articles to fetch concurrently (default: 4)
        #[arg(short = 'j', long)]
        concurrency: Option<usize>,
        /// Append to existing file instead of overwriting
        #[arg(short, long)]
        append: bool,
    },
    /// Convert PMID to PMCID
    #[command(name = "pmid-to-pmcid")]
    PmidToPmcid(Box<commands::convert::Convert>),
    /// Match citations to PMIDs (journal|year|volume|page|author format)
    #[command(name = "citmatch")]
    CitMatch(commands::citmatch::CitMatch),
    /// Query all NCBI databases for record counts
    #[command(name = "gquery")]
    GQuery(commands::gquery::GQuery),
    /// Check spelling of a search term using the ESpell API
    #[command(name = "spell-check")]
    SpellCheck(commands::espell::ESpell),
    /// Find related articles for given PMIDs
    Related(commands::related::Related),
    /// Find articles that cite the given PMIDs
    Citations(commands::citations::Citations),
    /// List NCBI databases or get detailed database information
    Info(commands::info::Info),
    /// Export article citations in various formats (BibTeX, RIS, CSL-JSON, NBIB)
    Export(commands::export::Export),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };

    let indicatif_layer = IndicatifLayer::new();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .without_time()
                .with_writer(indicatif_layer.get_stderr_writer()),
        )
        .with(indicatif_layer)
        .with(tracing_subscriber::EnvFilter::new(filter))
        .init();

    let ctx = cli.client_context();

    match &cli.command {
        Commands::Search(cmd) => cmd.execute(&ctx).await,
        Commands::Figures {
            pmcids,
            output_dir,
            s3_path,
            s3_region,
            failed_output,
            timeout,
            concurrency,
            overwrite,
        } => {
            let options = commands::figures::FiguresOptions {
                pmcids: pmcids.clone(),
                output_dir: output_dir.clone(),
                s3_path: s3_path.clone(),
                s3_region: s3_region.clone(),
                failed_output: failed_output.clone(),
                timeout_seconds: *timeout,
                concurrency: *concurrency,
                overwrite: *overwrite,
            };
            commands::figures::execute(options, &ctx).await
        }
        Commands::Markdown(cmd) => cmd.execute(&ctx).await,
        Commands::Metadata {
            pmcids,
            output,
            s3_path,
            s3_region,
            failed_output,
            timeout,
            concurrency,
            append,
        } => {
            let options = commands::metadata::MetadataOptions {
                pmcids: pmcids.clone(),
                output_file: output.clone(),
                s3_path: s3_path.clone(),
                s3_region: s3_region.clone(),
                failed_output: failed_output.clone(),
                timeout_seconds: *timeout,
                concurrency: *concurrency,
                append: *append,
            };
            commands::metadata::execute(options, &ctx).await
        }
        Commands::PmidToPmcid(cmd) => cmd.execute(&ctx).await,
        Commands::CitMatch(cmd) => cmd.execute(&ctx).await,
        Commands::GQuery(cmd) => cmd.execute(&ctx).await,
        Commands::SpellCheck(cmd) => cmd.execute(&ctx).await,
        Commands::Related(cmd) => cmd.execute(&ctx).await,
        Commands::Citations(cmd) => cmd.execute(&ctx).await,
        Commands::Info(cmd) => cmd.execute(&ctx).await,
        Commands::Export(cmd) => cmd.execute(&ctx).await,
    }
}
