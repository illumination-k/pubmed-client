//! Global query tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

/// Global query request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GlobalQueryRequest {
    #[schemars(
        description = "Search term to query across all NCBI databases (e.g., 'asthma', 'COVID-19')"
    )]
    pub term: String,

    #[schemars(
        description = "Only show databases with matching records (count > 0). Default: true"
    )]
    pub non_zero_only: Option<bool>,
}

/// Query all NCBI databases for record counts
pub async fn global_query(
    server: &super::PubMedServer,
    Parameters(params): Parameters<GlobalQueryRequest>,
) -> Result<CallToolResult, ErrorData> {
    let non_zero_only = params.non_zero_only.unwrap_or(true);

    info!(term = %params.term, "Querying all NCBI databases");

    let results = server
        .client
        .pubmed
        .global_query(&params.term)
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Global query failed: {}", e)),
            data: None,
        })?;

    let mut output = format!("Query: \"{}\"\n\n", results.term);

    let display_results: Vec<_> = if non_zero_only {
        results.results.iter().filter(|r| r.count > 0).collect()
    } else {
        results.results.iter().collect()
    };

    output.push_str(&format!(
        "Found results in {} databases{}:\n\n",
        display_results.len(),
        if non_zero_only {
            " (non-zero only)"
        } else {
            ""
        }
    ));

    // Format as table
    output.push_str("| Database | Count |\n");
    output.push_str("|----------|-------|\n");

    for db in &display_results {
        output.push_str(&format!("| {} | {} |\n", db.menu_name, db.count));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
