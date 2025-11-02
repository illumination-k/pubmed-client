//! PMC client for Python bindings
//!
//! This module provides Python wrappers for the PMC client.

use pyo3::prelude::*;
use std::sync::Arc;

use pubmed_client::PmcClient;

use crate::config::PyClientConfig;
use crate::utils::{get_runtime, to_py_err};

use super::models::PyPmcFullText;

// ================================================================================================
// Client Implementation
// ================================================================================================

/// PMC client for fetching full-text articles
///
/// Examples:
///     >>> client = PmcClient()
///     >>> full_text = client.fetch_full_text("PMC7906746")
///     >>> pmcid = client.check_pmc_availability("31978945")
#[pyclass(name = "PmcClient")]
pub struct PyPmcClient {
    pub client: Arc<PmcClient>,
}

#[pymethods]
impl PyPmcClient {
    /// Create a new PMC client with default configuration
    #[new]
    fn new() -> Self {
        PyPmcClient {
            client: Arc::new(PmcClient::new()),
        }
    }

    /// Create a new PMC client with custom configuration
    #[staticmethod]
    fn with_config(config: PyRef<PyClientConfig>) -> Self {
        PyPmcClient {
            client: Arc::new(PmcClient::with_config(config.inner.clone())),
        }
    }

    /// Fetch full text article from PMC
    ///
    /// Args:
    ///     pmcid: PMC ID (e.g., "PMC7906746")
    ///
    /// Returns:
    ///     PmcFullText object containing structured article content
    fn fetch_full_text(&self, py: Python, pmcid: String) -> PyResult<PyPmcFullText> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            let full_text = rt
                .block_on(client.fetch_full_text(&pmcid))
                .map_err(to_py_err)?;
            Ok(PyPmcFullText::from(full_text))
        })
    }

    /// Check if PMC full text is available for a PMID
    ///
    /// Args:
    ///     pmid: PubMed ID as a string
    ///
    /// Returns:
    ///     PMC ID if available, None otherwise
    fn check_pmc_availability(&self, py: Python, pmid: String) -> PyResult<Option<String>> {
        let client = self.client.clone();
        py.allow_threads(|| {
            let rt = get_runtime();
            rt.block_on(client.check_pmc_availability(&pmid))
                .map_err(to_py_err)
        })
    }

    fn __repr__(&self) -> String {
        "PmcClient()".to_string()
    }
}
