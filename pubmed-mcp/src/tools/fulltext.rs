//! Full-text retrieval tool for PMC articles

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use tracing::info;

use super::common::{
    collect_figures, collect_tables, internal_error, normalize_pmc_id, text_result,
};

/// Request parameters for get_pmc_fulltext tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FullTextRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,

    #[schemars(description = "Include references section (default: false)")]
    pub include_references: Option<bool>,

    #[schemars(description = "Maximum number of sections to return (default: all)")]
    pub max_sections: Option<usize>,
}

/// Get structured full-text content from a PMC article
pub async fn get_pmc_fulltext(
    server: &super::PubMedServer,
    Parameters(params): Parameters<FullTextRequest>,
) -> Result<CallToolResult, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);

    let include_refs = params.include_references.unwrap_or(false);

    info!(pmc_id = %pmc_id, "Fetching PMC full text (structured)");

    let article = server
        .client
        .pmc
        .fetch_full_text(&pmc_id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch PMC article: {}", e)))?;

    let mut result = String::new();

    // Metadata
    result.push_str(&format!(
        "Title: {}\n",
        article.title().unwrap_or("Untitled")
    ));
    result.push_str(&format!("PMC ID: {}\n", article.pmcid()));
    if let Some(ref doi) = article.doi() {
        result.push_str(&format!("DOI: {}\n", doi));
    }

    // Authors
    if !article.authors().is_empty() {
        let authors: Vec<String> = article
            .authors()
            .iter()
            .map(|a| a.full_name.clone())
            .collect();
        result.push_str(&format!("Authors: {}\n", authors.join(", ")));
    }

    // Journal
    result.push_str(&format!(
        "Journal: {}\n",
        article.journal().title.as_deref().unwrap_or("Untitled")
    ));

    // Sections
    let sections = if let Some(max) = params.max_sections {
        &article.sections()[..max.min(article.sections().len())]
    } else {
        article.sections()
    };

    for section in sections {
        let title = section
            .title
            .as_deref()
            .or(section.section_type.as_deref())
            .unwrap_or("Untitled");
        result.push_str(&format!("\n## {}\n", title));
        result.push_str(&format!("{}\n", section.content));
    }

    // Figures (collected from all sections)
    let all_figures = collect_figures(article.sections());
    if !all_figures.is_empty() {
        result.push_str(&format!("\n## Figures ({})\n", all_figures.len()));
        for fig in &all_figures {
            let label = fig.label.as_deref().unwrap_or(&fig.id);
            let caption = fig.caption.as_deref().unwrap_or("");
            result.push_str(&format!("- {}: {}\n", label, caption));
        }
    }

    // Tables (collected from all sections)
    let all_tables = collect_tables(article.sections());
    if !all_tables.is_empty() {
        result.push_str(&format!("\n## Tables ({})\n", all_tables.len()));
        for table in &all_tables {
            let label = table.label.as_deref().unwrap_or(&table.id);
            let caption = table.caption.as_deref().unwrap_or("");
            result.push_str(&format!("- {}: {}\n", label, caption));
        }
    }

    // References
    if include_refs && !article.references().is_empty() {
        result.push_str(&format!(
            "\n## References ({})\n",
            article.references().len()
        ));
        for reference in article.references() {
            result.push_str(&format!(
                "{}. {}\n",
                reference.id,
                reference.format_citation()
            ));
        }
    }

    text_result(result)
}
