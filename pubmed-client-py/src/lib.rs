//! Python bindings for pubmed-client using PyO3
//!
//! This module provides Python bindings for the Rust-based PubMed client library.

use pyo3::prelude::*;

// Module declarations
mod client;
mod config;
mod pmc;
mod pubmed;
mod query;
mod utils;

// Re-export main types for convenience
pub use client::PyClient;
pub use config::PyClientConfig;
pub use pmc::{
    PyArticleSection, PyExtractedFigure, PyFigure, PyOaSubsetInfo, PyPmcAffiliation, PyPmcAuthor,
    PyPmcClient, PyPmcFullText, PyReference, PyTable,
};
pub use pubmed::{
    PyAffiliation, PyArticleSummary, PyAuthor, PyCitationMatch, PyCitationMatches, PyCitationQuery,
    PyCitations, PyDatabaseCount, PyDatabaseInfo, PyEPostResult, PyGlobalQueryResults, PyPmcLinks,
    PyPubMedArticle, PyPubMedClient, PyRelatedArticles, PySpellCheckResult,
};
pub use query::PySearchQuery;

// ================================================================================================
// Module Definition
// ================================================================================================

/// Python bindings for PubMed and PMC API client
///
/// This module provides a high-performance Python interface to PubMed and PMC APIs
/// for retrieving biomedical research articles.
///
/// Main classes:
///     Client: Combined client for both PubMed and PMC
///     PubMedClient: Client for PubMed metadata
///     PmcClient: Client for PMC full-text articles
///     ClientConfig: Configuration for API clients
///
/// Examples:
///     >>> import pubmed_client
///     >>> client = pubmed_client.Client()
///     >>> articles = client.pubmed.search_and_fetch("covid-19", 10)
///     >>> for article in articles:
///     ...     print(article.title)
#[pymodule]
fn pubmed_client(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Add configuration
    m.add_class::<PyClientConfig>()?;

    // Add PubMed models
    m.add_class::<PyAffiliation>()?;
    m.add_class::<PyAuthor>()?;
    m.add_class::<PyPubMedArticle>()?;
    m.add_class::<PyRelatedArticles>()?;
    m.add_class::<PyPmcLinks>()?;
    m.add_class::<PyCitations>()?;
    m.add_class::<PyDatabaseInfo>()?;
    m.add_class::<PyCitationQuery>()?;
    m.add_class::<PyCitationMatch>()?;
    m.add_class::<PyCitationMatches>()?;
    m.add_class::<PyDatabaseCount>()?;
    m.add_class::<PyGlobalQueryResults>()?;
    m.add_class::<PyEPostResult>()?;
    m.add_class::<PySpellCheckResult>()?;
    m.add_class::<PyArticleSummary>()?;

    // Add PMC models
    m.add_class::<PyPmcAffiliation>()?;
    m.add_class::<PyPmcAuthor>()?;
    m.add_class::<PyFigure>()?;
    m.add_class::<PyExtractedFigure>()?;
    m.add_class::<PyTable>()?;
    m.add_class::<PyReference>()?;
    m.add_class::<PyArticleSection>()?;
    m.add_class::<PyPmcFullText>()?;
    m.add_class::<PyOaSubsetInfo>()?;

    // Add clients
    m.add_class::<PyPubMedClient>()?;
    m.add_class::<PyPmcClient>()?;
    m.add_class::<PyClient>()?;

    // Add query builder
    m.add_class::<PySearchQuery>()?;

    // Add exception hierarchy
    m.add(
        "PubMedException",
        m.py().get_type::<utils::PubMedException>(),
    )?;
    m.add("ParseException", m.py().get_type::<utils::ParseException>())?;
    m.add(
        "RequestException",
        m.py().get_type::<utils::RequestException>(),
    )?;
    m.add(
        "InvalidQueryException",
        m.py().get_type::<utils::InvalidQueryException>(),
    )?;
    m.add(
        "RateLimitException",
        m.py().get_type::<utils::RateLimitException>(),
    )?;
    m.add("ApiException", m.py().get_type::<utils::ApiException>())?;
    m.add(
        "SearchLimitException",
        m.py().get_type::<utils::SearchLimitException>(),
    )?;
    m.add(
        "HistorySessionException",
        m.py().get_type::<utils::HistorySessionException>(),
    )?;

    Ok(())
}

// ================================================================================================
// Stub Generation Support
// ================================================================================================

/// Gather stub information for the compiled `pubmed_client` module.
///
/// This mirrors what [`pyo3_stub_gen::define_stub_info_gatherer`] would produce, but pins the
/// default module name to the single-segment `pubmed_client` instead of reading the dotted
/// `module-name = "pubmed_client_py.pubmed_client"` from `pyproject.toml`. maturin builds the
/// extension as a submodule inside an auto-generated `pubmed_client` package, but from a typing
/// perspective the public surface is the flat top-level `pubmed_client` module — so the generated
/// `pubmed_client.pyi` must describe that flat module, not a `pubmed_client_py.pubmed_client`
/// submodule (which pyo3-stub-gen rejects under the pure-Rust layout).
pub fn stub_info() -> pyo3_stub_gen::Result<pyo3_stub_gen::StubInfo> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    pyo3_stub_gen::StubInfo::from_project_root(
        "pubmed_client".to_string(),
        manifest_dir.to_path_buf(),
        false,
        pyo3_stub_gen::StubGenConfig::default(),
    )
}
