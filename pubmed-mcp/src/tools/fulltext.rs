//! Full-text retrieval tool for PMC articles

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{collect_figures, collect_tables, internal_error, normalize_pmc_id};
use super::output::{FigureOut, ReferenceOut, SectionOut, TableOut, sections_out};

/// Request parameters for get_pmc_fulltext tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FullTextRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,

    #[schemars(description = "Include references section (default: false)")]
    pub include_references: Option<bool>,

    #[schemars(description = "Maximum number of top-level sections to return (default: all)")]
    pub max_sections: Option<usize>,
}

/// Structured answer of the `get_pmc_fulltext` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FullTextOutput {
    /// The PMC ID that was fetched.
    pub pmc_id: String,
    /// Article title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// DOI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// Journal name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    /// Author names, in author order.
    pub authors: Vec<String>,
    /// Abstract text, flattened across its parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    /// Number of top-level body sections in the article, before
    /// `max_sections` is applied.
    pub section_count: usize,
    /// Body sections with their subsections nested, in document order.
    pub sections: Vec<SectionOut>,
    /// Figures, collected across all sections.
    pub figures: Vec<FigureOut>,
    /// Tables, collected across all sections.
    pub tables: Vec<TableOut>,
    /// Reference list. Empty unless `include_references` is set; body
    /// sections cite entries by their `id`.
    pub references: Vec<ReferenceOut>,
}

/// Get structured full-text content from a PMC article
pub async fn get_pmc_fulltext(
    server: &super::PubMedServer,
    Parameters(params): Parameters<FullTextRequest>,
) -> Result<Json<FullTextOutput>, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);

    let include_refs = params.include_references.unwrap_or(false);

    info!(pmc_id = %pmc_id, "Fetching PMC full text (structured)");

    let article = server
        .client
        .pmc
        .fetch_full_text(&pmc_id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch PMC article: {}", e)))?;

    let all_sections = article.sections();
    let shown = match params.max_sections {
        Some(max) => &all_sections[..max.min(all_sections.len())],
        None => all_sections,
    };

    Ok(Json(FullTextOutput {
        pmc_id: article.pmcid().to_string(),
        title: article.title().map(str::to_string),
        doi: article.doi().map(str::to_string),
        journal: article.journal().title.clone(),
        authors: article
            .authors()
            .iter()
            .map(|author| author.full_name.clone())
            .collect(),
        abstract_text: article.abstract_text().map(str::to_string),
        section_count: all_sections.len(),
        sections: sections_out(shown),
        // Figures and tables are collected from the whole article rather than
        // only the sections shown: a caller narrowing `max_sections` still
        // wants to know what visual content exists.
        figures: collect_figures(all_sections)
            .into_iter()
            .map(FigureOut::from)
            .collect(),
        tables: collect_tables(all_sections)
            .into_iter()
            .map(TableOut::from)
            .collect(),
        references: if include_refs {
            article
                .references()
                .iter()
                .map(ReferenceOut::from)
                .collect()
        } else {
            Vec::new()
        },
    }))
}
