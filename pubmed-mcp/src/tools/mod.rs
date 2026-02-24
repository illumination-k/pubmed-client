//! Tools module for PubMed MCP server

use pubmed_client::Client as PubMedClient;
use rmcp::handler::server::router::tool::ToolRouter;
use std::collections::HashSet;
use std::sync::Arc;

pub mod citmatch;
pub mod espell;
pub mod gquery;
pub mod markdown;
pub mod search;
pub mod summary;

/// PubMed MCP Server
#[derive(Clone)]
pub struct PubMedServer {
    pub(crate) client: Arc<PubMedClient>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl PubMedServer {
    pub fn with_options(
        client: Arc<PubMedClient>,
        enabled_tools: Option<&HashSet<String>>,
    ) -> Self {
        let mut tool_router = Self::tool_router();
        if let Some(tools) = enabled_tools {
            let to_remove: Vec<String> = tool_router
                .list_all()
                .into_iter()
                .filter(|t| !tools.contains(t.name.as_ref()))
                .map(|t| t.name.to_string())
                .collect();
            for name in &to_remove {
                tool_router.remove_route(name);
            }
        }
        Self {
            client,
            tool_router,
        }
    }
}
