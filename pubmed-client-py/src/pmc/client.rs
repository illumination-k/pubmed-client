//! PMC client for Python bindings
//!
//! This module provides Python wrappers for the PMC client.

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen_derive::gen_stub_pyclass;
use std::path::PathBuf;
use std::sync::Arc;

use pubmed_client::PmcClient;

use crate::config::PyClientConfig;
use crate::utils::{get_runtime, to_py_err};

use super::models::{PyExtractedFigure, PyOaSubsetInfo, PyPmcFullText};

// ================================================================================================
// Client Implementation
// ================================================================================================

/// PMC client for fetching full-text articles
///
/// Examples:
///     >>> client = PmcClient()
///     >>> full_text = client.fetch_full_text("PMC7906746")
///     >>> pmcid = client.check_pmc_availability("31978945")
#[gen_stub_pyclass]
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
        py.detach(|| {
            let rt = get_runtime();
            let full_text = rt
                .block_on(client.fetch_full_text(&pmcid))
                .map_err(to_py_err)?;
            Ok(PyPmcFullText::from(full_text))
        })
    }

    /// Check if a PubMed article has PMC full text available
    ///
    /// Args:
    ///     pmid: PubMed ID as a string
    ///
    /// Returns:
    ///     PMC ID if available, None otherwise
    fn check_pmc_availability(&self, py: Python, pmid: String) -> PyResult<Option<String>> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.check_pmc_availability(&pmid))
                .map_err(to_py_err)
        })
    }

    /// Download and extract PMC tar.gz archive
    ///
    /// Downloads the tar.gz file for the specified PMC ID and extracts all files
    /// to the output directory.
    ///
    /// Args:
    ///     pmcid: PMC ID (e.g., "PMC7906746" or "7906746")
    ///     output_dir: Directory path where files should be extracted
    ///
    /// Returns:
    ///     List of extracted file paths
    ///
    /// Note:
    ///     This method is only available on non-WASM platforms
    ///
    /// Example:
    ///     >>> client = PmcClient()
    ///     >>> files = client.download_and_extract_tar("PMC7906746", "./output")
    ///     >>> for file in files:
    ///     ...     print(file)
    fn download_and_extract_tar(
        &self,
        py: Python,
        pmcid: String,
        output_dir: String,
    ) -> PyResult<Py<PyList>> {
        let client = self.client.clone();
        let output_path = PathBuf::from(output_dir);

        py.detach(|| {
            let rt = get_runtime();
            let files = rt
                .block_on(client.download_and_extract_tar(&pmcid, &output_path))
                .map_err(to_py_err)?;

            Python::attach(|py| {
                let list = PyList::empty(py);
                for file in files {
                    list.append(file)?;
                }
                Ok(list.into())
            })
        })
    }

    /// Extract figures with captions from PMC article
    ///
    /// Downloads the tar.gz file for the specified PMC ID, extracts all files,
    /// and matches figures with their captions from the XML metadata.
    ///
    /// Args:
    ///     pmcid: PMC ID (e.g., "PMC7906746" or "7906746")
    ///     output_dir: Directory path where files should be extracted
    ///
    /// Returns:
    ///     List of ExtractedFigure objects containing metadata and file information
    ///
    /// Note:
    ///     This method is only available on non-WASM platforms
    ///
    /// Example:
    ///     >>> client = PmcClient()
    ///     >>> figures = client.extract_figures_with_captions("PMC7906746", "./output")
    ///     >>> for fig in figures:
    ///     ...     print(f"{fig.figure.id}: {fig.extracted_file_path}")
    ///     ...     print(f"  Caption: {fig.figure.caption}")
    ///     ...     print(f"  Size: {fig.file_size} bytes")
    ///     ...     print(f"  Dimensions: {fig.dimensions}")
    fn extract_figures_with_captions(
        &self,
        py: Python,
        pmcid: String,
        output_dir: String,
    ) -> PyResult<Py<PyList>> {
        let client = self.client.clone();
        let output_path = PathBuf::from(output_dir);

        py.detach(|| {
            let rt = get_runtime();
            let extracted_figures = rt
                .block_on(client.extract_figures_with_captions(&pmcid, &output_path))
                .map_err(to_py_err)?;

            Python::attach(|py| {
                let list = PyList::empty(py);
                for fig in &extracted_figures {
                    let py_fig = PyExtractedFigure::from(fig);
                    list.append(py_fig)?;
                }
                Ok(list.into())
            })
        })
    }

    /// Check if a PMC article is in the OA (Open Access) subset
    ///
    /// The OA subset contains articles with programmatic access to full-text XML.
    /// Some publishers restrict programmatic access even though the article may be
    /// viewable on the PMC website.
    ///
    /// Args:
    ///     pmcid: PMC ID (with or without "PMC" prefix, e.g., "PMC7906746" or "7906746")
    ///
    /// Returns:
    ///     OaSubsetInfo object containing detailed information about OA availability
    ///
    /// Example:
    ///     >>> client = PmcClient()
    ///     >>> oa_info = client.is_oa_subset("PMC7906746")
    ///     >>> if oa_info.is_oa_subset:
    ///     ...     print(f"Article is in OA subset")
    ///     ...     if oa_info.download_link:
    ///     ...         print(f"Download: {oa_info.download_link}")
    ///     ... else:
    ///     ...     print(f"Not in OA subset: {oa_info.error_message}")
    fn is_oa_subset(&self, py: Python, pmcid: String) -> PyResult<PyOaSubsetInfo> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let oa_info = rt
                .block_on(client.is_oa_subset(&pmcid))
                .map_err(to_py_err)?;
            Ok(PyOaSubsetInfo::from(oa_info))
        })
    }

    fn __repr__(&self) -> String {
        "PmcClient()".to_string()
    }
}
