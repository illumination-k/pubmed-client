//! Citation match tool for PubMed MCP server

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::internal_error;
use pubmed_client::{CitationMatchStatus, CitationQuery};

/// Single citation input for matching
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CitationInput {
    #[schemars(description = "Journal title abbreviation (e.g., 'proc natl acad sci u s a')")]
    pub journal: String,

    #[schemars(description = "Publication year (e.g., '1991')")]
    pub year: String,

    #[schemars(description = "Volume number (e.g., '88')")]
    pub volume: String,

    #[schemars(description = "First page number (e.g., '3248')")]
    pub first_page: String,

    #[schemars(description = "Author name (e.g., 'mann bj')")]
    pub author_name: String,

    #[schemars(
        description = "User-defined key for identifying this citation in results (e.g., 'ref1')"
    )]
    pub key: Option<String>,
}

/// Citation match request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CitMatchRequest {
    #[schemars(
        description = "List of citations to match against PubMed. Each citation needs journal, year, volume, first_page, and author_name."
    )]
    pub citations: Vec<CitationInput>,
}

/// Outcome of matching one citation.
#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    /// Exactly one PubMed record matched.
    Found,
    /// No PubMed record matched.
    NotFound,
    /// More than one record matched the citation.
    Ambiguous,
}

impl From<&CitationMatchStatus> for MatchStatus {
    fn from(status: &CitationMatchStatus) -> Self {
        match status {
            CitationMatchStatus::Found => MatchStatus::Found,
            CitationMatchStatus::NotFound => MatchStatus::NotFound,
            CitationMatchStatus::Ambiguous => MatchStatus::Ambiguous,
        }
    }
}

/// One citation and the PMID it resolved to.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CitationMatchOut {
    /// The key supplied with the citation (or a generated `refN`), so the
    /// caller can join results back onto its own reference list.
    pub key: String,
    /// Journal as submitted.
    pub journal: String,
    /// Year as submitted.
    pub year: String,
    /// Volume as submitted.
    pub volume: String,
    /// First page as submitted.
    pub first_page: String,
    /// Author name as submitted.
    pub author_name: String,
    /// Match outcome.
    pub status: MatchStatus,
    /// Matched PubMed ID, present only when `status` is `found`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmid: Option<String>,
}

/// Structured answer of the `match_citations` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CitMatchOutput {
    /// Number of citations submitted.
    pub count: usize,
    /// How many resolved to a single PMID.
    pub found: usize,
    /// One entry per submitted citation, in submission order.
    pub matches: Vec<CitationMatchOut>,
}

/// Match citations to PubMed IDs (PMIDs)
pub async fn match_citations(
    server: &super::PubMedServer,
    Parameters(params): Parameters<CitMatchRequest>,
) -> Result<Json<CitMatchOutput>, ErrorData> {
    if params.citations.is_empty() {
        return Ok(Json(CitMatchOutput {
            count: 0,
            found: 0,
            matches: Vec::new(),
        }));
    }

    let citations: Vec<CitationQuery> = params
        .citations
        .iter()
        .enumerate()
        .map(|(i, c)| {
            CitationQuery::new(
                &c.journal,
                &c.year,
                &c.volume,
                &c.first_page,
                &c.author_name,
                c.key.as_deref().unwrap_or(&format!("ref{}", i + 1)),
            )
        })
        .collect();

    info!(
        citation_count = citations.len(),
        "Matching citations to PMIDs"
    );

    let results = server
        .client
        .pubmed
        .match_citations(&citations)
        .await
        .map_err(|e| internal_error(format!("Citation match failed: {}", e)))?;

    let matches: Vec<CitationMatchOut> = results
        .matches
        .iter()
        .map(|m| CitationMatchOut {
            key: m.key.clone(),
            journal: m.journal.clone(),
            year: m.year.clone(),
            volume: m.volume.clone(),
            first_page: m.first_page.clone(),
            author_name: m.author_name.clone(),
            status: MatchStatus::from(&m.status),
            pmid: m.pmid.clone(),
        })
        .collect();

    Ok(Json(CitMatchOutput {
        count: matches.len(),
        found: results.found_count(),
        matches,
    }))
}
