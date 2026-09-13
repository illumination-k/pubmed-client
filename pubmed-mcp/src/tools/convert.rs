//! ID conversion tool for PubMed MCP server

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::internal_error;

/// Request parameters for pmid_to_pmcid tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertIdRequest {
    #[schemars(description = "PubMed ID to convert (e.g., '31978945')")]
    pub pmid: String,
}

/// Structured answer of the `pmid_to_pmcid` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ConvertIdOutput {
    /// The PubMed ID that was looked up.
    pub pmid: String,
    /// Whether a PMC full-text version exists.
    pub available: bool,
    /// The PMC ID, ready to pass to `get_pmc_markdown` or `get_pmc_fulltext`.
    /// Absent when the article has no PMC version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmcid: Option<String>,
}

/// Convert a PubMed ID (PMID) to a PMC ID (PMCID) if the article has a PMC full-text version
pub async fn pmid_to_pmcid(
    server: &super::PubMedServer,
    Parameters(params): Parameters<ConvertIdRequest>,
) -> Result<Json<ConvertIdOutput>, ErrorData> {
    info!(pmid = %params.pmid, "Converting PMID to PMCID");

    let pmcid = server
        .client
        .pmc
        .check_pmc_availability(&params.pmid)
        .await
        .map_err(|e| internal_error(format!("Failed to check PMC availability: {}", e)))?;

    Ok(Json(ConvertIdOutput {
        pmid: params.pmid,
        available: pmcid.is_some(),
        pmcid,
    }))
}
