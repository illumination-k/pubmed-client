//! EInfo tools for PubMed MCP server (database information)

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::internal_error;

/// Request parameters for list_databases tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListDatabasesRequest {
    #[schemars(
        description = "Optional search filter to narrow the database list (case-insensitive substring match)"
    )]
    pub filter: Option<String>,
}

/// Structured answer of the `list_databases` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DatabaseListOutput {
    /// The filter that was applied, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Number of databases returned.
    pub count: usize,
    /// Entrez database names, as EInfo spells them.
    pub databases: Vec<String>,
}

/// List all available NCBI Entrez databases
pub async fn list_databases(
    server: &super::PubMedServer,
    Parameters(params): Parameters<ListDatabasesRequest>,
) -> Result<Json<DatabaseListOutput>, ErrorData> {
    info!("Listing NCBI databases");

    let databases = server
        .client
        .pubmed
        .get_database_list()
        .await
        .map_err(|e| internal_error(format!("Failed to list databases: {}", e)))?;

    let databases: Vec<String> = match params.filter {
        Some(ref filter) => {
            let filter_lower = filter.to_lowercase();
            databases
                .into_iter()
                .filter(|db| db.to_lowercase().contains(&filter_lower))
                .collect()
        }
        None => databases,
    };

    Ok(Json(DatabaseListOutput {
        filter: params.filter,
        count: databases.len(),
        databases,
    }))
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

/// A searchable field of an Entrez database.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct FieldOut {
    /// Field tag as used in a query (e.g. "TIAB").
    pub name: String,
    /// Human-readable field name.
    pub full_name: String,
    /// What the field indexes.
    pub description: String,
    /// Number of distinct terms in the field's index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_count: Option<u64>,
    /// Whether the field holds dates.
    pub is_date: bool,
    /// Whether the field is numerical.
    pub is_numerical: bool,
}

/// A link from one Entrez database to another.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LinkOut {
    /// ELink link name (e.g. "pubmed_pmc").
    pub name: String,
    /// Database the link points at.
    pub target_db: String,
    /// What the link means.
    pub description: String,
}

/// Structured answer of the `get_database_info` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DatabaseInfoOutput {
    /// Database name.
    pub name: String,
    /// Display name shown in the Entrez UI.
    pub menu_name: String,
    /// Database description.
    pub description: String,
    /// Number of records.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    /// Date of the last index update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update: Option<String>,
    /// Index build identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    /// Searchable fields, hidden ones excluded. Empty unless
    /// `include_fields` is set (the default).
    pub fields: Vec<FieldOut>,
    /// Cross-database links. Empty unless `include_links` is set.
    pub links: Vec<LinkOut>,
}

/// Get detailed information about a specific NCBI database
pub async fn get_database_info(
    server: &super::PubMedServer,
    Parameters(params): Parameters<DatabaseInfoRequest>,
) -> Result<Json<DatabaseInfoOutput>, ErrorData> {
    let include_fields = params.include_fields.unwrap_or(true);
    let include_links = params.include_links.unwrap_or(false);

    info!(database = %params.database, "Getting database info");

    let db_info = server
        .client
        .pubmed
        .get_database_info(&params.database)
        .await
        .map_err(|e| internal_error(format!("Failed to get database info: {}", e)))?;

    // Hidden fields are internal Entrez bookkeeping and cannot be used in a
    // query, so they would only be noise here.
    let fields = if include_fields {
        db_info
            .fields
            .iter()
            .filter(|field| !field.is_hidden)
            .map(|field| FieldOut {
                name: field.name.clone(),
                full_name: field.full_name.clone(),
                description: field.description.clone(),
                term_count: field.term_count,
                is_date: field.is_date,
                is_numerical: field.is_numerical,
            })
            .collect()
    } else {
        Vec::new()
    };

    let links = if include_links {
        db_info
            .links
            .iter()
            .map(|link| LinkOut {
                name: link.name.clone(),
                target_db: link.target_db.clone(),
                description: link.description.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(DatabaseInfoOutput {
        name: db_info.name,
        menu_name: db_info.menu_name,
        description: db_info.description,
        count: db_info.count,
        last_update: db_info.last_update,
        build: db_info.build,
        fields,
        links,
    }))
}
