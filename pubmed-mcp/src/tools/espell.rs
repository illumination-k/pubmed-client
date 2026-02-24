//! Spell check tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

/// Spell check request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SpellCheckRequest {
    #[schemars(
        description = "Search term to spell-check (e.g., 'asthmaa OR alergies', 'fiberblast cell grwth')"
    )]
    pub term: String,

    #[schemars(
        description = "NCBI database to check against. Use the same database you plan to search. Default: 'pubmed'"
    )]
    pub db: Option<String>,
}

/// Check spelling of a search term using the NCBI ESpell API
pub async fn spell_check(
    server: &super::PubMedServer,
    Parameters(params): Parameters<SpellCheckRequest>,
) -> Result<CallToolResult, ErrorData> {
    let db = params.db.as_deref().unwrap_or("pubmed");

    info!(term = %params.term, db = %db, "Checking spelling");

    let result = server
        .client
        .pubmed
        .spell_check_db(&params.term, db)
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Spell check failed: {}", e)),
            data: None,
        })?;

    let mut output = format!("Database: {}\n", result.database);
    output.push_str(&format!("Original query: \"{}\"\n", result.query));
    output.push_str(&format!(
        "Corrected query: \"{}\"\n",
        result.corrected_query
    ));

    if result.has_corrections() {
        let replacements = result.replacements();
        output.push_str(&format!("\nCorrections: {}\n", replacements.join(", ")));
    } else {
        output.push_str("\nNo spelling corrections needed.\n");
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
