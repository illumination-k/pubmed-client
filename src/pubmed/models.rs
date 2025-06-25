use serde::{Deserialize, Serialize};

/// Represents a PubMed article with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PubMedArticle {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// List of authors
    pub authors: Vec<String>,
    /// Journal name
    pub journal: String,
    /// Publication date
    pub pub_date: String,
    /// DOI (Digital Object Identifier)
    pub doi: Option<String>,
    /// Abstract text (if available)
    pub abstract_text: Option<String>,
    /// Article types (e.g., "Clinical Trial", "Review", etc.)
    pub article_types: Vec<String>,
}

/// Database information from EInfo API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseInfo {
    /// Database name (e.g., "pubmed", "pmc")
    pub name: String,
    /// Human-readable menu name
    pub menu_name: String,
    /// Database description
    pub description: String,
    /// Database build version
    pub build: Option<String>,
    /// Number of records in database
    pub count: Option<u64>,
    /// Last update timestamp
    pub last_update: Option<String>,
    /// Available search fields
    pub fields: Vec<FieldInfo>,
    /// Available links to other databases
    pub links: Vec<LinkInfo>,
}

/// Information about a database search field
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldInfo {
    /// Short field name (e.g., "titl", "auth")
    pub name: String,
    /// Full field name (e.g., "Title", "Author")
    pub full_name: String,
    /// Field description
    pub description: String,
    /// Number of indexed terms
    pub term_count: Option<u64>,
    /// Whether field contains dates
    pub is_date: bool,
    /// Whether field contains numerical values
    pub is_numerical: bool,
    /// Whether field uses single token indexing
    pub single_token: bool,
    /// Whether field uses hierarchical indexing
    pub hierarchy: bool,
    /// Whether field is hidden from users
    pub is_hidden: bool,
}

/// Information about database links
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkInfo {
    /// Link name
    pub name: String,
    /// Menu display name
    pub menu: String,
    /// Link description
    pub description: String,
    /// Target database
    pub target_db: String,
}
