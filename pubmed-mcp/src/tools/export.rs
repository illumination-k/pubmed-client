//! Citation export tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

/// Export format for citations
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// BibTeX format (for LaTeX)
    Bibtex,
    /// RIS format (for Zotero, Mendeley, EndNote)
    Ris,
    /// CSL-JSON format (for citation processors)
    CslJson,
    /// NBIB/MEDLINE format (PubMed native)
    Nbib,
}

/// Request parameters for export_citations tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportRequest {
    #[schemars(description = "PubMed IDs to export (e.g., ['31978945', '33515491'])")]
    pub pmids: Vec<String>,

    #[schemars(description = "Export format: bibtex, ris, csl_json, or nbib (default: bibtex)")]
    pub format: Option<ExportFormat>,
}

/// Export article citations in various standard formats (BibTeX, RIS, CSL-JSON, NBIB)
pub async fn export_citations(
    server: &super::PubMedServer,
    Parameters(params): Parameters<ExportRequest>,
) -> Result<CallToolResult, ErrorData> {
    if params.pmids.is_empty() {
        return Err(ErrorData {
            code: ErrorCode(-32602),
            message: Cow::from("At least one PMID is required"),
            data: None,
        });
    }

    let format = params.format.unwrap_or(ExportFormat::Bibtex);
    let format_name = match format {
        ExportFormat::Bibtex => "BibTeX",
        ExportFormat::Ris => "RIS",
        ExportFormat::CslJson => "CSL-JSON",
        ExportFormat::Nbib => "NBIB",
    };

    info!(
        pmids_count = params.pmids.len(),
        format = format_name,
        "Exporting citations"
    );

    // Fetch articles
    let pmid_refs: Vec<&str> = params.pmids.iter().map(|s| s.as_str()).collect();
    let articles = server
        .client
        .pubmed
        .fetch_articles(&pmid_refs)
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to fetch articles: {}", e)),
            data: None,
        })?;

    if articles.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No articles found for the given PMIDs.",
        )]));
    }

    let result = match format {
        ExportFormat::Bibtex => pubmed_client::export::articles_to_bibtex(&articles),
        ExportFormat::Ris => pubmed_client::export::articles_to_ris(&articles),
        ExportFormat::CslJson => {
            let json = pubmed_client::export::articles_to_csl_json(&articles);
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| "[]".to_string())
        }
        ExportFormat::Nbib => articles
            .iter()
            .map(|a| a.to_nbib())
            .collect::<Vec<_>>()
            .join("\n\n"),
    };

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
