//! Python bindings for pubmed-client-rs using PyO3
//!
//! This module provides Python bindings for the Rust-based PubMed client library.

use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

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
    PyArticleSection, PyExtractedFigure, PyFigure, PyPmcAffiliation, PyPmcAuthor, PyPmcClient,
    PyPmcFullText, PyReference, PyTable,
};
pub use pubmed::{
    PyAffiliation, PyAuthor, PyCitations, PyDatabaseInfo, PyPmcLinks, PyPubMedArticle,
    PyPubMedClient, PyRelatedArticles,
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

    // Add PMC models
    m.add_class::<PyPmcAffiliation>()?;
    m.add_class::<PyPmcAuthor>()?;
    m.add_class::<PyFigure>()?;
    m.add_class::<PyExtractedFigure>()?;
    m.add_class::<PyTable>()?;
    m.add_class::<PyReference>()?;
    m.add_class::<PyArticleSection>()?;
    m.add_class::<PyPmcFullText>()?;

    // Add clients
    m.add_class::<PyPubMedClient>()?;
    m.add_class::<PyPmcClient>()?;
    m.add_class::<PyClient>()?;

    // Add query builder
    m.add_class::<PySearchQuery>()?;

    Ok(())
}

// ================================================================================================
// Stub Generation Support
// ================================================================================================

define_stub_info_gatherer!(stub_info);
