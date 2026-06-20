//! ELink tools for PubMed MCP server (related articles, citations, PMC links)

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use tracing::info;

use super::common::{internal_error, invalid_params, text_result};

/// Request parameters for get_related_articles tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RelatedArticlesRequest {
    #[schemars(description = "PubMed IDs to find related articles for (e.g., [31978945])")]
    pub pmids: Vec<u32>,

    #[schemars(description = "Maximum number of related PMIDs to return (default: 20)")]
    pub max_results: Option<usize>,
}

/// Find related articles for given PMIDs using the ELink API
pub async fn get_related_articles(
    server: &super::PubMedServer,
    Parameters(params): Parameters<RelatedArticlesRequest>,
) -> Result<CallToolResult, ErrorData> {
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

    let displayed: Vec<_> = related.related_pmids.iter().take(max).collect();
    let total = related.related_pmids.len();

    let mut result = format!(
        "Source PMIDs: {:?}\nFound {} related articles (showing {}):\n\n",
        related.source_pmids,
        total,
        displayed.len()
    );

    for pmid in &displayed {
        result.push_str(&format!("- PMID: {}\n", pmid));
    }

    if total > max {
        result.push_str(&format!("\n... and {} more", total - max));
    }

    text_result(result)
}

/// Request parameters for get_citations tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CitationsRequest {
    #[schemars(description = "PubMed IDs to find citing articles for (e.g., [31978945])")]
    pub pmids: Vec<u32>,

    #[schemars(description = "Maximum number of citing PMIDs to return (default: 50)")]
    pub max_results: Option<usize>,
}

/// Get articles that cite the given PMIDs
pub async fn get_citations(
    server: &super::PubMedServer,
    Parameters(params): Parameters<CitationsRequest>,
) -> Result<CallToolResult, ErrorData> {
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

    let displayed: Vec<_> = citations.citing_pmids.iter().take(max).collect();
    let total = citations.citing_pmids.len();

    let mut result = format!(
        "Source PMIDs: {:?}\nFound {} citing articles in PubMed (showing {}):\n\n",
        citations.source_pmids,
        total,
        displayed.len()
    );

    for pmid in &displayed {
        result.push_str(&format!("- PMID: {}\n", pmid));
    }

    if total > max {
        result.push_str(&format!("\n... and {} more", total - max));
    }

    result.push_str("\nNote: Citation counts reflect PubMed-indexed articles only. Google Scholar and other sources may report higher counts.");

    text_result(result)
}

/// Request parameters for get_pmc_links tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PmcLinksRequest {
    #[schemars(
        description = "PubMed IDs to check for PMC full-text availability (e.g., [31978945, 33515491])"
    )]
    pub pmids: Vec<u32>,
}

/// Check PMC full-text availability for given PMIDs
pub async fn get_pmc_links(
    server: &super::PubMedServer,
    Parameters(params): Parameters<PmcLinksRequest>,
) -> Result<CallToolResult, ErrorData> {
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

    let mut result = format!(
        "Checked {} PMIDs, found {} with PMC full text:\n\n",
        pmc_links.source_pmids.len(),
        pmc_links.pmc_ids.len()
    );

    for pmc_id in &pmc_links.pmc_ids {
        result.push_str(&format!("- PMC{}\n", pmc_id));
    }

    if pmc_links.pmc_ids.is_empty() {
        result.push_str("No PMC full-text articles found for the given PMIDs.\n");
    }

    text_result(result)
}
