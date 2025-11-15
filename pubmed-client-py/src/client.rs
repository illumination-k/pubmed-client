//! Combined client for Python bindings
//!
//! This module provides the main Python client that combines both PubMed and PMC functionality.

use pyo3::prelude::*;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::sync::Arc;

use pubmed_client::Client;

use crate::config::PyClientConfig;
use crate::pmc::{PyPmcClient, PyPmcFullText};
use crate::pubmed::{
    PyCitations, PyDatabaseInfo, PyPmcLinks, PyPubMedArticle, PyPubMedClient, PyRelatedArticles,
};
use crate::utils::{get_runtime, to_py_err};

// ================================================================================================
// Combined Client
// ================================================================================================

/// Combined client with both PubMed and PMC functionality
///
/// This is the main client you'll typically use. It provides access to both
/// PubMed metadata searches and PMC full-text retrieval.
///
/// Examples:
///     >>> client = Client()
///     >>> # Access PubMed client
///     >>> articles = client.pubmed.search_and_fetch("covid-19", 10)
///     >>> # Access PMC client
///     >>> full_text = client.pmc.fetch_full_text("PMC7906746")
///     >>> # Search with full text
///     >>> results = client.search_with_full_text("covid-19", 5)
#[gen_stub_pyclass]
#[pyclass(name = "Client")]
pub struct PyClient {
    pub client: Arc<Client>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClient {
    /// Create a new combined client with default configuration
    #[new]
    fn new() -> Self {
        PyClient {
            client: Arc::new(Client::new()),
        }
    }

    /// Create a new combined client with custom configuration
    #[staticmethod]
    fn with_config(config: PyRef<PyClientConfig>) -> Self {
        PyClient {
            client: Arc::new(Client::with_config(config.inner.clone())),
        }
    }

    /// Get PubMed client for metadata operations
    #[getter]
    fn pubmed(&self) -> PyPubMedClient {
        PyPubMedClient {
            client: Arc::new(self.client.pubmed.clone()),
        }
    }

    /// Get PMC client for full-text operations
    #[getter]
    fn pmc(&self) -> PyPmcClient {
        PyPmcClient {
            client: Arc::new(self.client.pmc.clone()),
        }
    }

    /// Search for articles and attempt to fetch full text for each
    ///
    /// This is a convenience method that searches PubMed and attempts to fetch
    /// PMC full text for each result when available.
    ///
    /// Args:
    ///     query: Search query string
    ///     limit: Maximum number of articles to process
    ///
    /// Returns:
    ///     List of tuples (PubMedArticle, Optional[PmcFullText])
    fn search_with_full_text(
        &self,
        py: Python,
        query: String,
        limit: usize,
    ) -> PyResult<Vec<(PyPubMedArticle, Option<PyPmcFullText>)>> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let results = rt
                .block_on(client.search_with_full_text(&query, limit))
                .map_err(to_py_err)?;

            Ok(results
                .into_iter()
                .map(|(article, full_text)| {
                    (
                        PyPubMedArticle::from(article),
                        full_text.map(PyPmcFullText::from),
                    )
                })
                .collect())
        })
    }

    /// Get list of all available NCBI databases
    fn get_database_list(&self, py: Python) -> PyResult<Vec<String>> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.get_database_list()).map_err(to_py_err)
        })
    }

    /// Get detailed information about a specific database
    fn get_database_info(&self, py: Python, database: String) -> PyResult<PyDatabaseInfo> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let info = rt
                .block_on(client.get_database_info(&database))
                .map_err(to_py_err)?;
            Ok(PyDatabaseInfo::from(info))
        })
    }

    /// Get related articles for given PMIDs
    fn get_related_articles(&self, py: Python, pmids: Vec<u32>) -> PyResult<PyRelatedArticles> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let related = rt
                .block_on(client.get_related_articles(&pmids))
                .map_err(to_py_err)?;
            Ok(PyRelatedArticles::from(related))
        })
    }

    /// Get PMC links for given PMIDs
    fn get_pmc_links(&self, py: Python, pmids: Vec<u32>) -> PyResult<PyPmcLinks> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let links = rt
                .block_on(client.get_pmc_links(&pmids))
                .map_err(to_py_err)?;
            Ok(PyPmcLinks::from(links))
        })
    }

    /// Get citing articles for given PMIDs
    fn get_citations(&self, py: Python, pmids: Vec<u32>) -> PyResult<PyCitations> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let citations = rt
                .block_on(client.get_citations(&pmids))
                .map_err(to_py_err)?;
            Ok(PyCitations::from(citations))
        })
    }

    fn __repr__(&self) -> String {
        "Client()".to_string()
    }
}
