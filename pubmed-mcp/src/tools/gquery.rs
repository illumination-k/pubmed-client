//! Global query tool for PubMed MCP server

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::internal_error;

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

/// Hit count for one Entrez database.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DatabaseCountOut {
    /// Database name as used in a query (e.g. "pmc").
    pub db_name: String,
    /// Display name shown in the Entrez UI.
    pub menu_name: String,
    /// Number of matching records.
    pub count: u64,
    /// EGQuery status for this database (e.g. "Ok").
    pub status: String,
}

/// Structured answer of the `global_query` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GlobalQueryOutput {
    /// The term that was queried.
    pub term: String,
    /// Whether databases with zero hits were filtered out.
    pub non_zero_only: bool,
    /// Number of databases reported.
    pub count: usize,
    /// Per-database hit counts.
    pub databases: Vec<DatabaseCountOut>,
}

/// Query all NCBI databases for record counts
pub async fn global_query(
    server: &super::PubMedServer,
    Parameters(params): Parameters<GlobalQueryRequest>,
) -> Result<Json<GlobalQueryOutput>, ErrorData> {
    let non_zero_only = params.non_zero_only.unwrap_or(true);

    info!(term = %params.term, "Querying all NCBI databases");

    let results = server
        .client
        .pubmed
        .global_query(&params.term)
        .await
        .map_err(|e| internal_error(format!("Global query failed: {}", e)))?;

    let databases: Vec<DatabaseCountOut> = results
        .results
        .iter()
        .filter(|db| !non_zero_only || db.count > 0)
        .map(|db| DatabaseCountOut {
            db_name: db.db_name.clone(),
            menu_name: db.menu_name.clone(),
            count: db.count,
            status: db.status.clone(),
        })
        .collect();

    Ok(Json(GlobalQueryOutput {
        term: results.term,
        non_zero_only,
        count: databases.len(),
        databases,
    }))
}
