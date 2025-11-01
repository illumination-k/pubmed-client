use anyhow::Result;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ServerHandler, ServiceExt,
};
use serde::Deserialize;
use std::{borrow::Cow, sync::Arc};
use tracing::info;

use pubmed_client::Client as PubMedClient;

/// PubMed MCP Server
#[derive(Clone)]
struct PubMedServer {
    client: Arc<PubMedClient>,
    tool_router: ToolRouter<Self>,
}

/// Search request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchRequest {
    #[schemars(description = "Search query (e.g., 'COVID-19', 'cancer[ti]')")]
    query: String,
    #[schemars(description = "Maximum number of results (default: 10)")]
    max_results: Option<usize>,
}

#[tool_router]
impl PubMedServer {
    fn new() -> Self {
        let client = PubMedClient::new();
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Search PubMed for articles")]
    async fn search_pubmed(
        &self,
        Parameters(params): Parameters<SearchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let max = params.max_results.unwrap_or(10).min(100);

        info!(query = %params.query, max_results = max, "Searching PubMed");

        let articles = self
            .client
            .pubmed
            .search_and_fetch(&params.query, max)
            .await
            .map_err(|e| ErrorData {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Search failed: {}", e)),
                data: None,
            })?;

        let mut result = String::new();
        result.push_str(&format!("Found {} articles:\n\n", articles.len()));

        for (i, article) in articles.iter().enumerate() {
            result.push_str(&format!(
                "{}. {} (PMID: {})\n",
                i + 1,
                article.title,
                article.pmid
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(result)]))
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
