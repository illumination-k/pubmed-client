//! ESummary tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

/// Request parameters for fetch_summaries tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SummaryRequest {
    #[schemars(
        description = "List of PubMed IDs to fetch summaries for (e.g., ['31978945', '33515491'])"
    )]
    pub pmids: Vec<String>,
}

/// Fetch lightweight article summaries by PMIDs using the ESummary API
///
/// Returns basic metadata (title, authors, journal, dates, DOI) without
/// abstracts, MeSH terms, or chemical lists. Faster than search_pubmed
/// when you already have PMIDs and only need bibliographic overview data.
pub async fn fetch_summaries(
    server: &super::PubMedServer,
    Parameters(params): Parameters<SummaryRequest>,
) -> Result<CallToolResult, ErrorData> {
    if params.pmids.is_empty() {
        return Err(ErrorData {
            code: ErrorCode(-32602),
            message: Cow::from("At least one PMID is required"),
            data: None,
        });
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
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Fetch summaries failed: {}", e)),
            data: None,
        })?;

    let mut result = String::new();
    result.push_str(&format!("Retrieved {} summaries:\n\n", summaries.len()));

    for (i, summary) in summaries.iter().enumerate() {
        // Title and identifiers
        if let Some(ref pmc_id) = summary.pmc_id {
            result.push_str(&format!(
                "{}. {} (PMID: {} | PMC: {})\n",
                i + 1,
                summary.title,
                summary.pmid,
                pmc_id
            ));
        } else {
            result.push_str(&format!(
                "{}. {} (PMID: {})\n",
                i + 1,
                summary.title,
                summary.pmid
            ));
        }

        // Authors
        if !summary.authors.is_empty() {
            let authors_str = if summary.authors.len() > 5 {
                let first_five = summary.authors[..5].join(", ");
                format!("{}, et al.", first_five)
            } else {
                summary.authors.join(", ")
            };
            result.push_str(&format!("   Authors: {}\n", authors_str));
        }

        // Journal and date
        result.push_str(&format!(
            "   Journal: {} ({})\n",
            summary.journal, summary.pub_date
        ));

        // DOI
        if let Some(ref doi) = summary.doi {
            result.push_str(&format!("   DOI: {}\n", doi));
        }

        // Volume/Issue/Pages
        if !summary.volume.is_empty() || !summary.pages.is_empty() {
            let mut biblio = String::new();
            if !summary.volume.is_empty() {
                biblio.push_str(&format!("Vol. {}", summary.volume));
            }
            if !summary.issue.is_empty() {
                biblio.push_str(&format!("({})", summary.issue));
            }
            if !summary.pages.is_empty() {
                if !biblio.is_empty() {
                    biblio.push_str(": ");
                }
                biblio.push_str(&summary.pages);
            }
            result.push_str(&format!("   Biblio: {}\n", biblio));
        }

        result.push('\n');
    }

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
