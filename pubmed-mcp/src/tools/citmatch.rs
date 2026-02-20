//! Citation match tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

use pubmed_client::CitationQuery;

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

/// Match citations to PubMed IDs (PMIDs)
pub async fn match_citations(
    server: &super::PubMedServer,
    Parameters(params): Parameters<CitMatchRequest>,
) -> Result<CallToolResult, ErrorData> {
    if params.citations.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No citations provided.",
        )]));
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
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Citation match failed: {}", e)),
            data: None,
        })?;

    let mut output = format!(
        "Matched {} of {} citations:\n\n",
        results.found_count(),
        results.matches.len()
    );

    for m in &results.matches {
        let status_icon = match m.status {
            pubmed_client::CitationMatchStatus::Found => "Found",
            pubmed_client::CitationMatchStatus::NotFound => "Not Found",
            pubmed_client::CitationMatchStatus::Ambiguous => "Ambiguous",
        };

        if let Some(ref pmid) = m.pmid {
            output.push_str(&format!(
                "- [{}] {} ({}, {}): PMID {}\n",
                status_icon, m.key, m.journal, m.year, pmid
            ));
        } else {
            output.push_str(&format!(
                "- [{}] {} ({}, {})\n",
                status_icon, m.key, m.journal, m.year
            ));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
