//! ESummary tool for PubMed MCP server

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{internal_error, invalid_params};

/// Request parameters for fetch_summaries tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SummaryRequest {
    #[schemars(
        description = "List of PubMed IDs to fetch summaries for (e.g., ['31978945', '33515491'])"
    )]
    pub pmids: Vec<String>,
}

/// One ESummary record.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ArticleSummaryOut {
    /// PubMed ID.
    pub pmid: String,
    /// Article title.
    pub title: String,
    /// Author names, in author order. ESummary carries no affiliations.
    pub authors: Vec<String>,
    /// Journal name as ESummary abbreviates it.
    pub journal: String,
    /// Full journal name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_journal_name: Option<String>,
    /// Publication date.
    pub pub_date: String,
    /// Electronic publication date, when it differs from `pub_date`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epub_date: Option<String>,
    /// DOI, when the record carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// PMC ID, when a free full-text version exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmc_id: Option<String>,
    /// Journal volume.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Journal issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Page range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    /// Publication types (e.g. "Journal Article", "Review").
    pub pub_types: Vec<String>,
}

/// Structured answer of the `fetch_summaries` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SummariesOutput {
    /// Number of PMIDs requested.
    pub requested: usize,
    /// Number of summaries returned.
    pub count: usize,
    /// The summaries, in the order ESummary returned them.
    pub summaries: Vec<ArticleSummaryOut>,
}

/// ESummary reports absent string fields as empty strings; a JSON `null` says
/// "no value" without a consumer having to special-case `""`.
fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Fetch lightweight article summaries by PMIDs using the ESummary API
///
/// Returns basic metadata (title, authors, journal, dates, DOI) without
/// abstracts, MeSH terms, or chemical lists. Faster than search_pubmed
/// when you already have PMIDs and only need bibliographic overview data.
pub async fn fetch_summaries(
    server: &super::PubMedServer,
    Parameters(params): Parameters<SummaryRequest>,
) -> Result<Json<SummariesOutput>, ErrorData> {
    if params.pmids.is_empty() {
        return Err(invalid_params("At least one PMID is required"));
    }

    info!(
        pmids_count = params.pmids.len(),
        "Fetching article summaries via ESummary"
    );

    let pmid_refs: Vec<&str> = params.pmids.iter().map(|s| s.as_str()).collect();

    let summaries = server
        .client
        .pubmed
        .fetch_summaries(&pmid_refs)
        .await
        .map_err(|e| internal_error(format!("Fetch summaries failed: {}", e)))?;

    let summaries: Vec<ArticleSummaryOut> = summaries
        .iter()
        .map(|summary| ArticleSummaryOut {
            pmid: summary.pmid.clone(),
            title: summary.title.clone(),
            authors: summary.authors.clone(),
            journal: summary.journal.clone(),
            full_journal_name: non_empty(&summary.full_journal_name),
            pub_date: summary.pub_date.clone(),
            epub_date: non_empty(&summary.epub_date),
            doi: summary.doi.clone(),
            pmc_id: summary.pmc_id.clone(),
            volume: non_empty(&summary.volume),
            issue: non_empty(&summary.issue),
            pages: non_empty(&summary.pages),
            pub_types: summary.pub_types.clone(),
        })
        .collect();

    Ok(Json(SummariesOutput {
        requested: pmid_refs.len(),
        count: summaries.len(),
        summaries,
    }))
}

#[cfg(test)]
mod tests {
    use super::non_empty;

    #[test]
    fn blank_esummary_fields_become_none() {
        assert_eq!(non_empty(""), None);
        assert_eq!(non_empty("   "), None);
        assert_eq!(non_empty(" 88 "), Some("88".to_string()));
    }
}
