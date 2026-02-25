//! Figure extraction tool for PMC articles

use pubmed_client::{ArticleSection, Figure, Table};
use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

/// Request parameters for get_pmc_figures tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FiguresRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,
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

/// Get figure and table metadata from a PMC article
pub async fn get_pmc_figures(
    server: &super::PubMedServer,
    Parameters(params): Parameters<FiguresRequest>,
) -> Result<CallToolResult, ErrorData> {
    let pmc_id = if params.pmc_id.starts_with("PMC") {
        params.pmc_id.clone()
    } else {
        format!("PMC{}", params.pmc_id)
    };

    info!(pmc_id = %pmc_id, "Extracting figures from PMC article");

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

    let mut result = format!("Figures from: {} ({})\n\n", article.title, pmc_id);

    let all_figures = collect_figures(&article.sections);
    let all_tables = collect_tables(&article.sections);

    if all_figures.is_empty() {
        result.push_str("No figures found in this article.\n");
    } else {
        result.push_str(&format!("Found {} figures:\n\n", all_figures.len()));
        for (i, fig) in all_figures.iter().enumerate() {
            result.push_str(&format!("### Figure {} ({})\n", i + 1, fig.id));
            if let Some(ref label) = fig.label {
                result.push_str(&format!("Label: {}\n", label));
            }
            result.push_str(&format!("Caption: {}\n", fig.caption));
            if let Some(ref file_name) = fig.file_name {
                result.push_str(&format!("File: {}\n", file_name));
            }
            result.push('\n');
        }
    }

    if !all_tables.is_empty() {
        result.push_str(&format!("\nFound {} tables:\n\n", all_tables.len()));
        for (i, table) in all_tables.iter().enumerate() {
            result.push_str(&format!("### Table {} ({})\n", i + 1, table.id));
            if let Some(ref label) = table.label {
                result.push_str(&format!("Label: {}\n", label));
            }
            result.push_str(&format!("Caption: {}\n", table.caption));
            result.push('\n');
        }
    }

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
