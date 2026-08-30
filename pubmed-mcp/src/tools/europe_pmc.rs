//! Europe PMC tools for the PubMed MCP server.
//!
//! Europe PMC (<https://europepmc.org>) complements the NCBI E-utilities: it
//! indexes preprints, patents and agricultural literature alongside PubMed and
//! PMC, serves JATS full text for open-access records, and exposes reference /
//! citation graphs and external database cross-references. None of it requires
//! an API key, which pairs well with this server's unauthenticated default.

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use tracing::info;

use pubmed_client::{EuropePmcId, EuropePmcSearchOptions, ResultType as EuropePmcResultType};

use super::common::{internal_error, invalid_params, text_result};

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

/// Render an optional field as a labelled line, skipping empty values.
fn push_field(out: &mut String, label: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) {
        out.push_str(&format!("   {label}: {value}\n"));
    }
}

/// Format the `volume(issue):pages` locator shared by references and citations.
fn locator(volume: Option<&str>, issue: Option<&str>, page_info: Option<&str>) -> Option<String> {
    let mut locator = String::new();
    if let Some(volume) = volume {
        locator.push_str(volume);
    }
    if let Some(issue) = issue {
        locator.push_str(&format!("({issue})"));
    }
    if let Some(pages) = page_info {
        if !locator.is_empty() {
            locator.push(':');
        }
        locator.push_str(pages);
    }
    (!locator.is_empty()).then_some(locator)
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

/// Search Europe PMC across all of its sources.
pub async fn europe_pmc_search(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcSearchRequest>,
) -> Result<CallToolResult, ErrorData> {
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

    let mut out = format!("Found {} Europe PMC records:\n\n", results.len());

    for (i, record) in results.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} ({}/{})\n",
            i + 1,
            record.title.as_deref().unwrap_or("Untitled"),
            record.source,
            record.id
        ));
        push_field(&mut out, "Authors", record.author_string.as_deref());
        push_field(&mut out, "Journal", record.journal_title.as_deref());
        push_field(&mut out, "Year", record.pub_year.as_deref());
        push_field(&mut out, "PMID", record.pmid.as_deref());
        push_field(&mut out, "PMC", record.pmcid.as_deref());
        push_field(&mut out, "DOI", record.doi.as_deref());
        if record.is_open_access.as_deref() == Some("Y") {
            out.push_str("   Open access: yes\n");
        }
        // `resultType=core` carries these in the untyped `extra` map.
        if let Some(cited_by) = record.extra.get("citedByCount") {
            push_field(&mut out, "Cited by", Some(&cited_by.to_string()));
        }
        if let Some(abstract_text) = record.extra.get("abstractText").and_then(|v| v.as_str()) {
            push_field(
                &mut out,
                "Abstract",
                Some(&preview(abstract_text, ABSTRACT_PREVIEW_CHARS)),
            );
        }
        out.push('\n');
    }

    text_result(out)
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

    #[schemars(description = "Maximum number of sections to return (default: all)")]
    pub max_sections: Option<usize>,
}

/// Fetch the full text of a Europe PMC record.
pub async fn europe_pmc_fulltext(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcFullTextRequest>,
) -> Result<CallToolResult, ErrorData> {
    let id = resolve_id(params.source.as_deref(), &params.id)?;

    info!(id = %id, raw_xml = ?params.raw_xml, "Fetching Europe PMC full text");

    if params.raw_xml.unwrap_or(false) {
        let xml = server
            .client
            .europe_pmc
            .fetch_full_text_xml(&id)
            .await
            .map_err(|e| internal_error(format!("Failed to fetch Europe PMC full text: {e}")))?;
        return text_result(xml);
    }

    let article = server
        .client
        .europe_pmc
        .fetch_full_text(&id)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch Europe PMC full text: {e}")))?;

    let mut out = String::new();
    out.push_str(&format!(
        "Title: {}\n",
        article.title().unwrap_or("Untitled")
    ));
    out.push_str(&format!("Europe PMC ID: {id}\n"));
    out.push_str(&format!("PMC ID: {}\n", article.pmcid()));
    if let Some(doi) = article.doi() {
        out.push_str(&format!("DOI: {doi}\n"));
    }
    if !article.authors().is_empty() {
        let authors: Vec<&str> = article
            .authors()
            .iter()
            .map(|a| a.full_name.as_str())
            .collect();
        out.push_str(&format!("Authors: {}\n", authors.join(", ")));
    }
    if let Some(journal) = article.journal().title.as_deref() {
        out.push_str(&format!("Journal: {journal}\n"));
    }

    let sections = article.sections();
    let shown = match params.max_sections {
        Some(max) => &sections[..max.min(sections.len())],
        None => sections,
    };
    for section in shown {
        let title = section
            .title
            .as_deref()
            .or(section.section_type.as_deref())
            .unwrap_or("Untitled");
        out.push_str(&format!("\n## {title}\n{}\n", section.content));
    }

    text_result(out)
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

/// List the works cited by a Europe PMC record.
pub async fn europe_pmc_references(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcCitationGraphRequest>,
) -> Result<CallToolResult, ErrorData> {
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
    let mut out = format!("{id} cites {total} works (showing {}):\n\n", max.min(total));

    for (i, reference) in references.iter().take(max).enumerate() {
        out.push_str(&format!(
            "{}. {}\n",
            i + 1,
            reference.title.as_deref().unwrap_or("Untitled")
        ));
        push_field(&mut out, "Authors", reference.author_string.as_deref());
        push_field(
            &mut out,
            "Journal",
            reference.journal_abbreviation.as_deref(),
        );
        push_field(&mut out, "Year", reference.pub_year.as_deref());
        push_field(
            &mut out,
            "Location",
            locator(
                reference.volume.as_deref(),
                reference.issue.as_deref(),
                reference.page_info.as_deref(),
            )
            .as_deref(),
        );
        push_field(&mut out, "PMID", reference.pmid.as_deref());
        push_field(&mut out, "DOI", reference.doi.as_deref());
    }

    if total > max {
        out.push_str(&format!("\n... and {} more\n", total - max));
    }

    text_result(out)
}

/// List the articles citing a Europe PMC record.
pub async fn europe_pmc_citations(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcCitationGraphRequest>,
) -> Result<CallToolResult, ErrorData> {
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
    let mut out = format!(
        "{id} is cited by {total} articles (showing {}):\n\n",
        max.min(total)
    );

    for (i, citation) in citations.iter().take(max).enumerate() {
        out.push_str(&format!(
            "{}. {}\n",
            i + 1,
            citation.title.as_deref().unwrap_or("Untitled")
        ));
        push_field(&mut out, "Authors", citation.author_string.as_deref());
        push_field(
            &mut out,
            "Journal",
            citation.journal_abbreviation.as_deref(),
        );
        push_field(&mut out, "Year", citation.pub_year.as_deref());
        push_field(
            &mut out,
            "Location",
            locator(
                citation.volume.as_deref(),
                citation.issue.as_deref(),
                citation.page_info.as_deref(),
            )
            .as_deref(),
        );
        if let (Some(source), Some(cited_id)) = (citation.source.as_deref(), citation.id.as_deref())
        {
            push_field(
                &mut out,
                "Europe PMC ID",
                Some(&format!("{source}/{cited_id}")),
            );
        }
        push_field(&mut out, "Cited by", citation.cited_by_count.as_deref());
    }

    if total > max {
        out.push_str(&format!("\n... and {} more\n", total - max));
    }

    text_result(out)
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

/// List external database cross-references for a Europe PMC record.
pub async fn europe_pmc_database_links(
    server: &super::PubMedServer,
    Parameters(params): Parameters<EuropePmcDatabaseLinksRequest>,
) -> Result<CallToolResult, ErrorData> {
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
    let links: Vec<_> = links
        .iter()
        .filter(|link| match (&filter, link.db_name.as_deref()) {
            (Some(filter), Some(name)) => name.to_ascii_uppercase() == *filter,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .collect();

    if links.is_empty() {
        return text_result(format!("{id} has no external database cross-references.\n"));
    }

    let mut out = format!("{id} links to {} external database(s):\n\n", links.len());
    for link in links {
        let name = link.db_name.as_deref().unwrap_or("Unknown database");
        let count = link.db_count.unwrap_or(link.info.len() as u32);
        out.push_str(&format!("## {name} ({count} cross-reference(s))\n"));
        for entry in link.info.iter().take(max_entries) {
            // Europe PMC documents the four `info` slots only positionally, so
            // render whichever are populated rather than guessing at labels.
            let values: Vec<&str> = [
                entry.info1.as_deref(),
                entry.info2.as_deref(),
                entry.info3.as_deref(),
                entry.info4.as_deref(),
            ]
            .into_iter()
            .flatten()
            .filter(|v| !v.trim().is_empty())
            .collect();
            if !values.is_empty() {
                out.push_str(&format!("- {}\n", values.join(" | ")));
            }
        }
        if link.info.len() > max_entries {
            out.push_str(&format!("... and {} more\n", link.info.len() - max_entries));
        }
        out.push('\n');
    }

    text_result(out)
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
    fn locator_joins_only_the_present_parts() {
        assert_eq!(
            locator(Some("5"), Some("2"), Some("100-110")).as_deref(),
            Some("5(2):100-110")
        );
        assert_eq!(locator(Some("5"), None, None).as_deref(), Some("5"));
        assert_eq!(locator(None, None, Some("e1234")).as_deref(), Some("e1234"));
        assert_eq!(locator(None, None, None), None);
    }
}
