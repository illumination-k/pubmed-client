//! Full-text retrieval tool for PMC articles

use pubmed_client::{ArticleSection, Figure, Table};
use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

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

/// Collect all figures from sections recursively
fn collect_figures(sections: &[ArticleSection]) -> Vec<&Figure> {
    let mut figures = Vec::new();
    for section in sections {
        figures.extend(section.figures.iter());
        figures.extend(collect_figures(&section.subsections));
    }
    figures
}

/// Collect all tables from sections recursively
fn collect_tables(sections: &[ArticleSection]) -> Vec<&Table> {
    let mut tables = Vec::new();
    for section in sections {
        tables.extend(section.tables.iter());
        tables.extend(collect_tables(&section.subsections));
    }
    tables
}

/// Get structured full-text content from a PMC article
pub async fn get_pmc_fulltext(
    server: &super::PubMedServer,
    Parameters(params): Parameters<FullTextRequest>,
) -> Result<CallToolResult, ErrorData> {
    let pmc_id = if params.pmc_id.starts_with("PMC") {
        params.pmc_id.clone()
    } else {
        format!("PMC{}", params.pmc_id)
    };

    let include_refs = params.include_references.unwrap_or(false);

    info!(pmc_id = %pmc_id, "Fetching PMC full text (structured)");

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

    let mut result = String::new();

    // Metadata
    result.push_str(&format!("Title: {}\n", article.title));
    result.push_str(&format!("PMC ID: {}\n", article.pmcid));
    if let Some(ref doi) = article.doi {
        result.push_str(&format!("DOI: {}\n", doi));
    }

    // Authors
    if !article.authors.is_empty() {
        let authors: Vec<String> = article
            .authors
            .iter()
            .map(|a| a.full_name.clone())
            .collect();
        result.push_str(&format!("Authors: {}\n", authors.join(", ")));
    }

    // Journal
    result.push_str(&format!("Journal: {}\n", article.journal.title));

    // Sections
    let sections = if let Some(max) = params.max_sections {
        &article.sections[..max.min(article.sections.len())]
    } else {
        &article.sections
    };

    for section in sections {
        let title = section.title.as_deref().unwrap_or(&section.section_type);
        result.push_str(&format!("\n## {}\n", title));
        result.push_str(&format!("{}\n", section.content));
    }

    // Figures (collected from all sections)
    let all_figures = collect_figures(&article.sections);
    if !all_figures.is_empty() {
        result.push_str(&format!("\n## Figures ({})\n", all_figures.len()));
        for fig in &all_figures {
            let label = fig.label.as_deref().unwrap_or(&fig.id);
            result.push_str(&format!("- {}: {}\n", label, fig.caption));
        }
    }

    // Tables (collected from all sections)
    let all_tables = collect_tables(&article.sections);
    if !all_tables.is_empty() {
        result.push_str(&format!("\n## Tables ({})\n", all_tables.len()));
        for table in &all_tables {
            let label = table.label.as_deref().unwrap_or(&table.id);
            result.push_str(&format!("- {}: {}\n", label, table.caption));
        }
    }

    // References
    if include_refs && !article.references.is_empty() {
        result.push_str(&format!("\n## References ({})\n", article.references.len()));
        for reference in &article.references {
            result.push_str(&format!(
                "{}. {}\n",
                reference.id,
                reference.format_citation()
            ));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
