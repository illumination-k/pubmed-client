//! Citation export tool for PubMed MCP server

use pubmed_client::ExportFormat as _;
use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use tracing::info;

use super::common::{internal_error, invalid_params, text_result};

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
        return Err(invalid_params("At least one PMID is required"));
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

    let pmid_refs: Vec<&str> = params.pmids.iter().map(|s| s.as_str()).collect();
    let articles = server
        .client
        .pubmed
        .fetch_articles(&pmid_refs)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch articles: {}", e)))?;

    if articles.is_empty() {
        return text_result("No articles found for the given PMIDs.");
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

    text_result(result)
}
