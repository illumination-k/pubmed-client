//! ID conversion tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

/// Request parameters for pmid_to_pmcid tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ConvertIdRequest {
    #[schemars(description = "PubMed ID to convert (e.g., '31978945')")]
    pub pmid: String,
}

/// Convert a PubMed ID (PMID) to a PMC ID (PMCID) if the article has a PMC full-text version
pub async fn pmid_to_pmcid(
    server: &super::PubMedServer,
    Parameters(params): Parameters<ConvertIdRequest>,
) -> Result<CallToolResult, ErrorData> {
    info!(pmid = %params.pmid, "Converting PMID to PMCID");

    let pmc_id = server
        .client
        .pmc
        .check_pmc_availability(&params.pmid)
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to check PMC availability: {}", e)),
            data: None,
        })?;

    let result = match pmc_id {
        Some(pmcid) => format!(
            "PMID {} → {}\n\nFull text is available in PMC. Use get_pmc_markdown or get_pmc_fulltext with this PMC ID.",
            params.pmid, pmcid
        ),
        None => format!(
            "PMID {} has no PMC full-text version available.",
            params.pmid
        ),
    };

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
