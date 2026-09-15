//! Tools module for PubMed MCP server

use pubmed_client::Client as PubMedClient;
use rmcp::handler::server::router::tool::ToolRouter;
use std::collections::HashSet;
use std::sync::Arc;

pub mod articles;
pub mod citmatch;
pub mod common;
pub mod convert;
pub mod download;
pub mod einfo;
pub mod elink;
pub mod espell;
pub mod europe_pmc;
pub mod export;
pub mod figure_images;
pub mod figures;
pub mod fulltext;
pub mod gquery;
pub mod markdown;
pub mod output;
pub mod search;
pub mod summary;

/// Server-level policy, as opposed to the client configuration in
/// [`crate::config::ClientArgs`].
///
/// A struct rather than positional arguments: both knobs are optional and one of
/// them is a bare `bool`, which at a call site says nothing about what it turns
/// on.
#[derive(Debug, Default, Clone, Copy)]
pub struct ServerOptions<'a> {
    /// Tools to keep registered; `None` keeps all of them.
    pub enabled_tools: Option<&'a HashSet<String>>,
    /// Whether the download tools may write to this machine's filesystem.
    ///
    /// Off by default. Writing files next to whatever launched the server is a
    /// side effect the caller never asked for, and an MCP server is usually
    /// started by a host config that no one revisits, so the filesystem is
    /// something you hand out deliberately. An `s3://` destination needs no such
    /// gate: naming a bucket *is* the deliberate act.
    pub allow_local_downloads: bool,
}

/// PubMed MCP Server
#[derive(Clone)]
pub struct PubMedServer {
    pub(crate) client: Arc<PubMedClient>,
    pub(crate) tool_router: ToolRouter<Self>,
    pub(crate) allow_local_downloads: bool,
}

impl PubMedServer {
    pub fn with_options(client: Arc<PubMedClient>, options: ServerOptions<'_>) -> Self {
        let mut tool_router = Self::tool_router();
        if let Some(tools) = options.enabled_tools {
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
            allow_local_downloads: options.allow_local_downloads,
        }
    }
}
