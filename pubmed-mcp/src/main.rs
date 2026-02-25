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
    /// Possible values: search, markdown, citmatch, gquery, espell, summary,
    /// related-articles, citations, pmc-links, list-databases, database-info,
    /// fulltext, figures, convert-id, export
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
    RelatedArticles,
    Citations,
    PmcLinks,
    ListDatabases,
    DatabaseInfo,
    Fulltext,
    Figures,
    ConvertId,
    Export,
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
            ToolName::RelatedArticles => "get_related_articles",
            ToolName::Citations => "get_citations",
            ToolName::PmcLinks => "get_pmc_links",
            ToolName::ListDatabases => "list_databases",
            ToolName::DatabaseInfo => "get_database_info",
            ToolName::Fulltext => "get_pmc_fulltext",
            ToolName::Figures => "get_pmc_figures",
            ToolName::ConvertId => "pmid_to_pmcid",
            ToolName::Export => "export_citations",
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

    #[tool(
        description = "Find related articles for given PubMed IDs using the ELink API. Returns PMIDs of articles that PubMed considers related based on content similarity."
    )]
    async fn get_related_articles(
        &self,
        params: Parameters<tools::elink::RelatedArticlesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::elink::get_related_articles(self, params).await
    }

    #[tool(
        description = "Get articles that cite the given PubMed IDs. Returns PMIDs of citing articles from the PubMed database. Note: counts may be lower than Google Scholar as this only includes PubMed-indexed articles."
    )]
    async fn get_citations(
        &self,
        params: Parameters<tools::elink::CitationsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::elink::get_citations(self, params).await
    }

    #[tool(
        description = "Check PMC (PubMed Central) full-text availability for given PubMed IDs. Returns PMC IDs for articles that have free full-text versions available."
    )]
    async fn get_pmc_links(
        &self,
        params: Parameters<tools::elink::PmcLinksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::elink::get_pmc_links(self, params).await
    }

    #[tool(
        description = "List all available NCBI Entrez databases (PubMed, PMC, Nucleotide, Protein, Gene, etc.). Optionally filter by name."
    )]
    async fn list_databases(
        &self,
        params: Parameters<tools::einfo::ListDatabasesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::einfo::list_databases(self, params).await
    }

    #[tool(
        description = "Get detailed information about a specific NCBI database including description, record count, searchable fields, and cross-database links."
    )]
    async fn get_database_info(
        &self,
        params: Parameters<tools::einfo::DatabaseInfoRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::einfo::get_database_info(self, params).await
    }

    #[tool(
        description = "Get structured full-text content from a PMC article. Returns title, authors, abstract, sections, figures, tables, and optionally references. Use get_pmc_markdown for formatted markdown output instead."
    )]
    async fn get_pmc_fulltext(
        &self,
        params: Parameters<tools::fulltext::FullTextRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::fulltext::get_pmc_fulltext(self, params).await
    }

    #[tool(
        description = "Extract figure and table metadata from a PMC article. Returns figure IDs, labels, captions, and graphic URLs. Useful for understanding visual content without downloading full text."
    )]
    async fn get_pmc_figures(
        &self,
        params: Parameters<tools::figures::FiguresRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::figures::get_pmc_figures(self, params).await
    }

    #[tool(
        description = "Convert a PubMed ID (PMID) to a PMC ID (PMCID). Checks whether a full-text version is available in PubMed Central."
    )]
    async fn pmid_to_pmcid(
        &self,
        params: Parameters<tools::convert::ConvertIdRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::convert::pmid_to_pmcid(self, params).await
    }

    #[tool(
        description = "Export article citations in standard formats: BibTeX (for LaTeX), RIS (for Zotero/Mendeley/EndNote), CSL-JSON (for citation processors), or NBIB (PubMed native). Fetches article metadata and formats it for direct import into reference managers."
    )]
    async fn export_citations(
        &self,
        params: Parameters<tools::export::ExportRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::export::export_citations(self, params).await
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
