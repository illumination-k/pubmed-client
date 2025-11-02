//! PubMed client for Python bindings
//!
//! This module provides Python wrappers for the PubMed client.

use pyo3::prelude::*;
use std::sync::Arc;

use pubmed_client::PubMedClient as RustPubMedClient;

use crate::config::PyClientConfig;
use crate::utils::{get_runtime, to_py_err};

use super::models::{PyCitations, PyDatabaseInfo, PyPmcLinks, PyPubMedArticle, PyRelatedArticles};

// ================================================================================================
// Client Implementation
// ================================================================================================

/// PubMed client for searching and fetching article metadata
///
/// Examples:
///     >>> client = PubMedClient()
///     >>> articles = client.search_and_fetch("covid-19", 10)
///     >>> article = client.fetch_article("31978945")
#[pyclass(name = "PubMedClient")]
pub struct PyPubMedClient {
    pub client: Arc<RustPubMedClient>,
}

#[pymethods]
impl PyPubMedClient {
    /// Create a new PubMed client with default configuration
    #[new]
    fn new() -> Self {
        PyPubMedClient {
            client: Arc::new(RustPubMedClient::new()),
        }
    }

    /// Create a new PubMed client with custom configuration
    #[staticmethod]
    fn with_config(config: PyRef<PyClientConfig>) -> Self {
        PyPubMedClient {
            client: Arc::new(RustPubMedClient::with_config(config.inner.clone())),
        }
    }

    /// Search for articles and fetch their metadata
    ///
    /// Args:
    ///     query: Search query string
    ///     limit: Maximum number of articles to return
    ///
    /// Returns:
    ///     List of PubMedArticle objects
    fn search_and_fetch(
        &self,
        py: Python,
        query: String,
        limit: usize,
    ) -> PyResult<Vec<PyPubMedArticle>> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            let articles = rt
                .block_on(client.search_and_fetch(&query, limit))
                .map_err(to_py_err)?;
            Ok(articles.into_iter().map(PyPubMedArticle::from).collect())
        })
    }

    /// Fetch a single article by PMID
    ///
    /// Args:
    ///     pmid: PubMed ID as a string
    ///
    /// Returns:
    ///     PubMedArticle object
    fn fetch_article(&self, py: Python, pmid: String) -> PyResult<PyPubMedArticle> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            let article = rt
                .block_on(client.fetch_article(&pmid))
                .map_err(to_py_err)?;
            Ok(PyPubMedArticle::from(article))
        })
    }

    /// Get list of all available NCBI databases
    ///
    /// Returns:
    ///     List of database names
    fn get_database_list(&self, py: Python) -> PyResult<Vec<String>> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            rt.block_on(client.get_database_list()).map_err(to_py_err)
        })
    }

    /// Get detailed information about a specific database
    ///
    /// Args:
    ///     database: Database name (e.g., "pubmed", "pmc")
    ///
    /// Returns:
    ///     DatabaseInfo object
    fn get_database_info(&self, py: Python, database: String) -> PyResult<PyDatabaseInfo> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            let info = rt
                .block_on(client.get_database_info(&database))
                .map_err(to_py_err)?;
            Ok(PyDatabaseInfo::from(info))
        })
    }

    /// Get related articles for given PMIDs
    ///
    /// Args:
    ///     pmids: List of PubMed IDs
    ///
    /// Returns:
    ///     RelatedArticles object
    fn get_related_articles(&self, py: Python, pmids: Vec<u32>) -> PyResult<PyRelatedArticles> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            let related = rt
                .block_on(client.get_related_articles(&pmids))
                .map_err(to_py_err)?;
            Ok(PyRelatedArticles::from(related))
        })
    }

    /// Get PMC links for given PMIDs (full-text availability)
    ///
    /// Args:
    ///     pmids: List of PubMed IDs
    ///
    /// Returns:
    ///     PmcLinks object containing available PMC IDs
    fn get_pmc_links(&self, py: Python, pmids: Vec<u32>) -> PyResult<PyPmcLinks> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            let links = rt
                .block_on(client.get_pmc_links(&pmids))
                .map_err(to_py_err)?;
            Ok(PyPmcLinks::from(links))
        })
    }

    /// Get citing articles for given PMIDs
    ///
    /// Returns articles that cite the specified PMIDs from the PubMed database only.
    ///
    /// Important: Citation counts from this method may be LOWER than Google Scholar
    /// or scite.ai because this only includes peer-reviewed articles in PubMed.
    /// Other sources include preprints, books, and conference proceedings.
    ///
    /// Example: PMID 31978945 shows ~14,000 citations in PubMed vs ~23,000 in scite.ai.
    /// This is expected - this method provides PubMed-specific citation data.
    ///
    /// Args:
    ///     pmids: List of PubMed IDs
    ///
    /// Returns:
    ///     Citations object containing citing article PMIDs
    fn get_citations(&self, py: Python, pmids: Vec<u32>) -> PyResult<PyCitations> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            let citations = rt
                .block_on(client.get_citations(&pmids))
                .map_err(to_py_err)?;
            Ok(PyCitations::from(citations))
        })
    }

    fn __repr__(&self) -> String {
        "PubMedClient()".to_string()
    }
}
