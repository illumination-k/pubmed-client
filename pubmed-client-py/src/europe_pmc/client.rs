//! Europe PMC client for Python bindings
//!
//! Python wrapper around [`pubmed_client::EuropePmcClient`]: cross-source
//! search, JATS full text, reference and citation graphs, external database
//! links, and supplementary file download. No API key is required.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::path::PathBuf;
use std::sync::Arc;

use pubmed_client::{EuropePmcClient, EuropePmcSearchOptions, ResultType};

use crate::config::PyClientConfig;
use crate::pmc::PyPmcFullText;
use crate::utils::{get_runtime, to_py_err};

use super::models::{
    PyEuropePmcCitation, PyEuropePmcDatabaseLink, PyEuropePmcReference, PyEuropePmcResult,
    PyEuropePmcSearchResponse,
};
use super::resolve_id;

/// Map the Python-facing `result_type` string onto the Rust enum.
fn parse_result_type(result_type: &str) -> PyResult<ResultType> {
    match result_type.trim().to_ascii_lowercase().as_str() {
        "idlist" | "id_list" => Ok(ResultType::IdList),
        "lite" => Ok(ResultType::Lite),
        "core" => Ok(ResultType::Core),
        other => Err(PyValueError::new_err(format!(
            "invalid result_type {other:?}: expected 'idlist', 'lite' or 'core'"
        ))),
    }
}

/// Build search options from the keyword arguments shared by the search methods.
///
/// `result_type` and `cursor_mark` are `Option` rather than defaulted strings
/// because PyO3 renders a non-literal default as `...` in the generated
/// signature, which `stubtest` then flags as disagreeing with the stub.
fn search_options(
    result_type: Option<&str>,
    page_size: u32,
    cursor_mark: Option<&str>,
    sort: Option<String>,
) -> PyResult<EuropePmcSearchOptions> {
    Ok(EuropePmcSearchOptions {
        result_type: parse_result_type(result_type.unwrap_or("lite"))?,
        // Europe PMC rejects a page size outside this range outright.
        page_size: page_size.clamp(1, 1000),
        cursor_mark: cursor_mark.unwrap_or("*").to_string(),
        sort,
    })
}

// ================================================================================================
// Client Implementation
// ================================================================================================

/// Europe PMC client
///
/// Europe PMC (<https://europepmc.org>) is a complementary index to the NCBI
/// E-utilities: it covers preprints (PPR), patents (PAT), Agricola (AGR) and
/// Chinese Biological Abstracts (CBA) as well as PubMed (MED) and PMC, and it
/// requires no API key.
///
/// Records are addressed by a source database plus an id. Every method here
/// accepts the id bare ("PMC3258128", "33515491"), with an explicit `source`,
/// or fully qualified ("PPR/PPR123456"). With no source, a PMC-prefixed id is
/// read as a PMC record and anything else as a PubMed record.
///
/// Examples:
///     >>> client = EuropePmcClient()
///     >>> for result in client.search("malaria vaccine", 5):
///     ...     print(result.europe_pmc_id, result.title)
///     >>> article = client.fetch_full_text("PMC3258128")
///     >>> citations = client.get_citations("33515491")
#[gen_stub_pyclass]
#[pyclass(name = "EuropePmcClient")]
pub struct PyEuropePmcClient {
    pub client: Arc<EuropePmcClient>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEuropePmcClient {
    /// Create a new Europe PMC client with default configuration
    #[new]
    fn new() -> Self {
        PyEuropePmcClient {
            client: Arc::new(EuropePmcClient::new()),
        }
    }

    /// Create a new Europe PMC client with custom configuration
    ///
    /// Transport settings (timeout, user agent, retry, rate limit, cache) are
    /// taken from the config; its `base_url` is the NCBI override and is not
    /// used here.
    #[staticmethod]
    fn with_config(config: PyRef<PyClientConfig>) -> Self {
        PyEuropePmcClient {
            client: Arc::new(EuropePmcClient::with_config(config.inner.clone())),
        }
    }

    /// Search Europe PMC and return up to `limit` results
    ///
    /// Args:
    ///     query: Europe PMC query, e.g. "malaria vaccine" or "TITLE:CRISPR AND SRC:PPR"
    ///     limit: Maximum number of records to return (default: 10)
    ///
    /// Returns:
    ///     List of EuropePmcResult
    ///
    /// Examples:
    ///     >>> client = EuropePmcClient()
    ///     >>> results = client.search("malaria vaccine", 5)
    #[pyo3(signature = (query, limit = 10))]
    fn search(&self, py: Python, query: String, limit: usize) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let results = py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.search(&query, limit)).map_err(to_py_err)
        })?;
        let list = PyList::empty(py);
        for result in &results {
            list.append(PyEuropePmcResult::from(result))?;
        }
        Ok(list.into())
    }

    /// Fetch a single page of search results
    ///
    /// Use the returned `next_cursor_mark` as the `cursor_mark` of the
    /// following call to page through a result set. Europe PMC signals the end
    /// by returning the cursor it was given.
    ///
    /// Args:
    ///     query: Europe PMC query
    ///     result_type: "idlist", "lite" (default) or "core"
    ///     page_size: Records per page, 1-1000 (default: 25)
    ///     cursor_mark: Cursor for the page to fetch; "*" (default) is the first page
    ///     sort: Europe PMC sort expression, e.g. "P_PDATE_D desc" or "CITED desc"
    ///
    /// Returns:
    ///     EuropePmcSearchResponse
    #[pyo3(signature = (query, result_type = None, page_size = 25, cursor_mark = None, sort = None))]
    fn search_page(
        &self,
        py: Python,
        query: String,
        result_type: Option<String>,
        page_size: u32,
        cursor_mark: Option<String>,
        sort: Option<String>,
    ) -> PyResult<PyEuropePmcSearchResponse> {
        let opts = search_options(
            result_type.as_deref(),
            page_size,
            cursor_mark.as_deref(),
            sort,
        )?;
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let response = rt
                .block_on(client.search_page(&query, &opts))
                .map_err(to_py_err)?;
            Ok(PyEuropePmcSearchResponse::from(response))
        })
    }

    /// Fetch search results across pages until `max_results` or exhaustion
    ///
    /// Args:
    ///     query: Europe PMC query
    ///     max_results: Maximum number of records to collect
    ///     result_type: "idlist", "lite" (default) or "core"
    ///     page_size: Records per request, 1-1000 (default: 25)
    ///     sort: Europe PMC sort expression
    ///
    /// Returns:
    ///     List of EuropePmcResult
    #[pyo3(signature = (query, max_results, result_type = None, page_size = 25, sort = None))]
    fn search_all(
        &self,
        py: Python,
        query: String,
        max_results: usize,
        result_type: Option<String>,
        page_size: u32,
        sort: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let opts = search_options(result_type.as_deref(), page_size, None, sort)?;
        let client = self.client.clone();
        let results = py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.search_all(&query, max_results, &opts))
                .map_err(to_py_err)
        })?;
        let list = PyList::empty(py);
        for result in &results {
            list.append(PyEuropePmcResult::from(result))?;
        }
        Ok(list.into())
    }

    /// Fetch and parse the full text of a Europe PMC record
    ///
    /// Parsing into an article requires a PMC id, so this only supports
    /// PMC-sourced records; use `fetch_full_text_xml` for other sources.
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     PmcFullText
    #[pyo3(signature = (id, source = None))]
    fn fetch_full_text(
        &self,
        py: Python,
        id: String,
        source: Option<String>,
    ) -> PyResult<PyPmcFullText> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let article = rt
                .block_on(client.fetch_full_text(&epmc_id))
                .map_err(to_py_err)?;
            Ok(PyPmcFullText::from(article))
        })
    }

    /// Fetch the raw JATS XML full text of a Europe PMC record
    ///
    /// Works for any source that has full text available.
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     JATS XML as a string
    #[pyo3(signature = (id, source = None))]
    fn fetch_full_text_xml(
        &self,
        py: Python,
        id: String,
        source: Option<String>,
    ) -> PyResult<String> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.fetch_full_text_xml(&epmc_id))
                .map_err(to_py_err)
        })
    }

    /// Fetch all works cited by a record, following pages until exhausted
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     List of EuropePmcReference
    #[pyo3(signature = (id, source = None))]
    fn get_references(
        &self,
        py: Python,
        id: String,
        source: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        let references = py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.get_references(&epmc_id))
                .map_err(to_py_err)
        })?;
        let list = PyList::empty(py);
        for reference in &references {
            list.append(PyEuropePmcReference::from(reference))?;
        }
        Ok(list.into())
    }

    /// Fetch one page of the reference list for a record
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     page: 1-based page number
    ///     page_size: Entries per page (default: 100)
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     Tuple of (total hit count, list of EuropePmcReference)
    #[pyo3(signature = (id, page = 1, page_size = 100, source = None))]
    fn get_references_page(
        &self,
        py: Python,
        id: String,
        page: u32,
        page_size: u32,
        source: Option<String>,
    ) -> PyResult<(u64, Py<PyAny>)> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        let list = py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.get_references_page(&epmc_id, page, page_size))
                .map_err(to_py_err)
        })?;
        let items = PyList::empty(py);
        for reference in &list.references {
            items.append(PyEuropePmcReference::from(reference))?;
        }
        Ok((list.hit_count, items.into()))
    }

    /// Fetch all articles citing a record, following pages until exhausted
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     List of EuropePmcCitation
    #[pyo3(signature = (id, source = None))]
    fn get_citations(&self, py: Python, id: String, source: Option<String>) -> PyResult<Py<PyAny>> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        let citations = py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.get_citations(&epmc_id))
                .map_err(to_py_err)
        })?;
        let list = PyList::empty(py);
        for citation in &citations {
            list.append(PyEuropePmcCitation::from(citation))?;
        }
        Ok(list.into())
    }

    /// Fetch one page of the citation list for a record
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     page: 1-based page number
    ///     page_size: Entries per page (default: 100)
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     Tuple of (total hit count, list of EuropePmcCitation)
    #[pyo3(signature = (id, page = 1, page_size = 100, source = None))]
    fn get_citations_page(
        &self,
        py: Python,
        id: String,
        page: u32,
        page_size: u32,
        source: Option<String>,
    ) -> PyResult<(u64, Py<PyAny>)> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        let list = py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.get_citations_page(&epmc_id, page, page_size))
                .map_err(to_py_err)
        })?;
        let items = PyList::empty(py);
        for citation in &list.citations {
            items.append(PyEuropePmcCitation::from(citation))?;
        }
        Ok((list.hit_count, items.into()))
    }

    /// Fetch all external database cross-references for a record
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     List of EuropePmcDatabaseLink
    ///
    /// Examples:
    ///     >>> client = EuropePmcClient()
    ///     >>> for link in client.get_database_links("PMC3258128"):
    ///     ...     print(link.db_name, link.db_count)
    #[pyo3(signature = (id, source = None))]
    fn get_database_links(
        &self,
        py: Python,
        id: String,
        source: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        let links = py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.get_database_links(&epmc_id))
                .map_err(to_py_err)
        })?;
        let list = PyList::empty(py);
        for link in &links {
            list.append(PyEuropePmcDatabaseLink::from(link))?;
        }
        Ok(list.into())
    }

    /// Fetch the supplementary-files ZIP archive for a record into memory
    ///
    /// Europe PMC returns supplementary materials as a single ZIP; unpacking is
    /// left to the caller (e.g. Python's `zipfile`).
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     ZIP archive as bytes
    #[pyo3(signature = (id, source = None))]
    fn fetch_supplementary_files(
        &self,
        py: Python,
        id: String,
        source: Option<String>,
    ) -> PyResult<Vec<u8>> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.fetch_supplementary_files(&epmc_id))
                .map_err(to_py_err)
        })
    }

    /// Download the supplementary-files ZIP archive for a record to a path
    ///
    /// Parent directories are created if needed.
    ///
    /// Args:
    ///     id: Record id, bare or fully qualified
    ///     output_path: Full path of the ZIP file to write
    ///     source: Source database (MED, PMC, PPR, AGR, CBA, PAT)
    ///
    /// Returns:
    ///     The written path
    #[pyo3(signature = (id, output_path, source = None))]
    fn download_supplementary_files(
        &self,
        py: Python,
        id: String,
        output_path: String,
        source: Option<String>,
    ) -> PyResult<String> {
        let epmc_id = resolve_id(&id, source.as_deref())?;
        let output_path = PathBuf::from(output_path);
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let written = rt
                .block_on(client.download_supplementary_files(&epmc_id, &output_path))
                .map_err(to_py_err)?;
            Ok(written.to_string_lossy().into_owned())
        })
    }

    /// Clear the full-text cache, if one is configured
    fn clear_cache(&self, py: Python) {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.clear_cache());
        })
    }

    /// Number of cached full-text entries (best effort)
    fn cache_entry_count(&self) -> u64 {
        self.client.cache_entry_count()
    }

    fn __repr__(&self) -> String {
        "EuropePmcClient()".to_string()
    }
}
