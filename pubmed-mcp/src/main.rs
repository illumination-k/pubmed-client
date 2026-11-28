use anyhow::Result;
use clap::{Parser, ValueEnum};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::{Json, Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{info, warn};

mod config;
mod tools;
use config::ClientArgs;
use tools::PubMedServer;
use tools::as_of::{AsOf, Bounded};

#[derive(Parser)]
#[command(name = "pubmed-mcp", about = "PubMed MCP Server")]
struct Args {
    /// HTTP port to listen on (if not set, uses stdio)
    #[arg(short, long)]
    port: Option<u16>,

    /// Tools to enable, comma-separated (default: all).
    /// Possible values: search, markdown, citmatch, gquery, espell, summary,
    /// articles, related-articles, citations, pmc-links, list-databases,
    /// database-info, fulltext, figures, convert-id, export,
    /// europe-pmc-search, europe-pmc-fulltext, europe-pmc-references,
    /// europe-pmc-citations, europe-pmc-database-links
    #[arg(short, long, value_delimiter = ',', value_enum)]
    tools: Vec<ToolName>,

    /// Host header values accepted in HTTP mode, comma-separated
    /// (`host` or `host:port`). Replaces the default, which accepts only
    /// loopback hosts as DNS-rebinding protection; set it to the name clients
    /// use when the server sits behind a reverse proxy or load balancer.
    #[arg(long, env = "PUBMED_MCP_ALLOWED_HOSTS", value_delimiter = ',')]
    allowed_hosts: Vec<String>,

    /// Behave as if the server ran on this day (YYYY-MM-DD), for backtests.
    /// No tool returns a record that became available on or after it (PubMed
    /// Entrez date, Europe PMC first index date), and such a record looks
    /// exactly like one that does not exist. `global_query` is not offered.
    #[arg(long, env = "PUBMED_MCP_AS_OF", value_name = "YYYY-MM-DD")]
    as_of: Option<AsOf>,

    #[command(flatten)]
    client: ClientArgs,
}

#[derive(Clone, Debug, ValueEnum)]
enum ToolName {
    Search,
    Markdown,
    Citmatch,
    Gquery,
    Espell,
    Summary,
    Articles,
    RelatedArticles,
    Citations,
    PmcLinks,
    ListDatabases,
    DatabaseInfo,
    Fulltext,
    Figures,
    ConvertId,
    Export,
    EuropePmcSearch,
    EuropePmcFulltext,
    EuropePmcReferences,
    EuropePmcCitations,
    EuropePmcDatabaseLinks,
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
            ToolName::Articles => "fetch_articles",
            ToolName::RelatedArticles => "get_related_articles",
            ToolName::Citations => "get_citations",
            ToolName::PmcLinks => "get_pmc_links",
            ToolName::ListDatabases => "list_databases",
            ToolName::DatabaseInfo => "get_database_info",
            ToolName::Fulltext => "get_pmc_fulltext",
            ToolName::Figures => "get_pmc_figures",
            ToolName::ConvertId => "pmid_to_pmcid",
            ToolName::Export => "export_citations",
            ToolName::EuropePmcSearch => "europe_pmc_search",
            ToolName::EuropePmcFulltext => "europe_pmc_fulltext",
            ToolName::EuropePmcReferences => "europe_pmc_references",
            ToolName::EuropePmcCitations => "europe_pmc_citations",
            ToolName::EuropePmcDatabaseLinks => "europe_pmc_database_links",
        }
    }
}

#[tool_router]
impl PubMedServer {
    #[tool(
        description = "Search PubMed for articles with filters (study type: randomized_controlled_trial, clinical_trial, meta_analysis, systematic_review, review, observational_study, case_report; text availability: free_full_text, full_text, pmc_only; date range: start_year and end_year for publication date filtering). Returns PMIDs with title, authors, journal, date and a 200-character abstract preview (max 100 results); use fetch_articles for full abstracts and MeSH terms."
    )]
    async fn search_pubmed(
        &self,
        params: Parameters<tools::search::SearchRequest>,
    ) -> Result<Json<Bounded<tools::search::SearchOutput>>, ErrorData> {
        self.bounded(tools::search::search_pubmed(self, params).await)
    }

    #[tool(
        description = "Get markdown formatted content from a PMC (PubMed Central) article. Returns the full article text in well-formatted markdown including metadata, sections, references, and additional information like funding and acknowledgments."
    )]
    async fn get_pmc_markdown(
        &self,
        params: Parameters<tools::markdown::MarkdownRequest>,
    ) -> Result<Json<Bounded<tools::markdown::MarkdownOutput>>, ErrorData> {
        self.bounded(tools::markdown::get_pmc_markdown(self, params).await)
    }

    #[tool(
        description = "Match citations to PubMed IDs (PMIDs) using journal name, year, volume, page, and author. Useful for identifying PMIDs from reference lists."
    )]
    async fn match_citations(
        &self,
        params: Parameters<tools::citmatch::CitMatchRequest>,
    ) -> Result<Json<Bounded<tools::citmatch::CitMatchOutput>>, ErrorData> {
        self.bounded(tools::citmatch::match_citations(self, params).await)
    }

    #[tool(
        description = "Query all NCBI databases for record counts matching a search term. Returns hit counts across all Entrez databases (PubMed, PMC, Nucleotide, Protein, etc.)."
    )]
    async fn global_query(
        &self,
        params: Parameters<tools::gquery::GlobalQueryRequest>,
    ) -> Result<Json<Bounded<tools::gquery::GlobalQueryOutput>>, ErrorData> {
        self.bounded(tools::gquery::global_query(self, params).await)
    }

    #[tool(
        description = "Check spelling of a search term using the NCBI ESpell API. Returns spelling suggestions and the corrected query. Use it when a query may contain misspellings or a search returned unexpectedly few results."
    )]
    async fn spell_check(
        &self,
        params: Parameters<tools::espell::SpellCheckRequest>,
    ) -> Result<Json<Bounded<tools::espell::SpellCheckOutput>>, ErrorData> {
        self.bounded(tools::espell::spell_check(self, params).await)
    }

    #[tool(
        description = "Fetch lightweight article summaries by PubMed IDs using the ESummary API. Returns basic metadata (title, authors, journal, dates, DOI) without abstracts or MeSH terms. Faster than search_pubmed when you already have PMIDs and only need bibliographic overview data."
    )]
    async fn fetch_summaries(
        &self,
        params: Parameters<tools::summary::SummaryRequest>,
    ) -> Result<Json<Bounded<tools::summary::SummariesOutput>>, ErrorData> {
        self.bounded(tools::summary::fetch_summaries(self, params).await)
    }

    #[tool(
        description = "Fetch complete PubMed records by PubMed ID (PMID) using the EFetch API. Returns the full abstract, all authors (optionally with affiliations), journal details, MeSH headings, substances, keywords, article types, and identifiers. Use this when you already have PMIDs and need full metadata; search_pubmed only returns a 200-character abstract preview and fetch_summaries omits abstracts and MeSH terms entirely."
    )]
    async fn fetch_articles(
        &self,
        params: Parameters<tools::articles::ArticlesRequest>,
    ) -> Result<Json<Bounded<tools::articles::ArticlesOutput>>, ErrorData> {
        self.bounded(tools::articles::fetch_articles(self, params).await)
    }

    #[tool(
        description = "Find related articles for given PubMed IDs using the ELink API. Returns PMIDs of articles that PubMed considers related based on content similarity."
    )]
    async fn get_related_articles(
        &self,
        params: Parameters<tools::elink::RelatedArticlesRequest>,
    ) -> Result<Json<Bounded<tools::elink::RelatedArticlesOutput>>, ErrorData> {
        self.bounded(tools::elink::get_related_articles(self, params).await)
    }

    #[tool(
        description = "Get articles that cite the given PubMed IDs. Returns PMIDs of citing articles from the PubMed database. Note: counts may be lower than Google Scholar as this only includes PubMed-indexed articles."
    )]
    async fn get_citations(
        &self,
        params: Parameters<tools::elink::CitationsRequest>,
    ) -> Result<Json<Bounded<tools::elink::CitationsOutput>>, ErrorData> {
        self.bounded(tools::elink::get_citations(self, params).await)
    }

    #[tool(
        description = "Check PMC (PubMed Central) full-text availability for given PubMed IDs. Returns PMC IDs for articles that have free full-text versions available."
    )]
    async fn get_pmc_links(
        &self,
        params: Parameters<tools::elink::PmcLinksRequest>,
    ) -> Result<Json<Bounded<tools::elink::PmcLinksOutput>>, ErrorData> {
        self.bounded(tools::elink::get_pmc_links(self, params).await)
    }

    #[tool(
        description = "List all available NCBI Entrez databases (PubMed, PMC, Nucleotide, Protein, Gene, etc.). Optionally filter by name."
    )]
    async fn list_databases(
        &self,
        params: Parameters<tools::einfo::ListDatabasesRequest>,
    ) -> Result<Json<Bounded<tools::einfo::DatabaseListOutput>>, ErrorData> {
        self.bounded(tools::einfo::list_databases(self, params).await)
    }

    #[tool(
        description = "Get detailed information about a specific NCBI database including description, record count, searchable fields, and cross-database links."
    )]
    async fn get_database_info(
        &self,
        params: Parameters<tools::einfo::DatabaseInfoRequest>,
    ) -> Result<Json<Bounded<tools::einfo::DatabaseInfoOutput>>, ErrorData> {
        self.bounded(tools::einfo::get_database_info(self, params).await)
    }

    #[tool(
        description = "Get structured full-text content from a PMC article. Returns title, authors, abstract, sections, figures, tables, and optionally references. Use get_pmc_markdown for formatted markdown output instead."
    )]
    async fn get_pmc_fulltext(
        &self,
        params: Parameters<tools::fulltext::FullTextRequest>,
    ) -> Result<Json<Bounded<tools::fulltext::FullTextOutput>>, ErrorData> {
        self.bounded(tools::fulltext::get_pmc_fulltext(self, params).await)
    }

    #[tool(
        description = "Extract figure and table metadata from a PMC article. Returns figure IDs, labels, captions, and graphic URLs. Useful for understanding visual content without downloading full text."
    )]
    async fn get_pmc_figures(
        &self,
        params: Parameters<tools::figures::FiguresRequest>,
    ) -> Result<Json<Bounded<tools::figures::FiguresOutput>>, ErrorData> {
        self.bounded(tools::figures::get_pmc_figures(self, params).await)
    }

    #[tool(
        description = "Convert a PubMed ID (PMID) to a PMC ID (PMCID). Checks whether a full-text version is available in PubMed Central."
    )]
    async fn pmid_to_pmcid(
        &self,
        params: Parameters<tools::convert::ConvertIdRequest>,
    ) -> Result<Json<Bounded<tools::convert::ConvertIdOutput>>, ErrorData> {
        self.bounded(tools::convert::pmid_to_pmcid(self, params).await)
    }

    #[tool(
        description = "Export article citations in standard formats: BibTeX (for LaTeX), RIS (for Zotero/Mendeley/EndNote), CSL-JSON (for citation processors), or NBIB (PubMed native). Fetches article metadata and formats it for direct import into reference managers."
    )]
    async fn export_citations(
        &self,
        params: Parameters<tools::export::ExportRequest>,
    ) -> Result<Json<Bounded<tools::export::ExportOutput>>, ErrorData> {
        self.bounded(tools::export::export_citations(self, params).await)
    }

    #[tool(
        description = "Search Europe PMC across every source it indexes: PubMed/MEDLINE, PMC, preprints (PPR), patents (PAT), Agricola and Chinese Biological Abstracts. Complements search_pubmed by reaching preprints and non-PubMed literature, and needs no API key. Supports Europe PMC query syntax (e.g. 'TITLE:CRISPR AND SRC:PPR') and sort expressions such as 'CITED desc'."
    )]
    async fn europe_pmc_search(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcSearchRequest>,
    ) -> Result<Json<Bounded<tools::europe_pmc::EuropePmcSearchOutput>>, ErrorData> {
        self.bounded(tools::europe_pmc::europe_pmc_search(self, params).await)
    }

    #[tool(
        description = "Get the full text of a Europe PMC record as parsed sections, or as raw JATS XML with raw_xml=true. Europe PMC serves open-access full text for PMC records and for some sources PMC itself does not carry. Ids may be bare ('PMC3258128', '33515491') or qualified ('PPR/PPR123456')."
    )]
    async fn europe_pmc_fulltext(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcFullTextRequest>,
    ) -> Result<Json<Bounded<tools::europe_pmc::EuropePmcFullTextOutput>>, ErrorData> {
        self.bounded(tools::europe_pmc::europe_pmc_fulltext(self, params).await)
    }

    #[tool(
        description = "List the works cited by a Europe PMC record (its reference list), with titles, authors, journal, PMID and DOI where Europe PMC has matched them. Works for PubMed (MED), PMC and preprint records alike."
    )]
    async fn europe_pmc_references(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcCitationGraphRequest>,
    ) -> Result<Json<Bounded<tools::europe_pmc::EuropePmcCitationGraphOutput>>, ErrorData> {
        self.bounded(tools::europe_pmc::europe_pmc_references(self, params).await)
    }

    #[tool(
        description = "List the articles that cite a Europe PMC record. Broader coverage than get_citations (which is PubMed-only): includes preprints and non-PubMed sources, and reports each citing article's own citation count."
    )]
    async fn europe_pmc_citations(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcCitationGraphRequest>,
    ) -> Result<Json<Bounded<tools::europe_pmc::EuropePmcCitationGraphOutput>>, ErrorData> {
        self.bounded(tools::europe_pmc::europe_pmc_citations(self, params).await)
    }

    #[tool(
        description = "List cross-references from a Europe PMC record to external biological databases (UniProt, EMBL, PDB, ChEBI, ArrayExpress, ...). Useful for finding the accessions and datasets a paper deposited or referenced."
    )]
    async fn europe_pmc_database_links(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcDatabaseLinksRequest>,
    ) -> Result<Json<Bounded<tools::europe_pmc::EuropePmcDatabaseLinksOutput>>, ErrorData> {
        self.bounded(tools::europe_pmc::europe_pmc_database_links(self, params).await)
    }
}

// `router = self.tool_router` keeps tool listing/dispatch on the per-instance
// router (which `with_options` filters); the rmcp 3.x default is the static
// `Self::tool_router()`, which would ignore the `--tools` filtering.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for PubMedServer {
    fn get_info(&self) -> ServerConfig {
        let mut instructions = "PubMed MCP Server - Search and retrieve biomedical literature from PubMed and PMC databases.".to_string();
        if let Some(as_of) = self.as_of {
            instructions.push_str(&format!(" {}.", as_of.describe()));
        }
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(server_implementation())
            .with_instructions(instructions)
    }
}

/// The server icon, embedded so it reaches the client over stdio with no
/// network fetch. SVG is one image, scalable to any size the client wants.
const ICON_SVG: &[u8] = include_bytes!("../assets/icon.svg");

/// `serverInfo` for the `initialize` response: name/version plus the display
/// metadata (title, icon, website) a client shows in its server list.
fn server_implementation() -> Implementation {
    use base64::Engine as _;
    let icon = Icon::new(format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(ICON_SVG)
    ))
    .with_mime_type("image/svg+xml")
    .with_sizes(vec!["any".to_string()]);

    Implementation::new("pubmed-mcp", env!("CARGO_PKG_VERSION"))
        .with_title("PubMed")
        .with_description(env!("CARGO_PKG_DESCRIPTION"))
        .with_icons(vec![icon])
        .with_website_url("https://illumination-k.github.io/pubmed-client/")
}

/// The streamable HTTP transport mounted at `/mcp`. An empty `allowed_hosts`
/// keeps rmcp's loopback-only default rather than disabling the check.
fn http_router(
    client: Arc<pubmed_client::Client>,
    enabled_tools: Option<Arc<HashSet<String>>>,
    as_of: Option<AsOf>,
    allowed_hosts: &[String],
) -> axum::Router {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    };
    let mut config = StreamableHttpServerConfig::default();
    if !allowed_hosts.is_empty() {
        config = config.with_allowed_hosts(allowed_hosts.iter().cloned());
    }
    let service = StreamableHttpService::new(
        move || {
            Ok(tools::PubMedServer::with_options(
                Arc::clone(&client),
                enabled_tools.as_deref(),
                as_of,
            ))
        },
        LocalSessionManager::default().into(),
        config,
    );
    axum::Router::new().nest_service("/mcp", service)
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

    let client_config = args.client.build_config()?;
    info!(
        api_key = args.client.api_key.is_some(),
        email = args.client.email.is_some(),
        tool = %args.client.tool,
        cache = args.client.cache_enabled(),
        "Client configured"
    );

    if let Some(as_of) = args.as_of {
        info!(%as_of, "Running as of a past date; later records are hidden");
        for tool in &args.tools {
            if tools::UNAVAILABLE_AS_OF.contains(&tool.as_str()) {
                warn!(
                    tool = tool.as_str(),
                    "Tool cannot be bounded by --as-of and is not offered"
                );
            }
        }
    }

    if let Some(port) = args.port {
        let router = http_router(
            Arc::new(pubmed_client::Client::with_config(client_config)),
            enabled_tools,
            args.as_of,
            &args.allowed_hosts,
        );
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        info!("HTTP MCP server listening on port {port}");
        axum::serve(listener, router).await?;
    } else {
        let service = tools::PubMedServer::with_options(
            Arc::new(pubmed_client::Client::with_config(client_config)),
            enabled_tools.as_deref(),
            args.as_of,
        )
        .serve(stdio())
        .await?;
        info!("MCP server initialized, waiting for requests");
        service.waiting().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serves `http_router` on an ephemeral loopback port and returns the
    /// status code of an `initialize` POST carrying the given `Host` header.
    async fn initialize_status(allowed_hosts: &[String], host: &str) -> u16 {
        let router = http_router(
            Arc::new(pubmed_client::Client::new()),
            None,
            None,
            allowed_hosts,
        );
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await });

        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#;
        let request = format!(
            "POST /mcp HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut status_line = [0u8; 12];
        stream.read_exact(&mut status_line).await.unwrap();
        std::str::from_utf8(&status_line[9..12])
            .unwrap()
            .parse()
            .unwrap()
    }

    #[tokio::test]
    async fn allowed_hosts_controls_the_host_header_check() {
        let proxied = vec!["mcp.example.com".to_string()];
        let cases: [(&str, &[String], &str, bool); 4] = [
            ("default accepts loopback", &[], "127.0.0.1", true),
            (
                "default rejects a proxied name",
                &[],
                "mcp.example.com",
                false,
            ),
            (
                "configured name is accepted",
                &proxied,
                "mcp.example.com",
                true,
            ),
            (
                "configured list replaces loopback",
                &proxied,
                "localhost",
                false,
            ),
        ];
        for (name, allowed, host, accepted) in cases {
            let status = initialize_status(allowed, host).await;
            assert_eq!(status == 200, accepted, "{name}: got HTTP {status}");
        }
    }

    /// Catches clap definition mistakes (duplicate long names, a `--tool`
    /// shadowed by `--tools`, bad defaults) that would otherwise only show up
    /// as a panic when a user runs the server.
    #[test]
    fn cli_definition_is_valid() {
        Args::command().debug_assert();
    }

    /// The container smoke test in `ci-docker.yml` greps `--help` for this
    /// string. Flattening an `Args` struct silently copies *its* doc comment
    /// onto the parent command's `about`, which drops the server's own
    /// description from the first line of `--help`.
    #[test]
    fn help_output_keeps_the_server_description() {
        let help = Args::command().render_long_help().to_string();
        assert!(
            help.starts_with("PubMed MCP Server"),
            "--help must open with the server description, got:\n{help}"
        );
    }

    /// `--tools` filters by the *registered* tool name, so a `ToolName`
    /// whose `as_str()` drifts from its `#[tool]` method silently removes
    /// every tool instead of selecting one. Check the whole enum, not just
    /// the tool of the day.
    #[test]
    fn every_tool_name_matches_a_registered_tool() {
        let registered: Vec<String> = PubMedServer::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();

        for variant in ToolName::value_variants() {
            assert!(
                registered.contains(&variant.as_str().to_string()),
                "{:?} maps to `{}`, which no #[tool] method defines; registered: {registered:?}",
                variant,
                variant.as_str()
            );
        }

        assert_eq!(
            registered.len(),
            ToolName::value_variants().len(),
            "every registered tool should be selectable via --tools; registered: {registered:?}"
        );
    }

    /// Every tool answers with `Json<T>`, so every registered tool must
    /// advertise an `outputSchema` describing `T`. A tool that regresses to a
    /// bare text result loses the schema silently, and a client that relies on
    /// `structuredContent` would have nothing to validate against.
    #[test]
    fn every_tool_advertises_an_object_output_schema() {
        for tool in PubMedServer::tool_router().list_all() {
            let schema = tool
                .output_schema
                .as_ref()
                .unwrap_or_else(|| panic!("{} has no outputSchema", tool.name));
            assert_eq!(
                schema.get("type").and_then(|ty| ty.as_str()),
                Some("object"),
                "{}'s outputSchema should describe a JSON object, got: {schema:?}",
                tool.name
            );
        }
    }

    /// The as-of date is how an agent and the person reading its transcript
    /// learn that the view is bounded, so no tool may answer without a place
    /// for it — including the tools whose data `--as-of` leaves unchanged.
    #[test]
    fn every_tool_can_report_the_as_of_date() {
        for tool in PubMedServer::tool_router().list_all() {
            let schema = tool.output_schema.as_ref().unwrap();
            assert!(
                schema
                    .get("properties")
                    .and_then(|properties| properties.get("as_of"))
                    .is_some(),
                "{}'s outputSchema has no `as_of` property: {schema:?}",
                tool.name
            );
        }
    }

    #[test]
    fn answers_carry_the_as_of_date_only_when_one_is_set() {
        let answer = |as_of: Option<AsOf>| {
            let server =
                PubMedServer::with_options(Arc::new(pubmed_client::Client::new()), None, as_of);
            let Ok(Json(bounded)) = server.bounded(Ok(Json(tools::convert::ConvertIdOutput {
                pmid: "1".to_string(),
                available: false,
                pmcid: None,
            }))) else {
                panic!("an Ok answer stays Ok");
            };
            serde_json::to_value(bounded).unwrap()
        };

        assert_eq!(
            answer("2020-02-01".parse().ok()),
            serde_json::json!({ "pmid": "1", "available": false, "as_of": "2020-02-01" })
        );
        assert_eq!(
            answer(None),
            serde_json::json!({ "pmid": "1", "available": false })
        );
    }

    #[test]
    fn as_of_must_be_an_iso_date() {
        let args = Args::try_parse_from(["pubmed-mcp", "--as-of", "2020-02-01"]).unwrap();
        assert_eq!(
            args.as_of.map(|as_of| as_of.to_string()).as_deref(),
            Some("2020-02-01")
        );

        for invalid in ["2020", "2020-02-30", "02/01/2020", ""] {
            let Err(err) = Args::try_parse_from(["pubmed-mcp", "--as-of", invalid]) else {
                panic!("`--as-of {invalid}` must be rejected, not ignored");
            };
            assert!(err.to_string().contains("YYYY-MM-DD"), "{err}");
        }
    }

    #[test]
    fn client_options_are_parsed_alongside_the_server_options() {
        let args = Args::try_parse_from([
            "pubmed-mcp",
            "--port",
            "8080",
            "--tools",
            "search,markdown",
            "--api-key",
            "secret",
            "--email",
            "researcher@example.edu",
            "--tool",
            "my-server",
            "--cache",
        ])
        .expect("server and client options should coexist");

        assert_eq!(args.port, Some(8080));
        assert_eq!(args.tools.len(), 2);
        assert_eq!(args.client.api_key.as_deref(), Some("secret"));
        assert_eq!(args.client.email.as_deref(), Some("researcher@example.edu"));
        assert_eq!(args.client.tool, "my-server");
        assert!(args.client.cache);
    }
}
