//! Domain models for Europe PMC REST API JSON responses.
//!
//! These mirror the item-level shapes returned by the Europe PMC RESTful Web
//! Service (<https://europepmc.org/RestfulWebService>). They are intentionally
//! lenient: unknown or rarely-used fields are captured in an `extra` map rather
//! than modelled exhaustively, so new API fields never break deserialization.

use serde::{Deserialize, Serialize};

use super::de::opt_string_flex;

/// A single search result record from the Europe PMC `search` endpoint.
///
/// Field coverage matches `resultType=lite`; `resultType=core` adds many more
/// fields which are preserved in [`EuropePmcResult::extra`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EuropePmcResult {
    /// Record identifier within its source database (e.g. a PMID for `MED`).
    #[serde(default)]
    pub id: String,
    /// Source database code (`MED`, `PMC`, `PPR`, `AGR`, `CBA`, `PAT`, ...).
    #[serde(default)]
    pub source: String,
    /// PubMed ID, when the record is linked to PubMed.
    #[serde(default)]
    pub pmid: Option<String>,
    /// PubMed Central ID (e.g. `PMC7894017`), when full text is in PMC.
    #[serde(default)]
    pub pmcid: Option<String>,
    /// Digital Object Identifier.
    #[serde(default)]
    pub doi: Option<String>,
    /// Article title.
    #[serde(default)]
    pub title: Option<String>,
    /// Comma-separated author list, as provided by Europe PMC.
    #[serde(default)]
    pub author_string: Option<String>,
    /// Journal title.
    #[serde(default)]
    pub journal_title: Option<String>,
    /// Publication year.
    #[serde(default, deserialize_with = "opt_string_flex")]
    pub pub_year: Option<String>,
    /// Open access flag (`"Y"` / `"N"`).
    #[serde(default)]
    pub is_open_access: Option<String>,
    /// Any additional fields not modelled above (populated for `resultType=core`).
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A cited reference from the Europe PMC `references` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EuropePmcReference {
    /// Source database code of the referenced record, when matched.
    #[serde(default)]
    pub source: Option<String>,
    /// Identifier of the referenced record, when matched.
    #[serde(default, deserialize_with = "opt_string_flex")]
    pub id: Option<String>,
    /// Citation type (e.g. `"JOURNAL ARTICLE"`).
    #[serde(default)]
    pub citation_type: Option<String>,
    /// Title of the cited work.
    #[serde(default)]
    pub title: Option<String>,
    /// Comma-separated author list of the cited work.
    #[serde(default)]
    pub author_string: Option<String>,
    /// Abbreviated journal name.
    #[serde(default)]
    pub journal_abbreviation: Option<String>,
    /// Publication year.
    #[serde(default, deserialize_with = "opt_string_flex")]
    pub pub_year: Option<String>,
    /// Journal volume.
    #[serde(default)]
    pub volume: Option<String>,
    /// Journal issue.
    #[serde(default)]
    pub issue: Option<String>,
    /// Page range / location.
    #[serde(default)]
    pub page_info: Option<String>,
    /// PubMed ID of the cited work, when matched.
    #[serde(default, deserialize_with = "opt_string_flex")]
    pub pmid: Option<String>,
    /// DOI of the cited work, when present.
    #[serde(default)]
    pub doi: Option<String>,
    /// Any additional fields not modelled above.
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A citing article from the Europe PMC `citations` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EuropePmcCitation {
    /// Identifier of the citing record within its source database.
    #[serde(default, deserialize_with = "opt_string_flex")]
    pub id: Option<String>,
    /// Source database code of the citing record.
    #[serde(default)]
    pub source: Option<String>,
    /// Citation type (e.g. `"JOURNAL ARTICLE"`).
    #[serde(default)]
    pub citation_type: Option<String>,
    /// Title of the citing article.
    #[serde(default)]
    pub title: Option<String>,
    /// Comma-separated author list of the citing article.
    #[serde(default)]
    pub author_string: Option<String>,
    /// Abbreviated journal name.
    #[serde(default)]
    pub journal_abbreviation: Option<String>,
    /// Publication year.
    #[serde(default, deserialize_with = "opt_string_flex")]
    pub pub_year: Option<String>,
    /// Journal volume.
    #[serde(default)]
    pub volume: Option<String>,
    /// Journal issue.
    #[serde(default)]
    pub issue: Option<String>,
    /// Page range / location.
    #[serde(default)]
    pub page_info: Option<String>,
    /// Number of times the citing article has itself been cited.
    #[serde(default, deserialize_with = "opt_string_flex")]
    pub cited_by_count: Option<String>,
    /// Any additional fields not modelled above.
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// A grouping of cross-references to a single external database from the
/// Europe PMC `databaseLinks` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EuropePmcDatabaseLink {
    /// External database name (e.g. `"UNIPROT"`, `"EMBL"`, `"PDB"`).
    #[serde(default)]
    pub db_name: Option<String>,
    /// Number of cross-references to this database.
    #[serde(default)]
    pub db_count: Option<u32>,
    /// Individual cross-reference entries.
    #[serde(rename = "dbCrossReferenceInfo", default)]
    pub info: Vec<EuropePmcDbCrossReferenceInfo>,
}

/// A single external-database cross-reference entry. The meaning of each `info`
/// slot varies by database; Europe PMC documents them only positionally.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EuropePmcDbCrossReferenceInfo {
    /// First positional value (often the external accession/identifier).
    #[serde(default)]
    pub info1: Option<String>,
    /// Second positional value.
    #[serde(default)]
    pub info2: Option<String>,
    /// Third positional value.
    #[serde(default)]
    pub info3: Option<String>,
    /// Fourth positional value.
    #[serde(default)]
    pub info4: Option<String>,
}
