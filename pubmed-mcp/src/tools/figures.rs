//! Figure extraction tool for PMC articles

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use tracing::info;

use super::common::{
    collect_figures, collect_tables, internal_error, normalize_pmc_id, text_result,
};

/// Request parameters for get_pmc_figures tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FiguresRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,
}

/// Get figure and table metadata from a PMC article
pub async fn get_pmc_figures(
    server: &super::PubMedServer,
    Parameters(params): Parameters<FiguresRequest>,
) -> Result<CallToolResult, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);

    info!(pmc_id = %pmc_id, "Extracting figures from PMC article");

    let article = server
        .client
        .pmc
        .fetch_full_text(&pmc_id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch PMC article: {}", e)))?;

    let mut result = format!(
        "Figures from: {} ({})\n\n",
        article.title().unwrap_or("Untitled"),
        pmc_id
    );

    let all_figures = collect_figures(article.sections());
    let all_tables = collect_tables(article.sections());

    if all_figures.is_empty() {
        result.push_str("No figures found in this article.\n");
    } else {
        result.push_str(&format!("Found {} figures:\n\n", all_figures.len()));
        for (i, fig) in all_figures.iter().enumerate() {
            result.push_str(&format!("### Figure {} ({})\n", i + 1, fig.id));
            if let Some(ref label) = fig.label {
                result.push_str(&format!("Label: {}\n", label));
            }
            if let Some(ref caption) = fig.caption {
                result.push_str(&format!("Caption: {}\n", caption));
            }
            if let Some(ref graphic_href) = fig.graphic_href {
                result.push_str(&format!("File: {}\n", graphic_href));
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
            if let Some(ref caption) = table.caption {
                result.push_str(&format!("Caption: {}\n", caption));
            }
            result.push('\n');
        }
    }

    text_result(result)
}
