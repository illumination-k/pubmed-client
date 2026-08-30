//! Europe PMC models and `(source, id)` addressing for the Node.js bindings.
//!
//! Europe PMC (<https://europepmc.org>) complements the NCBI E-utilities: it
//! indexes preprints, patents and agricultural literature alongside PubMed and
//! PMC, and requires no API key.

use napi::bindgen_prelude::*;
use napi_derive::napi;

use pubmed_client::{
    EuropePmcCitation, EuropePmcDatabaseLink, EuropePmcDbCrossReferenceInfo, EuropePmcId,
    EuropePmcReference, EuropePmcResult, EuropePmcSearchResponse, EuropePmcSource, ResultType,
};

/// Resolve the `(source, id)` pair a call addresses.
///
/// Europe PMC identifies every record by a source database plus an id. Three
/// spellings are accepted so callers rarely need to pass both:
///
/// * a fully-qualified `"SOURCE/ID"` string (e.g. `"PPR/PPR123456"`), which
///   wins over any separate `source` argument;
/// * an explicit `source` plus a bare id;
/// * a bare id alone — a `PMC`-prefixed id implies the `PMC` source, anything
///   else is treated as a PubMed (`MED`) record.
pub(crate) fn resolve_id(id: &str, source: Option<&str>) -> Result<EuropePmcId> {
    let id = id.trim();
    if id.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "id must not be empty".to_string(),
        ));
    }

    if id.contains('/') {
        return id
            .parse::<EuropePmcId>()
            .map_err(|e| Error::new(Status::InvalidArg, format!("invalid Europe PMC id: {e}")));
    }

    let source = match source {
        Some(source) if !source.trim().is_empty() => EuropePmcSource::from(source),
        _ if id.to_ascii_uppercase().starts_with("PMC") => EuropePmcSource::Pmc,
        _ => EuropePmcSource::Med,
    };

    if source == EuropePmcSource::Pmc {
        return EuropePmcId::pmc(id)
            .map_err(|e| Error::new(Status::InvalidArg, format!("invalid PMC id: {e}")));
    }

    Ok(EuropePmcId::new(source, id))
}

/// Map a `resultType` string onto the level of detail Europe PMC understands.
pub(crate) fn parse_result_type(result_type: Option<&str>) -> Result<ResultType> {
    match result_type
        .map(str::trim)
        .unwrap_or("lite")
        .to_lowercase()
        .as_str()
    {
        "idlist" | "id_list" => Ok(ResultType::IdList),
        "lite" => Ok(ResultType::Lite),
        "core" => Ok(ResultType::Core),
        other => Err(Error::new(
            Status::InvalidArg,
            format!("invalid resultType '{other}': expected 'idlist', 'lite' or 'core'"),
        )),
    }
}

/// Serialize the unmodelled remainder of a Europe PMC record to a JSON object
/// string.
///
/// `resultType=core` returns dozens of fields beyond the modelled ones, and the
/// set changes over time; handing callers a JSON string to `JSON.parse` keeps
/// them reachable without pinning a shape Europe PMC is free to change.
/// Serialization of an already-parsed JSON map cannot fail, so a failure falls
/// back to an empty object rather than an error the caller cannot act on.
fn extra_json(extra: &serde_json::Map<String, serde_json::Value>) -> String {
    serde_json::to_string(extra).unwrap_or_else(|_| "{}".to_string())
}

/// A Europe PMC search result record
#[napi(object)]
pub struct EuropePmcSearchResult {
    /// Record identifier within its source database
    pub id: String,
    /// Source database code (MED, PMC, PPR, AGR, CBA, PAT, ...)
    pub source: String,
    /// Fully-qualified Europe PMC address ("SOURCE/ID")
    pub europe_pmc_id: String,
    /// PubMed ID, when the record is linked to PubMed
    pub pmid: Option<String>,
    /// PMC ID, when full text is in PMC
    pub pmcid: Option<String>,
    /// Digital Object Identifier
    pub doi: Option<String>,
    /// Article title
    pub title: Option<String>,
    /// Comma-separated author list, as provided by Europe PMC
    pub author_string: Option<String>,
    /// Journal title
    pub journal_title: Option<String>,
    /// Publication year
    pub pub_year: Option<String>,
    /// Open access flag as reported by Europe PMC ("Y" / "N")
    pub is_open_access: Option<String>,
    /// Fields Europe PMC returned but that are not modelled above, as a JSON
    /// object string. Populated mainly by `resultType: 'core'`.
    pub extra_json: String,
}

impl From<EuropePmcResult> for EuropePmcSearchResult {
    fn from(result: EuropePmcResult) -> Self {
        EuropePmcSearchResult {
            europe_pmc_id: format!("{}/{}", result.source, result.id),
            id: result.id,
            source: result.source,
            pmid: result.pmid,
            pmcid: result.pmcid,
            doi: result.doi,
            title: result.title,
            author_string: result.author_string,
            journal_title: result.journal_title,
            pub_year: result.pub_year,
            is_open_access: result.is_open_access,
            extra_json: extra_json(&result.extra),
        }
    }
}

/// One page of Europe PMC search results
#[napi(object)]
pub struct EuropePmcSearchPage {
    /// Total number of records matching the query, across all pages
    pub hit_count: i64,
    /// Cursor to pass as `cursorMark` to fetch the next page.
    ///
    /// Europe PMC keeps returning the same value once the last page is
    /// reached, so a cursor equal to the one just used means "no more pages".
    pub next_cursor_mark: Option<String>,
    /// Records on this page
    pub results: Vec<EuropePmcSearchResult>,
}

impl From<EuropePmcSearchResponse> for EuropePmcSearchPage {
    fn from(response: EuropePmcSearchResponse) -> Self {
        EuropePmcSearchPage {
            // Europe PMC's hitCount is well within i64; napi has no u64 object
            // field type, and i64 maps to a JS number.
            hit_count: response.hit_count as i64,
            next_cursor_mark: response.next_cursor_mark,
            results: response
                .results
                .into_iter()
                .map(EuropePmcSearchResult::from)
                .collect(),
        }
    }
}

/// A work cited by a Europe PMC record
#[napi(object)]
pub struct EuropePmcReferenceEntry {
    /// Source database of the cited record, when Europe PMC matched it
    pub source: Option<String>,
    /// Identifier of the cited record, when Europe PMC matched it
    pub id: Option<String>,
    /// Citation type (e.g. "JOURNAL ARTICLE")
    pub citation_type: Option<String>,
    pub title: Option<String>,
    pub author_string: Option<String>,
    pub journal_abbreviation: Option<String>,
    pub pub_year: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub page_info: Option<String>,
    pub pmid: Option<String>,
    pub doi: Option<String>,
    /// Fields not modelled above, as a JSON object string
    pub extra_json: String,
}

impl From<EuropePmcReference> for EuropePmcReferenceEntry {
    fn from(reference: EuropePmcReference) -> Self {
        EuropePmcReferenceEntry {
            source: reference.source,
            id: reference.id,
            citation_type: reference.citation_type,
            title: reference.title,
            author_string: reference.author_string,
            journal_abbreviation: reference.journal_abbreviation,
            pub_year: reference.pub_year,
            volume: reference.volume,
            issue: reference.issue,
            page_info: reference.page_info,
            pmid: reference.pmid,
            doi: reference.doi,
            extra_json: extra_json(&reference.extra),
        }
    }
}

/// An article citing a Europe PMC record
#[napi(object)]
pub struct EuropePmcCitationEntry {
    /// Identifier of the citing record within its source database
    pub id: Option<String>,
    /// Source database of the citing record
    pub source: Option<String>,
    /// Citation type (e.g. "JOURNAL ARTICLE")
    pub citation_type: Option<String>,
    pub title: Option<String>,
    pub author_string: Option<String>,
    pub journal_abbreviation: Option<String>,
    pub pub_year: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub page_info: Option<String>,
    /// Number of times the citing article has itself been cited
    pub cited_by_count: Option<String>,
    /// Fields not modelled above, as a JSON object string
    pub extra_json: String,
}

impl From<EuropePmcCitation> for EuropePmcCitationEntry {
    fn from(citation: EuropePmcCitation) -> Self {
        EuropePmcCitationEntry {
            id: citation.id,
            source: citation.source,
            citation_type: citation.citation_type,
            title: citation.title,
            author_string: citation.author_string,
            journal_abbreviation: citation.journal_abbreviation,
            pub_year: citation.pub_year,
            volume: citation.volume,
            issue: citation.issue,
            page_info: citation.page_info,
            cited_by_count: citation.cited_by_count,
            extra_json: extra_json(&citation.extra),
        }
    }
}

/// A single external-database cross-reference entry
///
/// Europe PMC documents the four slots only positionally, and their meaning
/// varies by database, so they are surfaced as-is rather than renamed.
#[napi(object)]
pub struct EuropePmcDbCrossReference {
    pub info1: Option<String>,
    pub info2: Option<String>,
    pub info3: Option<String>,
    pub info4: Option<String>,
}

impl From<EuropePmcDbCrossReferenceInfo> for EuropePmcDbCrossReference {
    fn from(info: EuropePmcDbCrossReferenceInfo) -> Self {
        EuropePmcDbCrossReference {
            info1: info.info1,
            info2: info.info2,
            info3: info.info3,
            info4: info.info4,
        }
    }
}

/// Cross-references from a record to one external database
#[napi(object)]
pub struct EuropePmcDatabaseLinkEntry {
    /// External database name (e.g. "UNIPROT", "EMBL", "PDB")
    pub db_name: Option<String>,
    /// Number of cross-references reported for this database
    pub db_count: Option<u32>,
    /// Individual cross-reference entries
    pub info: Vec<EuropePmcDbCrossReference>,
}

impl From<EuropePmcDatabaseLink> for EuropePmcDatabaseLinkEntry {
    fn from(link: EuropePmcDatabaseLink) -> Self {
        EuropePmcDatabaseLinkEntry {
            db_name: link.db_name,
            db_count: link.db_count,
            info: link
                .info
                .into_iter()
                .map(EuropePmcDbCrossReference::from)
                .collect(),
        }
    }
}
