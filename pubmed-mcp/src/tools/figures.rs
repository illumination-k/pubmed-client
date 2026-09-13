//! Figure extraction tool for PMC articles

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{collect_figures, collect_tables, internal_error, normalize_pmc_id};
use super::output::{FigureOut, TableOut};

/// Request parameters for get_pmc_figures tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FiguresRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,
}

/// Structured answer of the `get_pmc_figures` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FiguresOutput {
    /// The PMC ID that was inspected.
    pub pmc_id: String,
    /// Article title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Number of figures found.
    pub figure_count: usize,
    /// Number of tables found.
    pub table_count: usize,
    /// Figures, collected across all sections in document order.
    pub figures: Vec<FigureOut>,
    /// Tables, collected across all sections in document order.
    pub tables: Vec<TableOut>,
}

/// Get figure and table metadata from a PMC article
pub async fn get_pmc_figures(
    server: &super::PubMedServer,
    Parameters(params): Parameters<FiguresRequest>,
) -> Result<Json<FiguresOutput>, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);

    info!(pmc_id = %pmc_id, "Extracting figures from PMC article");

    let article = server
        .client
        .pmc
        .fetch_full_text(&pmc_id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch PMC article: {}", e)))?;

    let figures: Vec<FigureOut> = collect_figures(article.sections())
        .into_iter()
        .map(FigureOut::from)
        .collect();
    let tables: Vec<TableOut> = collect_tables(article.sections())
        .into_iter()
        .map(TableOut::from)
        .collect();

    Ok(Json(FiguresOutput {
        pmc_id,
        title: article.title().map(str::to_string),
        figure_count: figures.len(),
        table_count: tables.len(),
        figures,
        tables,
    }))
}
