//! ELink tools for PubMed MCP server (related articles, citations, PMC links)

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{internal_error, invalid_params};

/// Request parameters for get_related_articles tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RelatedArticlesRequest {
    #[schemars(description = "PubMed IDs to find related articles for (e.g., [31978945])")]
    pub pmids: Vec<u32>,

    #[schemars(description = "Maximum number of related PMIDs to return (default: 20)")]
    pub max_results: Option<usize>,
}

/// Structured answer of the `get_related_articles` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct RelatedArticlesOutput {
    /// The PMIDs that were queried.
    pub source_pmids: Vec<u32>,
    /// ELink link name behind the result (e.g. "pubmed_pubmed").
    pub link_type: String,
    /// Number of related articles PubMed reported, before `max_results`.
    pub total: usize,
    /// The related PMIDs, truncated to `max_results`.
    pub related_pmids: Vec<u32>,
}

/// Find related articles for given PMIDs using the ELink API
pub async fn get_related_articles(
    server: &super::PubMedServer,
    Parameters(params): Parameters<RelatedArticlesRequest>,
) -> Result<Json<RelatedArticlesOutput>, ErrorData> {
    if params.pmids.is_empty() {
        return Err(invalid_params("At least one PMID is required"));
    }

    let max = params.max_results.unwrap_or(20);

    info!(
        pmids_count = params.pmids.len(),
        max_results = max,
        "Finding related articles"
    );

    let related = server
        .client
        .pubmed
        .get_related_articles(&params.pmids)
        .await
        .map_err(|e| internal_error(format!("Failed to get related articles: {}", e)))?;

    Ok(Json(RelatedArticlesOutput {
        source_pmids: related.source_pmids,
        link_type: related.link_type,
        total: related.related_pmids.len(),
        related_pmids: related.related_pmids.into_iter().take(max).collect(),
    }))
}

/// Request parameters for get_citations tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CitationsRequest {
    #[schemars(description = "PubMed IDs to find citing articles for (e.g., [31978945])")]
    pub pmids: Vec<u32>,

    #[schemars(description = "Maximum number of citing PMIDs to return (default: 50)")]
    pub max_results: Option<usize>,
}

/// Structured answer of the `get_citations` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CitationsOutput {
    /// The PMIDs that were queried.
    pub source_pmids: Vec<u32>,
    /// ELink link name behind the result (e.g. "pubmed_pubmed_citedin").
    pub link_type: String,
    /// Number of citing articles PubMed reported, before `max_results`.
    /// Counts PubMed-indexed articles only, so it can be lower than what
    /// Google Scholar reports.
    pub total: usize,
    /// The citing PMIDs, truncated to `max_results`.
    pub citing_pmids: Vec<u32>,
}

/// Get articles that cite the given PMIDs
pub async fn get_citations(
    server: &super::PubMedServer,
    Parameters(params): Parameters<CitationsRequest>,
) -> Result<Json<CitationsOutput>, ErrorData> {
    if params.pmids.is_empty() {
        return Err(invalid_params("At least one PMID is required"));
    }

    let max = params.max_results.unwrap_or(50);

    info!(
        pmids_count = params.pmids.len(),
        max_results = max,
        "Finding citing articles"
    );

    let citations = server
        .client
        .pubmed
        .get_citations(&params.pmids)
        .await
        .map_err(|e| internal_error(format!("Failed to get citations: {}", e)))?;

    Ok(Json(CitationsOutput {
        source_pmids: citations.source_pmids,
        link_type: citations.link_type,
        total: citations.citing_pmids.len(),
        citing_pmids: citations.citing_pmids.into_iter().take(max).collect(),
    }))
}

/// Request parameters for get_pmc_links tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PmcLinksRequest {
    #[schemars(
        description = "PubMed IDs to check for PMC full-text availability (e.g., [31978945, 33515491])"
    )]
    pub pmids: Vec<u32>,
}

/// Structured answer of the `get_pmc_links` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct PmcLinksOutput {
    /// The PMIDs that were queried.
    pub source_pmids: Vec<u32>,
    /// Number of PMC full-text versions found.
    pub count: usize,
    /// PMC IDs of the free full-text versions, `PMC`-prefixed and ready to
    /// pass to the PMC tools.
    pub pmc_ids: Vec<String>,
}

/// Check PMC full-text availability for given PMIDs
pub async fn get_pmc_links(
    server: &super::PubMedServer,
    Parameters(params): Parameters<PmcLinksRequest>,
) -> Result<Json<PmcLinksOutput>, ErrorData> {
    if params.pmids.is_empty() {
        return Err(invalid_params("At least one PMID is required"));
    }

    info!(
        pmids_count = params.pmids.len(),
        "Checking PMC availability"
    );

    let pmc_links = server
        .client
        .pubmed
        .get_pmc_links(&params.pmids)
        .await
        .map_err(|e| internal_error(format!("Failed to get PMC links: {}", e)))?;

    // ELink reports the bare numeric id; the PMC tools expect the prefixed
    // form, so hand back an id the caller can use as-is.
    let pmc_ids: Vec<String> = pmc_links
        .pmc_ids
        .iter()
        .map(|id| super::common::normalize_pmc_id(id))
        .collect();

    Ok(Json(PmcLinksOutput {
        source_pmids: pmc_links.source_pmids,
        count: pmc_ids.len(),
        pmc_ids,
    }))
}
