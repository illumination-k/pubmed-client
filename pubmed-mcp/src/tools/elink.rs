//! ELink tools for PubMed MCP server (related articles, citations, PMC links)

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{internal_error, invalid_params};

/// The PMIDs to send to ELink: all of them, or under `--as-of` only the
/// visible ones. A hidden source PMID then contributes no links, which is
/// what ELink answers for a PMID it does not know.
///
/// The tools echo the *requested* PMIDs as `source_pmids` either way, as
/// ELink itself does, so the echo says nothing about which were hidden.
async fn visible_sources(
    server: &super::PubMedServer,
    pmids: &[u32],
) -> Result<Vec<u32>, ErrorData> {
    let Some(as_of) = server.as_of else {
        return Ok(pmids.to_vec());
    };
    let visible = server.visible_pmids(as_of, pmids.iter().copied()).await?;
    Ok(pmids
        .iter()
        .copied()
        .filter(|pmid| visible.contains(pmid))
        .collect())
}

/// Drop the linked PMIDs that `--as-of` hides, keeping ELink's order.
async fn visible_links(
    server: &super::PubMedServer,
    linked: Vec<u32>,
) -> Result<Vec<u32>, ErrorData> {
    let Some(as_of) = server.as_of else {
        return Ok(linked);
    };
    let visible = server.visible_pmids(as_of, linked.iter().copied()).await?;
    Ok(linked
        .into_iter()
        .filter(|pmid| visible.contains(pmid))
        .collect())
}

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

    let sources = visible_sources(server, &params.pmids).await?;
    let pubmed = &server.client.pubmed;
    // Under --as-of ELink narrows the list at the source; `visible_links`
    // then applies the whole rule to what is left.
    let related = match server.as_of {
        Some(as_of) => {
            pubmed
                .get_related_articles_in_date_range(&sources, &as_of.eutils_range())
                .await
        }
        None => pubmed.get_related_articles(&sources).await,
    }
    .map_err(|e| internal_error(format!("Failed to get related articles: {}", e)))?;
    let related_pmids = visible_links(server, related.related_pmids).await?;

    Ok(Json(RelatedArticlesOutput {
        source_pmids: params.pmids,
        link_type: related.link_type,
        total: related_pmids.len(),
        related_pmids: related_pmids.into_iter().take(max).collect(),
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

    let sources = visible_sources(server, &params.pmids).await?;
    let pubmed = &server.client.pubmed;
    // A heavily cited article has tens of thousands of citing records, nearly
    // all later than D: ELink narrows the list at the source, and
    // `visible_links` applies the whole rule to what is left.
    let citations = match server.as_of {
        Some(as_of) => {
            pubmed
                .get_citations_in_date_range(&sources, &as_of.eutils_range())
                .await
        }
        None => pubmed.get_citations(&sources).await,
    }
    .map_err(|e| internal_error(format!("Failed to get citations: {}", e)))?;
    let citing_pmids = visible_links(server, citations.citing_pmids).await?;

    Ok(Json(CitationsOutput {
        source_pmids: params.pmids,
        link_type: citations.link_type,
        total: citing_pmids.len(),
        citing_pmids: citing_pmids.into_iter().take(max).collect(),
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

    let sources = visible_sources(server, &params.pmids).await?;
    let pmc_links = server
        .client
        .pubmed
        .get_pmc_links(&sources)
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
        source_pmids: params.pmids,
        count: pmc_ids.len(),
        pmc_ids,
    }))
}
