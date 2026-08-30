//! Europe PMC data models for Python bindings
//!
//! Python wrappers for the JSON response models returned by the Europe PMC
//! REST API. Europe PMC is deliberately lenient about its own schema — fields
//! come and go, and `resultType=core` returns far more than is modelled — so
//! every record also exposes the unmodelled remainder through `extra()`.

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::sync::Arc;

use pubmed_client::{
    EuropePmcCitation, EuropePmcDatabaseLink, EuropePmcDbCrossReferenceInfo, EuropePmcReference,
    EuropePmcResult, EuropePmcSearchResponse,
};

use crate::utils::json_map_to_py;

// ================================================================================================
// Search
// ================================================================================================

/// Python wrapper for a Europe PMC search result record
#[gen_stub_pyclass]
#[pyclass(name = "EuropePmcResult", from_py_object)]
#[derive(Clone)]
pub struct PyEuropePmcResult {
    /// Record identifier within its source database
    #[pyo3(get)]
    pub id: String,
    /// Source database code (MED, PMC, PPR, AGR, CBA, PAT, ...)
    #[pyo3(get)]
    pub source: String,
    #[pyo3(get)]
    pub pmid: Option<String>,
    #[pyo3(get)]
    pub pmcid: Option<String>,
    #[pyo3(get)]
    pub doi: Option<String>,
    #[pyo3(get)]
    pub title: Option<String>,
    #[pyo3(get)]
    pub author_string: Option<String>,
    #[pyo3(get)]
    pub journal_title: Option<String>,
    #[pyo3(get)]
    pub pub_year: Option<String>,
    /// Open access flag as reported by Europe PMC ("Y" / "N")
    #[pyo3(get)]
    pub is_open_access: Option<String>,
    inner: Arc<EuropePmcResult>,
}

impl From<&EuropePmcResult> for PyEuropePmcResult {
    fn from(result: &EuropePmcResult) -> Self {
        PyEuropePmcResult {
            id: result.id.clone(),
            source: result.source.clone(),
            pmid: result.pmid.clone(),
            pmcid: result.pmcid.clone(),
            doi: result.doi.clone(),
            title: result.title.clone(),
            author_string: result.author_string.clone(),
            journal_title: result.journal_title.clone(),
            pub_year: result.pub_year.clone(),
            is_open_access: result.is_open_access.clone(),
            inner: Arc::new(result.clone()),
        }
    }
}

impl From<EuropePmcResult> for PyEuropePmcResult {
    fn from(result: EuropePmcResult) -> Self {
        Self::from(&result)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEuropePmcResult {
    /// Fully-qualified Europe PMC address of this record ("SOURCE/ID")
    #[getter]
    fn europe_pmc_id(&self) -> String {
        format!("{}/{}", self.source, self.id)
    }

    /// Fields returned by Europe PMC but not modelled as attributes
    ///
    /// `resultType="core"` returns dozens of extra fields (abstract text,
    /// citation counts, MeSH terms, grant data, ...); they are surfaced here as
    /// a plain dict rather than pinned to a schema that Europe PMC may change.
    fn extra(&self, py: Python) -> PyResult<Py<PyAny>> {
        json_map_to_py(py, &self.inner.extra)
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropePmcResult(source='{}', id='{}', title={:?})",
            self.source, self.id, self.title
        )
    }
}

/// Python wrapper for one page of Europe PMC search results
#[gen_stub_pyclass]
#[pyclass(name = "EuropePmcSearchResponse", from_py_object)]
#[derive(Clone)]
pub struct PyEuropePmcSearchResponse {
    /// Total number of records matching the query, across all pages
    #[pyo3(get)]
    pub hit_count: u64,
    /// Cursor to pass as `cursor_mark` to fetch the next page
    ///
    /// Europe PMC keeps returning the same value once the last page is
    /// reached, so a cursor equal to the one just used means "no more pages".
    #[pyo3(get)]
    pub next_cursor_mark: Option<String>,
    results: Vec<PyEuropePmcResult>,
}

impl From<EuropePmcSearchResponse> for PyEuropePmcSearchResponse {
    fn from(response: EuropePmcSearchResponse) -> Self {
        PyEuropePmcSearchResponse {
            hit_count: response.hit_count,
            next_cursor_mark: response.next_cursor_mark,
            results: response.results.iter().map(Into::into).collect(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEuropePmcSearchResponse {
    /// Records on this page
    fn results(&self, py: Python) -> PyResult<Py<PyAny>> {
        Ok(PyList::new(py, self.results.clone())?.into())
    }

    fn __len__(&self) -> usize {
        self.results.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropePmcSearchResponse(hit_count={}, results={})",
            self.hit_count,
            self.results.len()
        )
    }
}

// ================================================================================================
// Citation graph
// ================================================================================================

/// Python wrapper for a work cited by a Europe PMC record
#[gen_stub_pyclass]
#[pyclass(name = "EuropePmcReference", from_py_object)]
#[derive(Clone)]
pub struct PyEuropePmcReference {
    /// Source database of the cited record, when Europe PMC matched it
    #[pyo3(get)]
    pub source: Option<String>,
    /// Identifier of the cited record, when Europe PMC matched it
    #[pyo3(get)]
    pub id: Option<String>,
    #[pyo3(get)]
    pub citation_type: Option<String>,
    #[pyo3(get)]
    pub title: Option<String>,
    #[pyo3(get)]
    pub author_string: Option<String>,
    #[pyo3(get)]
    pub journal_abbreviation: Option<String>,
    #[pyo3(get)]
    pub pub_year: Option<String>,
    #[pyo3(get)]
    pub volume: Option<String>,
    #[pyo3(get)]
    pub issue: Option<String>,
    #[pyo3(get)]
    pub page_info: Option<String>,
    #[pyo3(get)]
    pub pmid: Option<String>,
    #[pyo3(get)]
    pub doi: Option<String>,
    inner: Arc<EuropePmcReference>,
}

impl From<&EuropePmcReference> for PyEuropePmcReference {
    fn from(reference: &EuropePmcReference) -> Self {
        PyEuropePmcReference {
            source: reference.source.clone(),
            id: reference.id.clone(),
            citation_type: reference.citation_type.clone(),
            title: reference.title.clone(),
            author_string: reference.author_string.clone(),
            journal_abbreviation: reference.journal_abbreviation.clone(),
            pub_year: reference.pub_year.clone(),
            volume: reference.volume.clone(),
            issue: reference.issue.clone(),
            page_info: reference.page_info.clone(),
            pmid: reference.pmid.clone(),
            doi: reference.doi.clone(),
            inner: Arc::new(reference.clone()),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEuropePmcReference {
    /// Fields returned by Europe PMC but not modelled as attributes
    fn extra(&self, py: Python) -> PyResult<Py<PyAny>> {
        json_map_to_py(py, &self.inner.extra)
    }

    fn __repr__(&self) -> String {
        format!("EuropePmcReference(title={:?})", self.title)
    }
}

/// Python wrapper for an article citing a Europe PMC record
#[gen_stub_pyclass]
#[pyclass(name = "EuropePmcCitation", from_py_object)]
#[derive(Clone)]
pub struct PyEuropePmcCitation {
    /// Identifier of the citing record within its source database
    #[pyo3(get)]
    pub id: Option<String>,
    /// Source database of the citing record
    #[pyo3(get)]
    pub source: Option<String>,
    #[pyo3(get)]
    pub citation_type: Option<String>,
    #[pyo3(get)]
    pub title: Option<String>,
    #[pyo3(get)]
    pub author_string: Option<String>,
    #[pyo3(get)]
    pub journal_abbreviation: Option<String>,
    #[pyo3(get)]
    pub pub_year: Option<String>,
    #[pyo3(get)]
    pub volume: Option<String>,
    #[pyo3(get)]
    pub issue: Option<String>,
    #[pyo3(get)]
    pub page_info: Option<String>,
    /// Number of times the citing article has itself been cited
    #[pyo3(get)]
    pub cited_by_count: Option<String>,
    inner: Arc<EuropePmcCitation>,
}

impl From<&EuropePmcCitation> for PyEuropePmcCitation {
    fn from(citation: &EuropePmcCitation) -> Self {
        PyEuropePmcCitation {
            id: citation.id.clone(),
            source: citation.source.clone(),
            citation_type: citation.citation_type.clone(),
            title: citation.title.clone(),
            author_string: citation.author_string.clone(),
            journal_abbreviation: citation.journal_abbreviation.clone(),
            pub_year: citation.pub_year.clone(),
            volume: citation.volume.clone(),
            issue: citation.issue.clone(),
            page_info: citation.page_info.clone(),
            cited_by_count: citation.cited_by_count.clone(),
            inner: Arc::new(citation.clone()),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEuropePmcCitation {
    /// Fields returned by Europe PMC but not modelled as attributes
    fn extra(&self, py: Python) -> PyResult<Py<PyAny>> {
        json_map_to_py(py, &self.inner.extra)
    }

    fn __repr__(&self) -> String {
        format!("EuropePmcCitation(title={:?})", self.title)
    }
}

// ================================================================================================
// Database links
// ================================================================================================

/// Python wrapper for a single external-database cross-reference entry
///
/// Europe PMC documents the four `info` slots only positionally, and their
/// meaning varies by database, so they are exposed as-is rather than renamed.
#[gen_stub_pyclass]
#[pyclass(name = "EuropePmcDbCrossReferenceInfo", from_py_object)]
#[derive(Clone)]
pub struct PyEuropePmcDbCrossReferenceInfo {
    #[pyo3(get)]
    pub info1: Option<String>,
    #[pyo3(get)]
    pub info2: Option<String>,
    #[pyo3(get)]
    pub info3: Option<String>,
    #[pyo3(get)]
    pub info4: Option<String>,
}

impl From<&EuropePmcDbCrossReferenceInfo> for PyEuropePmcDbCrossReferenceInfo {
    fn from(info: &EuropePmcDbCrossReferenceInfo) -> Self {
        PyEuropePmcDbCrossReferenceInfo {
            info1: info.info1.clone(),
            info2: info.info2.clone(),
            info3: info.info3.clone(),
            info4: info.info4.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEuropePmcDbCrossReferenceInfo {
    fn __repr__(&self) -> String {
        format!("EuropePmcDbCrossReferenceInfo(info1={:?})", self.info1)
    }
}

/// Python wrapper for cross-references to one external database
#[gen_stub_pyclass]
#[pyclass(name = "EuropePmcDatabaseLink", from_py_object)]
#[derive(Clone)]
pub struct PyEuropePmcDatabaseLink {
    /// External database name (e.g. "UNIPROT", "EMBL", "PDB")
    #[pyo3(get)]
    pub db_name: Option<String>,
    /// Number of cross-references reported for this database
    #[pyo3(get)]
    pub db_count: Option<u32>,
    info: Vec<PyEuropePmcDbCrossReferenceInfo>,
}

impl From<&EuropePmcDatabaseLink> for PyEuropePmcDatabaseLink {
    fn from(link: &EuropePmcDatabaseLink) -> Self {
        PyEuropePmcDatabaseLink {
            db_name: link.db_name.clone(),
            db_count: link.db_count,
            info: link.info.iter().map(Into::into).collect(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyEuropePmcDatabaseLink {
    /// Individual cross-reference entries
    fn info(&self, py: Python) -> PyResult<Py<PyAny>> {
        Ok(PyList::new(py, self.info.clone())?.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "EuropePmcDatabaseLink(db_name={:?}, entries={})",
            self.db_name,
            self.info.len()
        )
    }
}
