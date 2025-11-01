//! Markdown conversion tool for PMC articles

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

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

/// Get markdown formatted content from a PMC article
pub async fn get_pmc_markdown(
    server: &super::PubMedServer,
    Parameters(params): Parameters<MarkdownRequest>,
) -> Result<CallToolResult, ErrorData> {
    // Normalize PMC ID (add PMC prefix if missing)
    let pmc_id = if params.pmc_id.starts_with("PMC") {
        params.pmc_id.clone()
    } else {
        format!("PMC{}", params.pmc_id)
    };

    info!(pmc_id = %pmc_id, "Fetching PMC article for markdown conversion");

    // Fetch the full-text article
    let article = server
        .client
        .pmc
        .fetch_full_text(&pmc_id)
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to fetch PMC article: {}", e)),
            data: None,
        })?;

    // Build markdown converter with configuration
    let converter = PmcMarkdownConverter::new()
        .with_include_metadata(params.include_metadata.unwrap_or(true))
        .with_include_figure_captions(params.include_figure_captions.unwrap_or(true));
    info!(
        pmc_id = %pmc_id,
        title = %article.title,
        "Converting PMC article to markdown"
    );

    // Convert to markdown
    let markdown = converter.convert(&article);

    Ok(CallToolResult::success(vec![Content::text(markdown)]))
}
