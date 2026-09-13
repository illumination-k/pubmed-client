//! Europe PMC tools for the PubMed MCP server.
//!
//! Europe PMC (<https://europepmc.org>) complements the NCBI E-utilities: it
//! indexes preprints, patents and agricultural literature alongside PubMed and
//! PMC, serves JATS full text for open-access records, and exposes reference /
//! citation graphs and external database cross-references. None of it requires
//! an API key, which pairs well with this server's unauthenticated default.

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use pubmed_client::{
    EuropePmcCitation, EuropePmcId, EuropePmcReference, EuropePmcSearchOptions,
    ResultType as EuropePmcResultType,
};

use super::common::{internal_error, invalid_params};
use super::output::{SectionOut, sections_out};

/// Level of detail requested from the Europe PMC `search` endpoint.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EuropePmcResultDetail {
    /// Identifiers only.
    IdList,
    /// Core bibliographic fields (default).
    Lite,
    /// Full metadata, including abstracts and citation counts.
    Core,
}

impl EuropePmcResultDetail {
    fn to_result_type(self) -> EuropePmcResultType {
        match self {
            EuropePmcResultDetail::IdList => EuropePmcResultType::IdList,
            EuropePmcResultDetail::Lite => EuropePmcResultType::Lite,
            EuropePmcResultDetail::Core => EuropePmcResultType::Core,
        }
    }
}

/// Maximum number of records a single Europe PMC tool call will return.
const MAX_RESULTS_CAP: usize = 100;

/// Number of characters shown from an abstract in `core` search results.
const ABSTRACT_PREVIEW_CHARS: usize = 300;

/// Resolve the `(source, id)` pair addressed by a tool call.
///
/// Accepts three spellings so an agent does not have to know the Europe PMC
/// addressing scheme up front:
///
/// * a fully-qualified `"SOURCE/ID"` string (e.g. `"PPR/PPR123456"`), which
///   wins over any separate `source` argument;
/// * an explicit `source` plus a bare id;
/// * a bare id alone, where a `PMC`-prefixed id implies the `PMC` source and
///   anything else is treated as a PubMed (`MED`) record.
fn resolve_id(source: Option<&str>, id: &str) -> Result<EuropePmcId, ErrorData> {
    EuropePmcId::resolve(id, source).map_err(|e| invalid_params(e.to_string()))
}

/// Truncate `text` to at most `limit` characters, appending an ellipsis.
///
/// Cuts on a character boundary; a byte-index slice would panic on the
/// multi-byte characters that appear routinely in biomedical abstracts.
fn preview(text: &str, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        Some((idx, _)) => format!("{}...", &text[..idx]),
        None => text.to_string(),
    }
}

/// Request parameters for the `europe_pmc_search` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EuropePmcSearchRequest {
    #[schemars(
        description = "Europe PMC query (e.g., 'malaria vaccine', 'AUTH:\"Smith J\" AND SRC:PPR', 'TITLE:CRISPR')"
    )]
    pub query: String,

    #[schemars(description = "Maximum number of results (default: 10, max: 100)")]
    pub max_results: Option<usize>,

    #[schemars(
        description = "Level of detail: id_list, lite (default), or core (adds abstracts and citation counts)"
    )]
    pub result_type: Option<EuropePmcResultDetail>,

    #[schemars(
        description = "Sort expression accepted by Europe PMC (e.g., 'P_PDATE_D desc' for newest first, 'CITED desc' for most cited)"
    )]
    pub sort: Option<String>,
}

/// One record from a Europe PMC search.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcRecordOut {
    /// Source database code (MED, PMC, PPR, PAT, AGR, CBA).
    pub source: String,
    /// Identifier within that source; `source/id` addresses the record in the
    /// other Europe PMC tools.
    pub id: String,
    /// Record title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Comma-separated author list, as Europe PMC returns it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<String>,
    /// Journal title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    /// Publication year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pub_year: Option<String>,
    /// PubMed ID, when the record has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmid: Option<String>,
    /// PMC ID, when the record has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmcid: Option<String>,
    /// DOI, when the record has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// Whether Europe PMC flags the record as open access.
    pub is_open_access: bool,
    /// Citation count. Only `core` results carry it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cited_by_count: Option<u64>,
    /// First 300 characters of the abstract, ellipsised when cut. Only
    /// `core` results carry an abstract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_preview: Option<String>,
}

/// Structured answer of the `europe_pmc_search` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcSearchOutput {
    /// The query that was sent to Europe PMC.
    pub query: String,
    /// Number of records returned.
    pub count: usize,
    /// The matching records, in the order Europe PMC returned them.
    pub results: Vec<EuropePmcRecordOut>,
}

/// Search Europe PMC across all of its sources.
pub async fn europe_pmc_search(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcSearchRequest>,
) -> Result<Json<EuropePmcSearchOutput>, ErrorData> {
    if params.query.trim().is_empty() {
        return Err(invalid_params("`query` must not be empty"));
    }

    let max = params.max_results.unwrap_or(10).clamp(1, MAX_RESULTS_CAP);
    let detail = params.result_type.unwrap_or(EuropePmcResultDetail::Lite);

    info!(
        query = %params.query,
        max_results = max,
        result_type = ?detail,
        sort = ?params.sort,
        "Searching Europe PMC"
    );

    let opts = EuropePmcSearchOptions {
        result_type: detail.to_result_type(),
        page_size: max as u32,
        sort: params.sort.clone(),
        ..Default::default()
    };

    let results = server
        .client
        .europe_pmc
        .search_all(&params.query, max, &opts)
        .await
        .map_err(|e| internal_error(format!("Europe PMC search failed: {e}")))?;

    let results: Vec<EuropePmcRecordOut> = results
        .iter()
        .map(|record| EuropePmcRecordOut {
            source: record.source.clone(),
            id: record.id.clone(),
            title: record.title.clone(),
            authors: record.author_string.clone(),
            journal: record.journal_title.clone(),
            pub_year: record.pub_year.clone(),
            pmid: record.pmid.clone(),
            pmcid: record.pmcid.clone(),
            doi: record.doi.clone(),
            is_open_access: record.is_open_access.as_deref() == Some("Y"),
            // `resultType=core` carries these in the untyped `extra` map.
            cited_by_count: record.extra.get("citedByCount").and_then(|v| v.as_u64()),
            abstract_preview: record
                .extra
                .get("abstractText")
                .and_then(|v| v.as_str())
                .map(|text| preview(text, ABSTRACT_PREVIEW_CHARS)),
        })
        .collect();

    Ok(Json(EuropePmcSearchOutput {
        query: params.query,
        count: results.len(),
        results,
    }))
}

/// Request parameters for the `europe_pmc_fulltext` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EuropePmcFullTextRequest {
    #[schemars(
        description = "Record id, either bare ('PMC3258128', '33515491') or fully qualified ('PMC/PMC3258128', 'PPR/PPR123456')"
    )]
    pub id: String,

    #[schemars(
        description = "Source database: MED (PubMed), PMC, PPR (preprints), AGR, CBA, PAT. Defaults to PMC for PMC-prefixed ids, otherwise MED."
    )]
    pub source: Option<String>,

    #[schemars(
        description = "Return the raw JATS XML instead of parsed sections (default: false). Required for non-PMC sources."
    )]
    pub raw_xml: Option<bool>,

    #[schemars(description = "Maximum number of top-level sections to return (default: all)")]
    pub max_sections: Option<usize>,
}

/// Structured answer of the `europe_pmc_fulltext` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcFullTextOutput {
    /// The `source/id` that was fetched.
    pub europe_pmc_id: String,
    /// PMC ID of the parsed article.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmc_id: Option<String>,
    /// Article title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// DOI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// Journal name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    /// Author names, in author order.
    pub authors: Vec<String>,
    /// Abstract text, flattened across its parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    /// Number of top-level body sections, before `max_sections` is applied.
    pub section_count: usize,
    /// Body sections with their subsections nested. Empty when `raw_xml` was
    /// requested.
    pub sections: Vec<SectionOut>,
    /// The unparsed JATS XML, present only when `raw_xml` was requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_xml: Option<String>,
}

/// Fetch the full text of a Europe PMC record.
pub async fn europe_pmc_fulltext(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcFullTextRequest>,
) -> Result<Json<EuropePmcFullTextOutput>, ErrorData> {
    let id = resolve_id(params.source.as_deref(), &params.id)?;

    info!(id = %id, raw_xml = ?params.raw_xml, "Fetching Europe PMC full text");

    if params.raw_xml.unwrap_or(false) {
        let xml = server
            .client
            .europe_pmc
            .fetch_full_text_xml(&id)
            .await
            .map_err(|e| internal_error(format!("Failed to fetch Europe PMC full text: {e}")))?;
        return Ok(Json(EuropePmcFullTextOutput {
            europe_pmc_id: id.to_string(),
            pmc_id: None,
            title: None,
            doi: None,
            journal: None,
            authors: Vec::new(),
            abstract_text: None,
            section_count: 0,
            sections: Vec::new(),
            raw_xml: Some(xml),
        }));
    }

    let article = server
        .client
        .europe_pmc
        .fetch_full_text(&id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch Europe PMC full text: {e}")))?;

    let all_sections = article.sections();
    let shown = match params.max_sections {
        Some(max) => &all_sections[..max.min(all_sections.len())],
        None => all_sections,
    };

    Ok(Json(EuropePmcFullTextOutput {
        europe_pmc_id: id.to_string(),
        pmc_id: Some(article.pmcid().to_string()),
        title: article.title().map(str::to_string),
        doi: article.doi().map(str::to_string),
        journal: article.journal().title.clone(),
        authors: article
            .authors()
            .iter()
            .map(|author| author.full_name.clone())
            .collect(),
        abstract_text: article.abstract_text().map(str::to_string),
        section_count: all_sections.len(),
        sections: sections_out(shown),
        raw_xml: None,
    }))
}

/// Request parameters for the `europe_pmc_references` and
/// `europe_pmc_citations` tools.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EuropePmcCitationGraphRequest {
    #[schemars(
        description = "Record id, either bare ('PMC3258128', '33515491') or fully qualified ('MED/33515491')"
    )]
    pub id: String,

    #[schemars(
        description = "Source database: MED (PubMed), PMC, PPR (preprints), AGR, CBA, PAT. Defaults to PMC for PMC-prefixed ids, otherwise MED."
    )]
    pub source: Option<String>,

    #[schemars(description = "Maximum number of entries to return (default: 50, max: 100)")]
    pub max_results: Option<usize>,
}

/// One work in a Europe PMC citation graph: either a work the record cites,
/// or an article citing it.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcWorkOut {
    /// Source database code of the matched record, when Europe PMC resolved
    /// the entry to one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Identifier within that source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Title of the work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Comma-separated author list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<String>,
    /// Abbreviated journal name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal: Option<String>,
    /// Publication year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pub_year: Option<String>,
    /// Journal volume.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Journal issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Page range or electronic location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_info: Option<String>,
    /// PubMed ID, when Europe PMC matched one. References carry it directly;
    /// for citations it is the `id` of a `MED` record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmid: Option<String>,
    /// DOI, when the entry carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// How often this work has itself been cited. Only citations carry it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cited_by_count: Option<String>,
}

impl From<&EuropePmcReference> for EuropePmcWorkOut {
    fn from(reference: &EuropePmcReference) -> Self {
        Self {
            source: reference.source.clone(),
            id: reference.id.clone(),
            title: reference.title.clone(),
            authors: reference.author_string.clone(),
            journal: reference.journal_abbreviation.clone(),
            pub_year: reference.pub_year.clone(),
            volume: reference.volume.clone(),
            issue: reference.issue.clone(),
            page_info: reference.page_info.clone(),
            pmid: reference.pmid.clone(),
            doi: reference.doi.clone(),
            cited_by_count: None,
        }
    }
}

impl From<&EuropePmcCitation> for EuropePmcWorkOut {
    fn from(citation: &EuropePmcCitation) -> Self {
        Self {
            source: citation.source.clone(),
            id: citation.id.clone(),
            title: citation.title.clone(),
            authors: citation.author_string.clone(),
            journal: citation.journal_abbreviation.clone(),
            pub_year: citation.pub_year.clone(),
            volume: citation.volume.clone(),
            issue: citation.issue.clone(),
            page_info: citation.page_info.clone(),
            // The citations endpoint has no dedicated pmid field: a citing
            // record in MED *is* addressed by its PMID.
            pmid: (citation.source.as_deref() == Some("MED"))
                .then(|| citation.id.clone())
                .flatten(),
            doi: None,
            cited_by_count: citation.cited_by_count.clone(),
        }
    }
}

/// Structured answer of the `europe_pmc_references` and
/// `europe_pmc_citations` tools.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcCitationGraphOutput {
    /// The `source/id` that was queried.
    pub europe_pmc_id: String,
    /// Total number of entries Europe PMC holds, before `max_results`.
    pub total: usize,
    /// Number of entries returned.
    pub count: usize,
    /// The entries, in the order Europe PMC returned them.
    pub entries: Vec<EuropePmcWorkOut>,
}

/// List the works cited by a Europe PMC record.
pub async fn europe_pmc_references(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcCitationGraphRequest>,
) -> Result<Json<EuropePmcCitationGraphOutput>, ErrorData> {
    let id = resolve_id(params.source.as_deref(), &params.id)?;
    let max = params.max_results.unwrap_or(50).clamp(1, MAX_RESULTS_CAP);

    info!(id = %id, max_results = max, "Fetching Europe PMC references");

    let references = server
        .client
        .europe_pmc
        .get_references(&id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch Europe PMC references: {e}")))?;

    let total = references.len();
    let entries: Vec<EuropePmcWorkOut> = references
        .iter()
        .take(max)
        .map(EuropePmcWorkOut::from)
        .collect();

    Ok(Json(EuropePmcCitationGraphOutput {
        europe_pmc_id: id.to_string(),
        total,
        count: entries.len(),
        entries,
    }))
}

/// List the articles citing a Europe PMC record.
pub async fn europe_pmc_citations(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcCitationGraphRequest>,
) -> Result<Json<EuropePmcCitationGraphOutput>, ErrorData> {
    let id = resolve_id(params.source.as_deref(), &params.id)?;
    let max = params.max_results.unwrap_or(50).clamp(1, MAX_RESULTS_CAP);

    info!(id = %id, max_results = max, "Fetching Europe PMC citations");

    let citations = server
        .client
        .europe_pmc
        .get_citations(&id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch Europe PMC citations: {e}")))?;

    let total = citations.len();
    let entries: Vec<EuropePmcWorkOut> = citations
        .iter()
        .take(max)
        .map(EuropePmcWorkOut::from)
        .collect();

    Ok(Json(EuropePmcCitationGraphOutput {
        europe_pmc_id: id.to_string(),
        total,
        count: entries.len(),
        entries,
    }))
}

/// Request parameters for the `europe_pmc_database_links` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EuropePmcDatabaseLinksRequest {
    #[schemars(
        description = "Record id, either bare ('PMC3258128', '33515491') or fully qualified ('MED/33515491')"
    )]
    pub id: String,

    #[schemars(
        description = "Source database: MED (PubMed), PMC, PPR (preprints), AGR, CBA, PAT. Defaults to PMC for PMC-prefixed ids, otherwise MED."
    )]
    pub source: Option<String>,

    #[schemars(
        description = "Filter to a single external database by name (e.g., 'UNIPROT', 'PDB', 'EMBL')"
    )]
    pub db_name: Option<String>,

    #[schemars(
        description = "Maximum number of cross-reference entries to show per database (default: 20)"
    )]
    pub max_entries_per_db: Option<usize>,
}

/// One cross-reference to an external database.
///
/// Europe PMC documents the four `info` slots only positionally, and their
/// meaning varies by database, so they are passed through unlabelled rather
/// than guessed at.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcCrossReferenceOut {
    /// First positional value, usually the external accession.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info1: Option<String>,
    /// Second positional value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info2: Option<String>,
    /// Third positional value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info3: Option<String>,
    /// Fourth positional value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info4: Option<String>,
}

/// Cross-references from one record into one external database.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcDatabaseLinkOut {
    /// External database name (e.g. "UNIPROT", "EMBL", "PDB").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_name: Option<String>,
    /// Total number of cross-references to this database.
    pub total: usize,
    /// The entries returned, capped by `max_entries_per_db`.
    pub entries: Vec<EuropePmcCrossReferenceOut>,
}

/// Structured answer of the `europe_pmc_database_links` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EuropePmcDatabaseLinksOutput {
    /// The `source/id` that was queried.
    pub europe_pmc_id: String,
    /// Number of external databases reported.
    pub count: usize,
    /// One group per external database.
    pub databases: Vec<EuropePmcDatabaseLinkOut>,
}

/// List external database cross-references for a Europe PMC record.
pub async fn europe_pmc_database_links(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcDatabaseLinksRequest>,
) -> Result<Json<EuropePmcDatabaseLinksOutput>, ErrorData> {
    let id = resolve_id(params.source.as_deref(), &params.id)?;
    let max_entries = params.max_entries_per_db.unwrap_or(20).max(1);

    info!(id = %id, db_name = ?params.db_name, "Fetching Europe PMC database links");

    let links = server
        .client
        .europe_pmc
        .get_database_links(&id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch Europe PMC database links: {e}")))?;

    let filter = params.db_name.as_deref().map(str::to_ascii_uppercase);
    let databases: Vec<EuropePmcDatabaseLinkOut> = links
        .iter()
        .filter(|link| match (&filter, link.db_name.as_deref()) {
            (Some(filter), Some(name)) => name.to_ascii_uppercase() == *filter,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .map(|link| EuropePmcDatabaseLinkOut {
            db_name: link.db_name.clone(),
            // `dbCount` is Europe PMC's own total; fall back to what actually
            // arrived when the field is absent.
            total: link
                .db_count
                .map_or(link.info.len(), |count| count as usize),
            entries: link
                .info
                .iter()
                .take(max_entries)
                .map(|entry| EuropePmcCrossReferenceOut {
                    info1: entry.info1.clone(),
                    info2: entry.info2.clone(),
                    info3: entry.info3.clone(),
                    info4: entry.info4.clone(),
                })
                .collect(),
        })
        .collect();

    Ok(Json(EuropePmcDatabaseLinksOutput {
        europe_pmc_id: id.to_string(),
        count: databases.len(),
        databases,
    }))
}

#[cfg(test)]
mod tests {
    use pubmed_client::EuropePmcSource;

    use super::*;

    #[test]
    fn bare_pmc_id_defaults_to_pmc_source() {
        let id = resolve_id(None, "PMC3258128").unwrap();
        assert_eq!(id.source, EuropePmcSource::Pmc);
        assert_eq!(id.id, "PMC3258128");
    }

    #[test]
    fn bare_numeric_id_defaults_to_med_source() {
        let id = resolve_id(None, "33515491").unwrap();
        assert_eq!(id.source, EuropePmcSource::Med);
        assert_eq!(id.to_string(), "MED/33515491");
    }

    #[test]
    fn explicit_pmc_source_normalizes_a_bare_number() {
        let id = resolve_id(Some("pmc"), "3258128").unwrap();
        assert_eq!(id.to_string(), "PMC/PMC3258128");
    }

    #[test]
    fn qualified_id_wins_over_source_argument() {
        let id = resolve_id(Some("MED"), "PPR/PPR123456").unwrap();
        assert_eq!(id.source, EuropePmcSource::Ppr);
        assert_eq!(id.id, "PPR123456");
    }

    #[test]
    fn unknown_source_is_passed_through() {
        let id = resolve_id(Some("xyz"), "42").unwrap();
        assert_eq!(id.to_string(), "XYZ/42");
    }

    #[test]
    fn empty_id_is_rejected() {
        assert!(resolve_id(None, "   ").is_err());
    }

    #[test]
    fn malformed_qualified_id_is_rejected() {
        assert!(resolve_id(None, "MED/").is_err());
    }

    #[test]
    fn preview_truncates_on_a_character_boundary() {
        // Each 'µ' is two bytes, so a byte-index slice at 10 would panic.
        let text = "µ".repeat(20);
        assert_eq!(preview(&text, 10), format!("{}...", "µ".repeat(10)));
        assert_eq!(preview("short", 10), "short");
    }

    #[test]
    fn a_med_citation_exposes_its_id_as_a_pmid() {
        let citation = EuropePmcCitation {
            id: Some("33515491".to_string()),
            source: Some("MED".to_string()),
            ..Default::default()
        };
        assert_eq!(
            EuropePmcWorkOut::from(&citation).pmid.as_deref(),
            Some("33515491")
        );

        // A preprint id is not a PMID, so it must not be reported as one.
        let preprint = EuropePmcCitation {
            id: Some("PPR123456".to_string()),
            source: Some("PPR".to_string()),
            ..Default::default()
        };
        assert_eq!(EuropePmcWorkOut::from(&preprint).pmid, None);
    }
}
