//! Tools module for PubMed MCP server

use pubmed_client::Client as PubMedClient;
use rmcp::handler::server::router::tool::ToolRouter;
use std::collections::HashSet;
use std::sync::Arc;

pub mod articles;
pub mod as_of;
#[cfg(test)]
mod as_of_tests;
pub mod citmatch;
pub mod common;
pub mod convert;
pub mod einfo;
pub mod elink;
pub mod espell;
pub mod europe_pmc;
pub mod export;
pub mod figures;
pub mod fulltext;
pub mod gquery;
pub mod markdown;
pub mod output;
pub mod search;
pub mod summary;

/// PubMed MCP Server
#[derive(Clone)]
pub struct PubMedServer {
    pub(crate) client: Arc<PubMedClient>,
    pub(crate) tool_router: ToolRouter<Self>,
    /// When set, the server behaves as if it ran on this day (see [`as_of`]).
    pub(crate) as_of: Option<as_of::AsOf>,
}

/// Tools that cannot honour `--as-of` and are therefore not offered under it.
///
/// `global_query` reports hit counts across every Entrez database, and those
/// counts cannot all be bounded by date.
pub const UNAVAILABLE_AS_OF: &[&str] = &["global_query"];

impl PubMedServer {
    pub fn with_options(
        client: Arc<PubMedClient>,
        enabled_tools: Option<&HashSet<String>>,
        as_of: Option<as_of::AsOf>,
    ) -> Self {
        let mut tool_router = Self::tool_router();
        if as_of.is_some() {
            for name in UNAVAILABLE_AS_OF {
                tool_router.remove_route(name);
            }
        }
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
            as_of,
        }
    }
}
