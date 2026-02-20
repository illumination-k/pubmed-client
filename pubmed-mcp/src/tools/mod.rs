//! Tools module for PubMed MCP server

use pubmed_client::Client as PubMedClient;
use rmcp::handler::server::router::tool::ToolRouter;
use std::sync::Arc;

pub mod citmatch;
pub mod gquery;
pub mod markdown;
pub mod search;

/// PubMed MCP Server
#[derive(Clone)]
pub struct PubMedServer {
    pub(crate) client: Arc<PubMedClient>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl PubMedServer {
    pub fn new() -> Self {
        let client = PubMedClient::new();
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }
}
