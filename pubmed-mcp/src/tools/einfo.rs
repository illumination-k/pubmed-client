//! EInfo tools for PubMed MCP server (database information)

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

/// Request parameters for list_databases tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListDatabasesRequest {
    #[schemars(
        description = "Optional search filter to narrow the database list (case-insensitive substring match)"
    )]
    pub filter: Option<String>,
}

/// List all available NCBI Entrez databases
pub async fn list_databases(
    server: &super::PubMedServer,
    Parameters(params): Parameters<ListDatabasesRequest>,
) -> Result<CallToolResult, ErrorData> {
    info!("Listing NCBI databases");

    let databases = server
        .client
        .pubmed
        .get_database_list()
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to list databases: {}", e)),
            data: None,
        })?;

    let filtered: Vec<&String> = if let Some(ref filter) = params.filter {
        let filter_lower = filter.to_lowercase();
        databases
            .iter()
            .filter(|db| db.to_lowercase().contains(&filter_lower))
            .collect()
    } else {
        databases.iter().collect()
    };

    let mut result = format!("Available NCBI databases ({}):\n\n", filtered.len());
    for db in &filtered {
        result.push_str(&format!("- {}\n", db));
    }

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

/// Request parameters for get_database_info tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DatabaseInfoRequest {
    #[schemars(description = "Database name (e.g., 'pubmed', 'pmc', 'nucleotide', 'protein')")]
    pub database: String,

    #[schemars(description = "Include searchable field list (default: true)")]
    pub include_fields: Option<bool>,

    #[schemars(description = "Include cross-database link list (default: false)")]
    pub include_links: Option<bool>,
}

/// Get detailed information about a specific NCBI database
pub async fn get_database_info(
    server: &super::PubMedServer,
    Parameters(params): Parameters<DatabaseInfoRequest>,
) -> Result<CallToolResult, ErrorData> {
    let include_fields = params.include_fields.unwrap_or(true);
    let include_links = params.include_links.unwrap_or(false);

    info!(database = %params.database, "Getting database info");

    let db_info = server
        .client
        .pubmed
        .get_database_info(&params.database)
        .await
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to get database info: {}", e)),
            data: None,
        })?;

    let mut result = String::new();

    result.push_str(&format!("Database: {}\n", db_info.name));
    result.push_str(&format!("Display name: {}\n", db_info.menu_name));
    result.push_str(&format!("Description: {}\n", db_info.description));
    if let Some(count) = db_info.count {
        result.push_str(&format!("Record count: {}\n", count));
    }
    if let Some(ref update) = db_info.last_update {
        result.push_str(&format!("Last updated: {}\n", update));
    }
    if let Some(ref build) = db_info.build {
        result.push_str(&format!("Build: {}\n", build));
    }

    if include_fields && !db_info.fields.is_empty() {
        result.push_str(&format!(
            "\nSearchable fields ({}):\n",
            db_info.fields.len()
        ));
        result.push_str("| Name | Full Name | Description |\n");
        result.push_str("|------|-----------|-------------|\n");
        for field in &db_info.fields {
            if !field.is_hidden {
                result.push_str(&format!(
                    "| {} | {} | {} |\n",
                    field.name, field.full_name, field.description
                ));
            }
        }
    }

    if include_links && !db_info.links.is_empty() {
        result.push_str(&format!(
            "\nCross-database links ({}):\n",
            db_info.links.len()
        ));
        for link in &db_info.links {
            result.push_str(&format!(
                "- {} → {} ({})\n",
                link.name, link.target_db, link.description
            ));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
