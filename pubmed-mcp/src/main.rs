use anyhow::Result;
use clap::{Parser, ValueEnum};
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
    transport::stdio, ServerHandler, ServiceExt,
};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::info;

mod tools;
use tools::PubMedServer;

#[derive(Parser)]
#[command(name = "pubmed-mcp", about = "PubMed MCP Server")]
struct Args {
    /// HTTP port to listen on (if not set, uses stdio)
    #[arg(short, long)]
    port: Option<u16>,

    /// Tools to enable, comma-separated (default: all).
    /// Possible values: search, markdown, citmatch, gquery, espell, summary
    #[arg(short, long, value_delimiter = ',', value_enum)]
    tools: Vec<ToolName>,
}

#[derive(Clone, Debug, ValueEnum)]
enum ToolName {
    Search,
    Markdown,
    Citmatch,
    Gquery,
    Espell,
    Summary,
}

impl ToolName {
    fn as_str(&self) -> &'static str {
        match self {
            ToolName::Search => "search_pubmed",
            ToolName::Markdown => "get_pmc_markdown",
            ToolName::Citmatch => "match_citations",
            ToolName::Gquery => "global_query",
            ToolName::Espell => "spell_check",
            ToolName::Summary => "fetch_summaries",
        }
    }
}

#[tool_router]
impl PubMedServer {
    #[tool(
        description = "Search PubMed for articles with filters (study type: randomized_controlled_trial, clinical_trial, meta_analysis, systematic_review, review, observational_study, case_report; text availability: free_full_text, full_text, pmc_only; date range: start_year and end_year for publication date filtering)"
    )]
    async fn search_pubmed(
        &self,
        params: Parameters<tools::search::SearchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::search::search_pubmed(self, params).await
    }

    #[tool(
        description = "Get markdown formatted content from a PMC (PubMed Central) article. Returns the full article text in well-formatted markdown including metadata, sections, references, and additional information like funding and acknowledgments."
    )]
    async fn get_pmc_markdown(
        &self,
        params: Parameters<tools::markdown::MarkdownRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::markdown::get_pmc_markdown(self, params).await
    }

    #[tool(
        description = "Match citations to PubMed IDs (PMIDs) using journal name, year, volume, page, and author. Useful for identifying PMIDs from reference lists."
    )]
    async fn match_citations(
        &self,
        params: Parameters<tools::citmatch::CitMatchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::citmatch::match_citations(self, params).await
    }

    #[tool(
        description = "Query all NCBI databases for record counts matching a search term. Returns hit counts across all Entrez databases (PubMed, PMC, Nucleotide, Protein, etc.)."
    )]
    async fn global_query(
        &self,
        params: Parameters<tools::gquery::GlobalQueryRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::gquery::global_query(self, params).await
    }

    #[tool(
        description = "Check spelling of a search term using the NCBI ESpell API. Returns spelling suggestions and corrected query. Use before searching to improve accuracy."
    )]
    async fn spell_check(
        &self,
        params: Parameters<tools::espell::SpellCheckRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::espell::spell_check(self, params).await
    }

    #[tool(
        description = "Fetch lightweight article summaries by PubMed IDs using the ESummary API. Returns basic metadata (title, authors, journal, dates, DOI) without abstracts or MeSH terms. Faster than search_pubmed when you already have PMIDs and only need bibliographic overview data."
    )]
    async fn fetch_summaries(
        &self,
        params: Parameters<tools::summary::SummaryRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::summary::fetch_summaries(self, params).await
    }
}

#[tool_handler]
impl ServerHandler for PubMedServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "pubmed-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "PubMed MCP Server - Search and retrieve biomedical literature from PubMed and PMC databases.".to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing to stderr to avoid interfering with JSON-RPC on stdout
    // MCP protocol uses stdin/stdout for JSON-RPC messages
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    info!("Starting PubMed MCP Server");

    let enabled_tools: Option<Arc<HashSet<String>>> = if args.tools.is_empty() {
        None
    } else {
        Some(Arc::new(
            args.tools.iter().map(|t| t.as_str().to_string()).collect(),
        ))
    };

    if let Some(port) = args.port {
        let shared_client = Arc::new(pubmed_client::Client::new());
        let et = enabled_tools.clone();

        use rmcp::transport::streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpService,
        };
        let service = StreamableHttpService::new(
            move || {
                let client = Arc::clone(&shared_client);
                Ok(tools::PubMedServer::with_options(client, et.as_deref()))
            },
            LocalSessionManager::default().into(),
            Default::default(),
        );

        let router = axum::Router::new().nest_service("/mcp", service);
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        info!("HTTP MCP server listening on port {port}");
        axum::serve(listener, router).await?;
    } else {
        let service = tools::PubMedServer::with_options(
            Arc::new(pubmed_client::Client::new()),
            enabled_tools.as_deref(),
        )
        .serve(stdio())
        .await?;
        info!("MCP server initialized, waiting for requests");
        service.waiting().await?;
    }

    Ok(())
}
