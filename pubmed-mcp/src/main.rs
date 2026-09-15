use anyhow::Result;
use clap::builder::BoolishValueParser;
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
use tracing::info;

mod config;
mod tools;
use config::ClientArgs;
use tools::{PubMedServer, ServerOptions};

#[derive(Parser)]
#[command(name = "pubmed-mcp", about = "PubMed MCP Server")]
struct Args {
    /// HTTP port to listen on (if not set, uses stdio)
    #[arg(short, long)]
    port: Option<u16>,

    /// Tools to enable, comma-separated (default: all).
    /// Possible values: search, markdown, citmatch, gquery, espell, summary,
    /// articles, related-articles, citations, pmc-links, list-databases,
    /// database-info, fulltext, figures, download-figures, download-files,
    /// figure-images, convert-id, export,
    /// europe-pmc-search, europe-pmc-fulltext, europe-pmc-references,
    /// europe-pmc-citations, europe-pmc-database-links
    #[arg(short, long, value_delimiter = ',', value_enum)]
    tools: Vec<ToolName>,

    /// Let the download tools write to this machine's filesystem
    ///
    /// Off by default: `download_pmc_figures` and `download_pmc_files` accept
    /// only `s3://bucket/prefix` destinations unless this is set, so a server
    /// dropped into a host config cannot litter the filesystem of whoever
    /// launched it. Boolish like `--cache`, so
    /// `PUBMED_MCP_ALLOW_LOCAL_DOWNLOADS=1` works as well as `=true`.
    #[arg(
        long,
        env = "PUBMED_MCP_ALLOW_LOCAL_DOWNLOADS",
        num_args = 0..=1,
        default_value_t = false,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
    )]
    allow_local_downloads: bool,

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
    DownloadFigures,
    DownloadFiles,
    FigureImages,
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
            ToolName::DownloadFigures => "download_pmc_figures",
            ToolName::DownloadFiles => "download_pmc_files",
            ToolName::FigureImages => "get_pmc_figure_images",
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
        description = "Search PubMed for articles with filters (study type: randomized_controlled_trial, clinical_trial, meta_analysis, systematic_review, review, observational_study, case_report; text availability: free_full_text, full_text, pmc_only; date range: start_year and end_year for publication date filtering)"
    )]
    async fn search_pubmed(
        &self,
        params: Parameters<tools::search::SearchRequest>,
    ) -> Result<Json<tools::search::SearchOutput>, ErrorData> {
        tools::search::search_pubmed(self, params).await
    }

    #[tool(
        description = "Get markdown formatted content from a PMC (PubMed Central) article. Returns the full article text in well-formatted markdown including metadata, sections, references, and additional information like funding and acknowledgments."
    )]
    async fn get_pmc_markdown(
        &self,
        params: Parameters<tools::markdown::MarkdownRequest>,
    ) -> Result<Json<tools::markdown::MarkdownOutput>, ErrorData> {
        tools::markdown::get_pmc_markdown(self, params).await
    }

    #[tool(
        description = "Match citations to PubMed IDs (PMIDs) using journal name, year, volume, page, and author. Useful for identifying PMIDs from reference lists."
    )]
    async fn match_citations(
        &self,
        params: Parameters<tools::citmatch::CitMatchRequest>,
    ) -> Result<Json<tools::citmatch::CitMatchOutput>, ErrorData> {
        tools::citmatch::match_citations(self, params).await
    }

    #[tool(
        description = "Query all NCBI databases for record counts matching a search term. Returns hit counts across all Entrez databases (PubMed, PMC, Nucleotide, Protein, etc.)."
    )]
    async fn global_query(
        &self,
        params: Parameters<tools::gquery::GlobalQueryRequest>,
    ) -> Result<Json<tools::gquery::GlobalQueryOutput>, ErrorData> {
        tools::gquery::global_query(self, params).await
    }

    #[tool(
        description = "Check spelling of a search term using the NCBI ESpell API. Returns spelling suggestions and corrected query. Use before searching to improve accuracy."
    )]
    async fn spell_check(
        &self,
        params: Parameters<tools::espell::SpellCheckRequest>,
    ) -> Result<Json<tools::espell::SpellCheckOutput>, ErrorData> {
        tools::espell::spell_check(self, params).await
    }

    #[tool(
        description = "Fetch lightweight article summaries by PubMed IDs using the ESummary API. Returns basic metadata (title, authors, journal, dates, DOI) without abstracts or MeSH terms. Faster than search_pubmed when you already have PMIDs and only need bibliographic overview data."
    )]
    async fn fetch_summaries(
        &self,
        params: Parameters<tools::summary::SummaryRequest>,
    ) -> Result<Json<tools::summary::SummariesOutput>, ErrorData> {
        tools::summary::fetch_summaries(self, params).await
    }

    #[tool(
        description = "Fetch complete PubMed records by PubMed ID (PMID) using the EFetch API. Returns the full abstract, all authors (optionally with affiliations), journal details, MeSH headings, substances, keywords, article types, and identifiers. Use this when you already have PMIDs and need full metadata; search_pubmed only returns a 200-character abstract preview and fetch_summaries omits abstracts and MeSH terms entirely."
    )]
    async fn fetch_articles(
        &self,
        params: Parameters<tools::articles::ArticlesRequest>,
    ) -> Result<Json<tools::articles::ArticlesOutput>, ErrorData> {
        tools::articles::fetch_articles(self, params).await
    }

    #[tool(
        description = "Find related articles for given PubMed IDs using the ELink API. Returns PMIDs of articles that PubMed considers related based on content similarity."
    )]
    async fn get_related_articles(
        &self,
        params: Parameters<tools::elink::RelatedArticlesRequest>,
    ) -> Result<Json<tools::elink::RelatedArticlesOutput>, ErrorData> {
        tools::elink::get_related_articles(self, params).await
    }

    #[tool(
        description = "Get articles that cite the given PubMed IDs. Returns PMIDs of citing articles from the PubMed database. Note: counts may be lower than Google Scholar as this only includes PubMed-indexed articles."
    )]
    async fn get_citations(
        &self,
        params: Parameters<tools::elink::CitationsRequest>,
    ) -> Result<Json<tools::elink::CitationsOutput>, ErrorData> {
        tools::elink::get_citations(self, params).await
    }

    #[tool(
        description = "Check PMC (PubMed Central) full-text availability for given PubMed IDs. Returns PMC IDs for articles that have free full-text versions available."
    )]
    async fn get_pmc_links(
        &self,
        params: Parameters<tools::elink::PmcLinksRequest>,
    ) -> Result<Json<tools::elink::PmcLinksOutput>, ErrorData> {
        tools::elink::get_pmc_links(self, params).await
    }

    #[tool(
        description = "List all available NCBI Entrez databases (PubMed, PMC, Nucleotide, Protein, Gene, etc.). Optionally filter by name."
    )]
    async fn list_databases(
        &self,
        params: Parameters<tools::einfo::ListDatabasesRequest>,
    ) -> Result<Json<tools::einfo::DatabaseListOutput>, ErrorData> {
        tools::einfo::list_databases(self, params).await
    }

    #[tool(
        description = "Get detailed information about a specific NCBI database including description, record count, searchable fields, and cross-database links."
    )]
    async fn get_database_info(
        &self,
        params: Parameters<tools::einfo::DatabaseInfoRequest>,
    ) -> Result<Json<tools::einfo::DatabaseInfoOutput>, ErrorData> {
        tools::einfo::get_database_info(self, params).await
    }

    #[tool(
        description = "Get structured full-text content from a PMC article. Returns title, authors, abstract, sections, figures, tables, and optionally references. Use get_pmc_markdown for formatted markdown output instead."
    )]
    async fn get_pmc_fulltext(
        &self,
        params: Parameters<tools::fulltext::FullTextRequest>,
    ) -> Result<Json<tools::fulltext::FullTextOutput>, ErrorData> {
        tools::fulltext::get_pmc_fulltext(self, params).await
    }

    #[tool(
        description = "Extract figure and table metadata from a PMC article. Returns figure IDs, labels, captions, and graphic URLs, but no image data. Useful for understanding visual content without downloading full text; use get_pmc_figure_images to see the images themselves, or download_pmc_figures to write them to a directory or object storage."
    )]
    async fn get_pmc_figures(
        &self,
        params: Parameters<tools::figures::FiguresRequest>,
    ) -> Result<Json<tools::figures::FiguresOutput>, ErrorData> {
        tools::figures::get_pmc_figures(self, params).await
    }

    #[tool(
        description = "Download a PMC article's figures from the PMC Open Access Cloud and return where each one landed, alongside its caption and dimensions. Only the figures are written. output_dir is required: an object-storage prefix ('s3://bucket/prefix', also MinIO/R2 via AWS_* environment variables), or a local directory if the server was started with --allow-local-downloads (writing to the server's filesystem is off by default). Use get_pmc_figure_images instead to receive the images inline without writing anything."
    )]
    async fn download_pmc_figures(
        &self,
        params: Parameters<tools::download::DownloadFiguresRequest>,
    ) -> Result<Json<tools::download::DownloadFiguresOutput>, ErrorData> {
        tools::download::download_pmc_figures(self, params).await
    }

    #[tool(
        description = "Download a PMC article's full Open Access package (full-text XML, figures, PDF, supplementary materials) and return where each file landed. output_dir is required: an object-storage prefix ('s3://bucket/prefix', also MinIO/R2 via AWS_* environment variables), or a local directory if the server was started with --allow-local-downloads (writing to the server's filesystem is off by default)."
    )]
    async fn download_pmc_files(
        &self,
        params: Parameters<tools::download::DownloadFilesRequest>,
    ) -> Result<Json<tools::download::DownloadFilesOutput>, ErrorData> {
        tools::download::download_pmc_files(self, params).await
    }

    #[tool(
        description = "Get a PMC article's figures as inline image data, with no directory needed. Returns each image as an MCP image content block (or an embedded blob for vector formats like PDF/EPS) plus its caption, MIME type and pixel dimensions. Select figures with figure_ids; the response is capped by max_figures and max_total_bytes.",
        output_schema = rmcp::handler::server::tool::schema_for_output::<tools::figure_images::FigureImagesOutput>()
    )]
    async fn get_pmc_figure_images(
        &self,
        params: Parameters<tools::figure_images::FigureImagesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        tools::figure_images::get_pmc_figure_images(self, params).await
    }

    #[tool(
        description = "Convert a PubMed ID (PMID) to a PMC ID (PMCID). Checks whether a full-text version is available in PubMed Central."
    )]
    async fn pmid_to_pmcid(
        &self,
        params: Parameters<tools::convert::ConvertIdRequest>,
    ) -> Result<Json<tools::convert::ConvertIdOutput>, ErrorData> {
        tools::convert::pmid_to_pmcid(self, params).await
    }

    #[tool(
        description = "Export article citations in standard formats: BibTeX (for LaTeX), RIS (for Zotero/Mendeley/EndNote), CSL-JSON (for citation processors), or NBIB (PubMed native). Fetches article metadata and formats it for direct import into reference managers."
    )]
    async fn export_citations(
        &self,
        params: Parameters<tools::export::ExportRequest>,
    ) -> Result<Json<tools::export::ExportOutput>, ErrorData> {
        tools::export::export_citations(self, params).await
    }

    #[tool(
        description = "Search Europe PMC across every source it indexes: PubMed/MEDLINE, PMC, preprints (PPR), patents (PAT), Agricola and Chinese Biological Abstracts. Complements search_pubmed by reaching preprints and non-PubMed literature, and needs no API key. Supports Europe PMC query syntax (e.g. 'TITLE:CRISPR AND SRC:PPR') and sort expressions such as 'CITED desc'."
    )]
    async fn europe_pmc_search(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcSearchRequest>,
    ) -> Result<Json<tools::europe_pmc::EuropePmcSearchOutput>, ErrorData> {
        tools::europe_pmc::europe_pmc_search(self, params).await
    }

    #[tool(
        description = "Get the full text of a Europe PMC record as parsed sections, or as raw JATS XML with raw_xml=true. Europe PMC serves open-access full text for PMC records and for some sources PMC itself does not carry. Ids may be bare ('PMC3258128', '33515491') or qualified ('PPR/PPR123456')."
    )]
    async fn europe_pmc_fulltext(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcFullTextRequest>,
    ) -> Result<Json<tools::europe_pmc::EuropePmcFullTextOutput>, ErrorData> {
        tools::europe_pmc::europe_pmc_fulltext(self, params).await
    }

    #[tool(
        description = "List the works cited by a Europe PMC record (its reference list), with titles, authors, journal, PMID and DOI where Europe PMC has matched them. Works for PubMed (MED), PMC and preprint records alike."
    )]
    async fn europe_pmc_references(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcCitationGraphRequest>,
    ) -> Result<Json<tools::europe_pmc::EuropePmcCitationGraphOutput>, ErrorData> {
        tools::europe_pmc::europe_pmc_references(self, params).await
    }

    #[tool(
        description = "List the articles that cite a Europe PMC record. Broader coverage than get_citations (which is PubMed-only): includes preprints and non-PubMed sources, and reports each citing article's own citation count."
    )]
    async fn europe_pmc_citations(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcCitationGraphRequest>,
    ) -> Result<Json<tools::europe_pmc::EuropePmcCitationGraphOutput>, ErrorData> {
        tools::europe_pmc::europe_pmc_citations(self, params).await
    }

    #[tool(
        description = "List cross-references from a Europe PMC record to external biological databases (UniProt, EMBL, PDB, ChEBI, ArrayExpress, ...). Useful for finding the accessions and datasets a paper deposited or referenced."
    )]
    async fn europe_pmc_database_links(
        &self,
        params: Parameters<tools::europe_pmc::EuropePmcDatabaseLinksRequest>,
    ) -> Result<Json<tools::europe_pmc::EuropePmcDatabaseLinksOutput>, ErrorData> {
        tools::europe_pmc::europe_pmc_database_links(self, params).await
    }
}

// `router = self.tool_router` keeps tool listing/dispatch on the per-instance
// router (which `with_options` filters); the rmcp 3.x default is the static
// `Self::tool_router()`, which would ignore the `--tools` filtering.
#[tool_handler(router = self.tool_router)]
impl ServerHandler for PubMedServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("pubmed-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "PubMed MCP Server - Search and retrieve biomedical literature from PubMed and PMC databases.",
            )
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

    let client_config = args.client.build_config()?;
    info!(
        api_key = args.client.api_key.is_some(),
        email = args.client.email.is_some(),
        tool = %args.client.tool,
        cache = args.client.cache_enabled(),
        allow_local_downloads = args.allow_local_downloads,
        "Client configured"
    );

    let allow_local_downloads = args.allow_local_downloads;

    if let Some(port) = args.port {
        let shared_client = Arc::new(pubmed_client::Client::with_config(client_config));
        let et = enabled_tools.clone();

        use rmcp::transport::streamable_http_server::{
            StreamableHttpService, session::local::LocalSessionManager,
        };
        let service = StreamableHttpService::new(
            move || {
                let client = Arc::clone(&shared_client);
                Ok(tools::PubMedServer::with_options(
                    client,
                    ServerOptions {
                        enabled_tools: et.as_deref(),
                        allow_local_downloads,
                    },
                ))
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
            Arc::new(pubmed_client::Client::with_config(client_config)),
            ServerOptions {
                enabled_tools: enabled_tools.as_deref(),
                allow_local_downloads,
            },
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

    /// Every tool answers with structured content, so every registered tool
    /// must advertise an `outputSchema` describing it. A tool that regresses to
    /// a bare text result loses the schema silently, and a client that relies
    /// on `structuredContent` would have nothing to validate against. Tools
    /// returning `Json<T>` get the schema from `T`; one returning a
    /// `CallToolResult` (to carry image content of its own) must pass
    /// `output_schema` explicitly, and this catches it if it forgets.
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

    /// The default here is a promise: a server launched from a host config with
    /// no flags cannot write to the machine it runs on. `PUBMED_MCP_*` variables
    /// are boolish because MCP host configs and container runtimes all spell a
    /// flag differently, so check the spellings rather than just `=true`.
    #[test]
    fn local_downloads_are_off_unless_asked_for() {
        assert!(
            !Args::try_parse_from(["pubmed-mcp"])
                .expect("the server should start with no arguments")
                .allow_local_downloads,
            "writing to the host filesystem must be opt-in"
        );

        for enabling in [
            "--allow-local-downloads",
            "--allow-local-downloads=true",
            "--allow-local-downloads=1",
            "--allow-local-downloads=yes",
            "--allow-local-downloads=on",
        ] {
            assert!(
                Args::try_parse_from(["pubmed-mcp", enabling])
                    .unwrap_or_else(|e| panic!("{enabling} should parse: {e}"))
                    .allow_local_downloads,
                "{enabling} should enable local downloads"
            );
        }

        assert!(
            !Args::try_parse_from(["pubmed-mcp", "--allow-local-downloads=false"])
                .expect("an explicit false should parse")
                .allow_local_downloads
        );
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
