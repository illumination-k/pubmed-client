//! R bindings for the PubMed / PMC client library, built with extendr.
//!
//! The public surface is intentionally small (an MVP): create a client, search
//! PubMed, fetch article metadata, and retrieve PMC full text / Markdown. The
//! client handle is exposed to R as an [`ExternalPtr`] and threaded through the
//! free functions below; the ergonomic R API lives in `R/pubmed-client.R`.
//!
//! Like the Python bindings, all calls are synchronous from the caller's point
//! of view: a process-wide Tokio runtime drives the async `pubmed-client` API
//! and blocks until completion.

use std::sync::Arc;
use std::sync::OnceLock;

use extendr_api::prelude::*;
use tokio::runtime::Runtime;

use pubmed_client::{Client, ClientConfig, PmcArticle, PmcMarkdownConverter, PubMedArticle};

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
}
