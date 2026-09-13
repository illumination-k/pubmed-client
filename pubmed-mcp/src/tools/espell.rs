//! Spell check tool for PubMed MCP server

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::internal_error;

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

/// Structured answer of the `spell_check` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SpellCheckOutput {
    /// Database the term was checked against.
    pub database: String,
    /// The term as submitted.
    pub query: String,
    /// The term with ESpell's corrections applied; equal to `query` when
    /// nothing needed correcting.
    pub corrected_query: String,
    /// Whether ESpell proposed any correction.
    pub has_corrections: bool,
    /// The individual replacement words ESpell suggested.
    pub corrections: Vec<String>,
}

/// Check spelling of a search term using the NCBI ESpell API
pub async fn spell_check(
    server: &super::PubMedServer,
    Parameters(params): Parameters<SpellCheckRequest>,
) -> Result<Json<SpellCheckOutput>, ErrorData> {
    let db = params.db.as_deref().unwrap_or("pubmed");

    info!(term = %params.term, db = %db, "Checking spelling");

    let result = server
        .client
        .pubmed
        .spell_check_db(&params.term, db)
        .await
        .map_err(|e| internal_error(format!("Spell check failed: {}", e)))?;

    Ok(Json(SpellCheckOutput {
        has_corrections: result.has_corrections(),
        corrections: result
            .replacements()
            .into_iter()
            .map(str::to_string)
            .collect(),
        database: result.database,
        query: result.query,
        corrected_query: result.corrected_query,
    }))
}
