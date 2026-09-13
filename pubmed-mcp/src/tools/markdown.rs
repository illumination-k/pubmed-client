//! Markdown conversion tool for PMC articles

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{internal_error, normalize_pmc_id};
use pubmed_client::PmcMarkdownConverter;

/// Request parameters for PMC markdown conversion
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MarkdownRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,

    #[schemars(description = "Include metadata section (default: true)")]
    pub include_metadata: Option<bool>,

    #[schemars(description = "Include figure captions (default: true)")]
    pub include_figure_captions: Option<bool>,
}

/// Structured answer of the `get_pmc_markdown` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct MarkdownOutput {
    /// The PMC ID that was rendered.
    pub pmc_id: String,
    /// Article title, for labelling the document without parsing it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The whole article as markdown. This tool's product is a document, so
    /// it stays one string; use `get_pmc_fulltext` for the section tree.
    pub markdown: String,
}

/// Get markdown formatted content from a PMC article
pub async fn get_pmc_markdown(
    server: &super::PubMedServer,
    Parameters(params): Parameters<MarkdownRequest>,
) -> Result<Json<MarkdownOutput>, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);

    info!(pmc_id = %pmc_id, "Fetching PMC article for markdown conversion");

    let article = server
        .client
        .pmc
        .fetch_full_text(&pmc_id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch PMC article: {}", e)))?;

    let converter = PmcMarkdownConverter::new()
        .with_include_metadata(params.include_metadata.unwrap_or(true))
        .with_include_figure_captions(params.include_figure_captions.unwrap_or(true));
    info!(
        pmc_id = %pmc_id,
        title = %article.title().unwrap_or("Untitled"),
        "Converting PMC article to markdown"
    );

    Ok(Json(MarkdownOutput {
        pmc_id,
        title: article.title().map(str::to_string),
        markdown: converter.convert(&article),
    }))
}
