//! R bindings for the PubMed / PMC client library, built with extendr.
//!
//! The public surface is intentionally small (an MVP): create a client, search
//! PubMed, fetch article metadata, retrieve PMC full text / Markdown, and reach
//! Europe PMC for cross-source search and citation graphs. The client handle is
//! exposed to R as an [`ExternalPtr`] and threaded through the free functions
//! below; the ergonomic R API lives in `R/pubmed-client.R`.
//!
//! Like the Python bindings, all calls are synchronous from the caller's point
//! of view: a process-wide Tokio runtime drives the async `pubmed-client` API
//! and blocks until completion.

use std::sync::Arc;
use std::sync::OnceLock;

use extendr_api::prelude::*;
use tokio::runtime::Runtime;

use pubmed_client::{
    Client, ClientConfig, EuropePmcCitation, EuropePmcDatabaseLink, EuropePmcId,
    EuropePmcReference, EuropePmcResult, EuropePmcSearchOptions, EuropePmcSource, PmcArticle,
    PmcMarkdownConverter, PubMedArticle, ResultType,
};

// ------------------------------------------------------------------------------------------------
// Runtime management
// ------------------------------------------------------------------------------------------------

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Get or create the process-wide Tokio runtime used to block on async calls.
///
/// A single shared runtime keeps connection pools and the NCBI rate limiter
/// alive across calls, mirroring the Python bindings.
#[allow(clippy::expect_used)]
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("failed to create Tokio runtime"))
}

/// Map a `pubmed-client` error into an extendr error so it surfaces in R as a
/// normal `stop()` condition with a readable message.
fn to_r_err<T>(result: pubmed_client::error::Result<T>) -> Result<T> {
    result.map_err(|e| Error::Other(e.to_string()))
}

// ------------------------------------------------------------------------------------------------
// Client handle
// ------------------------------------------------------------------------------------------------

/// Opaque client handle stored behind an R external pointer.
struct ClientHandle {
    inner: Arc<Client>,
}

/// Create a new client.
///
/// All configuration arguments are optional (`NULL` in R); omitted values fall
/// back to the library defaults.
#[extendr]
fn client_new(
    api_key: Option<String>,
    email: Option<String>,
    tool: Option<String>,
    rate_limit: Option<f64>,
    timeout_seconds: Option<f64>,
) -> ExternalPtr<ClientHandle> {
    let mut config = ClientConfig::new();
    if let Some(api_key) = api_key {
        config = config.with_api_key(api_key);
    }
    if let Some(email) = email {
        config = config.with_email(email);
    }
    if let Some(tool) = tool {
        config = config.with_tool(tool);
    }
    if let Some(rate_limit) = rate_limit {
        config = config.with_rate_limit(rate_limit);
    }
    if let Some(timeout_seconds) = timeout_seconds {
        config = config.with_timeout_seconds(timeout_seconds as u64);
    }

    ExternalPtr::new(ClientHandle {
        inner: Arc::new(Client::with_config(config)),
    })
}

// ------------------------------------------------------------------------------------------------
// Conversions to R objects
// ------------------------------------------------------------------------------------------------

/// Convert an optional string into an R value (`NULL` when absent).
fn opt_str(value: Option<String>) -> Robj {
    match value {
        Some(v) => v.into(),
        None => r!(NULL),
    }
}

/// Convert a [`PubMedArticle`] into a named R list.
fn article_to_robj(article: &PubMedArticle) -> Robj {
    let authors: Vec<String> = article
        .authors
        .iter()
        .map(|a| a.full_name.clone())
        .collect();

    list!(
        pmid = article.pmid.clone(),
        title = article.title.clone(),
        journal = article.journal.clone(),
        pub_date = article.pub_date.clone(),
        doi = opt_str(article.doi.clone()),
        pmc_id = opt_str(article.pmc_id.clone()),
        abstract_text = opt_str(article.abstract_text.clone()),
        author_count = article.author_count as i32,
        authors = authors,
        volume = opt_str(article.volume.clone()),
        issue = opt_str(article.issue.clone()),
        pages = opt_str(article.pages.clone()),
        language = opt_str(article.language.clone()),
        journal_abbreviation = opt_str(article.journal_abbreviation.clone()),
        issn = opt_str(article.issn.clone()),
        keywords = article.keywords.clone().unwrap_or_default(),
        article_types = article.article_types.clone()
    )
    .into()
}

/// Convert a [`PmcArticle`] into a named R list of summary metadata.
fn fulltext_to_robj(article: &PmcArticle) -> Robj {
    list!(
        pmcid = article.pmcid().to_string(),
        pmid = opt_str(article.pmid().map(|p| p.to_string())),
        title = opt_str(article.title().map(|t| t.to_string())),
        doi = opt_str(article.doi().map(|d| d.to_string())),
        author_count = article.authors().len() as i32,
        section_count = article.sections().len() as i32,
        reference_count = article.references().len() as i32
    )
    .into()
}

// ------------------------------------------------------------------------------------------------
// PubMed operations
// ------------------------------------------------------------------------------------------------

/// Search PubMed and return the matching PMIDs as a character vector.
#[extendr]
fn client_search_articles(
    client: ExternalPtr<ClientHandle>,
    query: &str,
    limit: i32,
) -> Result<Vec<String>> {
    let client = client.inner.clone();
    let result = runtime().block_on(client.pubmed.search_articles(query, limit as usize, None));
    to_r_err(result)
}

/// Fetch full metadata for a single article by PMID.
#[extendr]
fn client_fetch_article(client: ExternalPtr<ClientHandle>, pmid: &str) -> Result<Robj> {
    let client = client.inner.clone();
    let result = runtime().block_on(client.pubmed.fetch_article(pmid));
    Ok(article_to_robj(&to_r_err(result)?))
}

/// Fetch full metadata for several PMIDs, returning a list of article lists.
#[extendr]
fn client_fetch_articles(client: ExternalPtr<ClientHandle>, pmids: Vec<String>) -> Result<Robj> {
    let client = client.inner.clone();
    let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
    let result = runtime().block_on(client.pubmed.fetch_articles(&pmid_refs));
    let articles = to_r_err(result)?;
    let items: Vec<Robj> = articles.iter().map(article_to_robj).collect();
    Ok(List::from_values(items).into())
}

/// Search PubMed and fetch metadata for each hit in one call.
#[extendr]
fn client_search_and_fetch(
    client: ExternalPtr<ClientHandle>,
    query: &str,
    limit: i32,
) -> Result<Robj> {
    let client = client.inner.clone();
    let result = runtime().block_on(client.pubmed.search_and_fetch(query, limit as usize, None));
    let articles = to_r_err(result)?;
    let items: Vec<Robj> = articles.iter().map(article_to_robj).collect();
    Ok(List::from_values(items).into())
}

// ------------------------------------------------------------------------------------------------
// PMC operations
// ------------------------------------------------------------------------------------------------

/// Fetch PMC full-text summary metadata for a PMCID.
#[extendr]
fn pmc_fetch_fulltext(client: ExternalPtr<ClientHandle>, pmcid: &str) -> Result<Robj> {
    let client = client.inner.clone();
    let result = runtime().block_on(client.pmc.fetch_full_text(pmcid));
    Ok(fulltext_to_robj(&to_r_err(result)?))
}

/// Fetch a PMC article and render it to Markdown.
#[extendr]
fn pmc_markdown(client: ExternalPtr<ClientHandle>, pmcid: &str) -> Result<String> {
    let client = client.inner.clone();
    let article: PmcArticle = to_r_err(runtime().block_on(client.pmc.fetch_full_text(pmcid)))?;
    let converter = PmcMarkdownConverter::new();
    Ok(converter.convert(&article))
}

// ------------------------------------------------------------------------------------------------
// Europe PMC
// ------------------------------------------------------------------------------------------------

/// Resolve the `(source, id)` pair an R call addresses.
///
/// Europe PMC identifies every record by a source database plus an id. Three
/// spellings are accepted so the common cases need only the id:
///
/// * a fully-qualified `"SOURCE/ID"` string (e.g. `"PPR/PPR123456"`), which
///   wins over any separate `source`;
/// * an explicit `source` plus a bare id;
/// * a bare id alone — a `PMC`-prefixed id implies the `PMC` source, anything
///   else is treated as a PubMed (`MED`) record.
fn resolve_epmc_id(id: &str, source: Option<&str>) -> Result<EuropePmcId> {
    let id = id.trim();
    if id.is_empty() {
        return Err(Error::Other("`id` must not be empty".to_string()));
    }

    if id.contains('/') {
        return id
            .parse::<EuropePmcId>()
            .map_err(|e| Error::Other(format!("invalid Europe PMC id: {e}")));
    }

    let source = match source {
        Some(source) if !source.trim().is_empty() => EuropePmcSource::from(source),
        _ if id.to_ascii_uppercase().starts_with("PMC") => EuropePmcSource::Pmc,
        _ => EuropePmcSource::Med,
    };

    if source == EuropePmcSource::Pmc {
        return EuropePmcId::pmc(id).map_err(|e| Error::Other(format!("invalid PMC id: {e}")));
    }

    Ok(EuropePmcId::new(source, id))
}

/// Map an R `result_type` string onto the level of detail Europe PMC understands.
fn parse_result_type(result_type: Option<&str>) -> Result<ResultType> {
    match result_type
        .map(str::trim)
        .unwrap_or("lite")
        .to_ascii_lowercase()
        .as_str()
    {
        "idlist" | "id_list" => Ok(ResultType::IdList),
        "lite" => Ok(ResultType::Lite),
        "core" => Ok(ResultType::Core),
        other => Err(Error::Other(format!(
            "invalid `result_type` '{other}': expected 'idlist', 'lite' or 'core'"
        ))),
    }
}

/// Serialize a record's unmodelled fields as a JSON object string.
///
/// `result_type = "core"` returns far more than is modelled, and the set changes
/// over time. Handing R a JSON string (to parse with e.g. `jsonlite::fromJSON`)
/// keeps those fields reachable without the package depending on a JSON reader.
/// Serializing an already-parsed JSON map cannot fail, so a failure falls back
/// to an empty object rather than an error the caller cannot act on.
fn extra_json(extra: &serde_json::Map<String, serde_json::Value>) -> String {
    serde_json::to_string(extra).unwrap_or_else(|_| "{}".to_string())
}

/// Convert a [`EuropePmcResult`] into a named R list.
fn epmc_result_to_robj(result: &EuropePmcResult) -> Robj {
    list!(
        id = result.id.clone(),
        source = result.source.clone(),
        europe_pmc_id = format!("{}/{}", result.source, result.id),
        pmid = opt_str(result.pmid.clone()),
        pmcid = opt_str(result.pmcid.clone()),
        doi = opt_str(result.doi.clone()),
        title = opt_str(result.title.clone()),
        author_string = opt_str(result.author_string.clone()),
        journal_title = opt_str(result.journal_title.clone()),
        pub_year = opt_str(result.pub_year.clone()),
        is_open_access = opt_str(result.is_open_access.clone()),
        extra_json = extra_json(&result.extra)
    )
    .into()
}

/// Convert a [`EuropePmcReference`] into a named R list.
fn epmc_reference_to_robj(reference: &EuropePmcReference) -> Robj {
    list!(
        source = opt_str(reference.source.clone()),
        id = opt_str(reference.id.clone()),
        citation_type = opt_str(reference.citation_type.clone()),
        title = opt_str(reference.title.clone()),
        author_string = opt_str(reference.author_string.clone()),
        journal_abbreviation = opt_str(reference.journal_abbreviation.clone()),
        pub_year = opt_str(reference.pub_year.clone()),
        volume = opt_str(reference.volume.clone()),
        issue = opt_str(reference.issue.clone()),
        page_info = opt_str(reference.page_info.clone()),
        pmid = opt_str(reference.pmid.clone()),
        doi = opt_str(reference.doi.clone()),
        extra_json = extra_json(&reference.extra)
    )
    .into()
}

/// Convert a [`EuropePmcCitation`] into a named R list.
fn epmc_citation_to_robj(citation: &EuropePmcCitation) -> Robj {
    list!(
        id = opt_str(citation.id.clone()),
        source = opt_str(citation.source.clone()),
        citation_type = opt_str(citation.citation_type.clone()),
        title = opt_str(citation.title.clone()),
        author_string = opt_str(citation.author_string.clone()),
        journal_abbreviation = opt_str(citation.journal_abbreviation.clone()),
        pub_year = opt_str(citation.pub_year.clone()),
        volume = opt_str(citation.volume.clone()),
        issue = opt_str(citation.issue.clone()),
        page_info = opt_str(citation.page_info.clone()),
        cited_by_count = opt_str(citation.cited_by_count.clone()),
        extra_json = extra_json(&citation.extra)
    )
    .into()
}

/// Convert a [`EuropePmcDatabaseLink`] into a named R list.
///
/// The four `info` slots are documented by Europe PMC only positionally, and
/// their meaning varies by database, so they are carried through as-is.
fn epmc_database_link_to_robj(link: &EuropePmcDatabaseLink) -> Robj {
    let entries: Vec<Robj> = link
        .info
        .iter()
        .map(|info| {
            list!(
                info1 = opt_str(info.info1.clone()),
                info2 = opt_str(info.info2.clone()),
                info3 = opt_str(info.info3.clone()),
                info4 = opt_str(info.info4.clone())
            )
            .into()
        })
        .collect();

    list!(
        db_name = opt_str(link.db_name.clone()),
        db_count = match link.db_count {
            Some(count) => r!(count as i32),
            None => r!(NULL),
        },
        info = List::from_values(entries)
    )
    .into()
}

/// Search Europe PMC, returning a list of record lists.
#[extendr]
fn epmc_search(
    client: ExternalPtr<ClientHandle>,
    query: &str,
    limit: i32,
    result_type: Option<String>,
    sort: Option<String>,
) -> Result<Robj> {
    let options = EuropePmcSearchOptions {
        result_type: parse_result_type(result_type.as_deref())?,
        page_size: (limit.max(1) as u32).clamp(1, 1000),
        sort,
        ..Default::default()
    };

    let client = client.inner.clone();
    let results = to_r_err(runtime().block_on(client.europe_pmc.search_all(
        query,
        limit.max(0) as usize,
        &options,
    )))?;
    let items: Vec<Robj> = results.iter().map(epmc_result_to_robj).collect();
    Ok(List::from_values(items).into())
}

/// Fetch Europe PMC full-text summary metadata for a record.
#[extendr]
fn epmc_fetch_fulltext(
    client: ExternalPtr<ClientHandle>,
    id: &str,
    source: Option<String>,
) -> Result<Robj> {
    let epmc_id = resolve_epmc_id(id, source.as_deref())?;
    let client = client.inner.clone();
    let article = to_r_err(runtime().block_on(client.europe_pmc.fetch_full_text(&epmc_id)))?;
    Ok(fulltext_to_robj(&article))
}

/// Fetch the raw JATS XML for a Europe PMC record.
#[extendr]
fn epmc_fetch_xml(
    client: ExternalPtr<ClientHandle>,
    id: &str,
    source: Option<String>,
) -> Result<String> {
    let epmc_id = resolve_epmc_id(id, source.as_deref())?;
    let client = client.inner.clone();
    to_r_err(runtime().block_on(client.europe_pmc.fetch_full_text_xml(&epmc_id)))
}

/// Fetch the works a Europe PMC record cites, as a list of reference lists.
#[extendr]
fn epmc_references(
    client: ExternalPtr<ClientHandle>,
    id: &str,
    source: Option<String>,
) -> Result<Robj> {
    let epmc_id = resolve_epmc_id(id, source.as_deref())?;
    let client = client.inner.clone();
    let references = to_r_err(runtime().block_on(client.europe_pmc.get_references(&epmc_id)))?;
    let items: Vec<Robj> = references.iter().map(epmc_reference_to_robj).collect();
    Ok(List::from_values(items).into())
}

/// Fetch the articles citing a Europe PMC record, as a list of citation lists.
#[extendr]
fn epmc_citations(
    client: ExternalPtr<ClientHandle>,
    id: &str,
    source: Option<String>,
) -> Result<Robj> {
    let epmc_id = resolve_epmc_id(id, source.as_deref())?;
    let client = client.inner.clone();
    let citations = to_r_err(runtime().block_on(client.europe_pmc.get_citations(&epmc_id)))?;
    let items: Vec<Robj> = citations.iter().map(epmc_citation_to_robj).collect();
    Ok(List::from_values(items).into())
}

/// Fetch a Europe PMC record's external database cross-references.
#[extendr]
fn epmc_database_links(
    client: ExternalPtr<ClientHandle>,
    id: &str,
    source: Option<String>,
) -> Result<Robj> {
    let epmc_id = resolve_epmc_id(id, source.as_deref())?;
    let client = client.inner.clone();
    let links = to_r_err(runtime().block_on(client.europe_pmc.get_database_links(&epmc_id)))?;
    let items: Vec<Robj> = links.iter().map(epmc_database_link_to_robj).collect();
    Ok(List::from_values(items).into())
}

// ------------------------------------------------------------------------------------------------
// Module registration
// ------------------------------------------------------------------------------------------------

extendr_module! {
    mod pubmedclient;
    fn client_new;
    fn client_search_articles;
    fn client_fetch_article;
    fn client_fetch_articles;
    fn client_search_and_fetch;
    fn pmc_fetch_fulltext;
    fn pmc_markdown;
    fn epmc_search;
    fn epmc_fetch_fulltext;
    fn epmc_fetch_xml;
    fn epmc_references;
    fn epmc_citations;
    fn epmc_database_links;
}
