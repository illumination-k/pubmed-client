use anyhow::Result;
use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
    transport::stdio, ServerHandler, ServiceExt,
};
use tracing::info;

mod tools;
use tools::PubMedServer;

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
}

#[tool_handler]
impl ServerHandler for PubMedServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "pubmed-mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
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

    // Create and start server
    let service = PubMedServer::new().serve(stdio()).await?;
    info!("MCP server initialized, waiting for requests");

    service.waiting().await?;

    Ok(())
}
