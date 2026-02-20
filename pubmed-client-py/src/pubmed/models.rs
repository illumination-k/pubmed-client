//! PubMed data models for Python bindings
//!
//! This module provides Python wrappers for PubMed data structures.

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::sync::Arc;

use pubmed_client::{pubmed, PubMedArticle};

// ================================================================================================
// PubMed Data Models
// ================================================================================================

/// Python wrapper for Author affiliation
#[gen_stub_pyclass]
#[pyclass(name = "Affiliation")]
#[derive(Clone)]
pub struct PyAffiliation {
    #[pyo3(get)]
    pub institution: Option<String>,
    #[pyo3(get)]
    pub department: Option<String>,
    #[pyo3(get)]
    pub address: Option<String>,
    #[pyo3(get)]
    pub country: Option<String>,
    #[pyo3(get)]
    pub email: Option<String>,
}

impl From<&pubmed::Affiliation> for PyAffiliation {
    fn from(affiliation: &pubmed::Affiliation) -> Self {
        PyAffiliation {
            institution: affiliation.institution.clone(),
            department: affiliation.department.clone(),
            address: affiliation.address.clone(),
            country: affiliation.country.clone(),
            email: None, // Email is now on Author, not Affiliation
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAffiliation {
    fn __repr__(&self) -> String {
        format!(
            "Affiliation(institution={:?}, country={:?})",
            self.institution, self.country
        )
    }
}

/// Python wrapper for Author
#[gen_stub_pyclass]
#[pyclass(name = "Author")]
#[derive(Clone)]
pub struct PyAuthor {
    #[pyo3(get)]
    pub surname: Option<String>,
    #[pyo3(get)]
    pub given_names: Option<String>,
    #[pyo3(get)]
    pub initials: Option<String>,
    #[pyo3(get)]
    pub suffix: Option<String>,
    #[pyo3(get)]
    pub full_name: String,
    #[pyo3(get)]
    pub orcid: Option<String>,
    #[pyo3(get)]
    pub email: Option<String>,
    #[pyo3(get)]
    pub is_corresponding: bool,
    inner: Arc<pubmed::Author>,
}

impl From<&pubmed::Author> for PyAuthor {
    fn from(author: &pubmed::Author) -> Self {
        PyAuthor {
            surname: author.surname.clone(),
            given_names: author.given_names.clone(),
            initials: author.initials.clone(),
            suffix: author.suffix.clone(),
            full_name: author.full_name.clone(),
            orcid: author.orcid.clone(),
            email: author.email.clone(),
            is_corresponding: author.is_corresponding,
            inner: Arc::new(author.clone()),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyAuthor {
    /// Get list of affiliations
    fn affiliations(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for affiliation in &self.inner.affiliations {
            let py_affiliation = PyAffiliation::from(affiliation);
            list.append(py_affiliation)?;
        }
        Ok(list.into())
    }

    /// Get list of roles/contributions
    fn roles(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::new(py, &self.inner.roles)?;
        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!("Author(full_name='{}')", self.full_name)
    }
}

/// Python wrapper for PubMedArticle
#[gen_stub_pyclass]
#[pyclass(name = "PubMedArticle")]
#[derive(Clone)]
pub struct PyPubMedArticle {
    #[pyo3(get)]
    pub pmid: String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub journal: String,
    #[pyo3(get)]
    pub pub_date: String,
    #[pyo3(get)]
    pub doi: Option<String>,
    #[pyo3(get)]
    pub pmc_id: Option<String>,
    #[pyo3(get)]
    pub abstract_text: Option<String>,
    #[pyo3(get)]
    pub author_count: u32,
    #[pyo3(get)]
    pub volume: Option<String>,
    #[pyo3(get)]
    pub issue: Option<String>,
    #[pyo3(get)]
    pub pages: Option<String>,
    #[pyo3(get)]
    pub language: Option<String>,
    #[pyo3(get)]
    pub journal_abbreviation: Option<String>,
    #[pyo3(get)]
    pub issn: Option<String>,
    inner: Arc<PubMedArticle>,
}

impl From<PubMedArticle> for PyPubMedArticle {
    fn from(article: PubMedArticle) -> Self {
        PyPubMedArticle {
            pmid: article.pmid.clone(),
            title: article.title.clone(),
            journal: article.journal.clone(),
            pub_date: article.pub_date.clone(),
            doi: article.doi.clone(),
            pmc_id: article.pmc_id.clone(),
            abstract_text: article.abstract_text.clone(),
            author_count: article.author_count,
            volume: article.volume.clone(),
            issue: article.issue.clone(),
            pages: article.pages.clone(),
            language: article.language.clone(),
            journal_abbreviation: article.journal_abbreviation.clone(),
            issn: article.issn.clone(),
            inner: Arc::new(article),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPubMedArticle {
    /// Get list of authors
    fn authors(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for author in &self.inner.authors {
            let py_author = PyAuthor::from(author);
            list.append(py_author)?;
        }
        Ok(list.into())
    }

    /// Get article types
    fn article_types(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::new(py, &self.inner.article_types)?;
        Ok(list.into())
    }

    /// Get keywords
    fn keywords(&self, py: Python) -> PyResult<Py<PyAny>> {
        match &self.inner.keywords {
            Some(keywords) => {
                let list = PyList::new(py, keywords)?;
                Ok(list.into())
            }
            None => Ok(py.None()),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PubMedArticle(pmid='{}', title='{}')",
            self.pmid, self.title
        )
    }
}

/// Python wrapper for RelatedArticles
#[gen_stub_pyclass]
#[pyclass(name = "RelatedArticles")]
#[derive(Clone)]
pub struct PyRelatedArticles {
    #[pyo3(get)]
    pub source_pmids: Vec<u32>,
    #[pyo3(get)]
    pub related_pmids: Vec<u32>,
    #[pyo3(get)]
    pub link_type: String,
}

impl From<pubmed::RelatedArticles> for PyRelatedArticles {
    fn from(related: pubmed::RelatedArticles) -> Self {
        PyRelatedArticles {
            source_pmids: related.source_pmids,
            related_pmids: related.related_pmids,
            link_type: related.link_type,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRelatedArticles {
    fn __repr__(&self) -> String {
        format!(
            "RelatedArticles(source_pmids={:?}, related_count={})",
            self.source_pmids,
            self.related_pmids.len()
        )
    }

    fn __len__(&self) -> usize {
        self.related_pmids.len()
    }
}

/// Python wrapper for PmcLinks
#[gen_stub_pyclass]
#[pyclass(name = "PmcLinks")]
#[derive(Clone)]
pub struct PyPmcLinks {
    #[pyo3(get)]
    pub source_pmids: Vec<u32>,
    #[pyo3(get)]
    pub pmc_ids: Vec<String>,
}

impl From<pubmed::PmcLinks> for PyPmcLinks {
    fn from(links: pubmed::PmcLinks) -> Self {
        PyPmcLinks {
            source_pmids: links.source_pmids,
            pmc_ids: links.pmc_ids,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPmcLinks {
    fn __repr__(&self) -> String {
        format!(
            "PmcLinks(source_pmids={:?}, pmc_count={})",
            self.source_pmids,
            self.pmc_ids.len()
        )
    }

    fn __len__(&self) -> usize {
        self.pmc_ids.len()
    }
}

/// Python wrapper for Citations
#[gen_stub_pyclass]
#[pyclass(name = "Citations")]
#[derive(Clone)]
pub struct PyCitations {
    #[pyo3(get)]
    pub source_pmids: Vec<u32>,
    #[pyo3(get)]
    pub citing_pmids: Vec<u32>,
}

impl From<pubmed::Citations> for PyCitations {
    fn from(citations: pubmed::Citations) -> Self {
        PyCitations {
            source_pmids: citations.source_pmids,
            citing_pmids: citations.citing_pmids,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCitations {
    fn __repr__(&self) -> String {
        format!(
            "Citations(source_pmids={:?}, citing_count={})",
            self.source_pmids,
            self.citing_pmids.len()
        )
    }

    fn __len__(&self) -> usize {
        self.citing_pmids.len()
    }
}

/// Python wrapper for DatabaseInfo
#[gen_stub_pyclass]
#[pyclass(name = "DatabaseInfo")]
#[derive(Clone)]
pub struct PyDatabaseInfo {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub menu_name: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub build: Option<String>,
    #[pyo3(get)]
    pub count: Option<u64>,
    #[pyo3(get)]
    pub last_update: Option<String>,
}

impl From<pubmed::DatabaseInfo> for PyDatabaseInfo {
    fn from(info: pubmed::DatabaseInfo) -> Self {
        PyDatabaseInfo {
            name: info.name,
            menu_name: info.menu_name,
            description: info.description,
            build: info.build,
            count: info.count,
            last_update: info.last_update,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDatabaseInfo {
    fn __repr__(&self) -> String {
        format!(
            "DatabaseInfo(name='{}', description='{}')",
            self.name, self.description
        )
    }
}

// ================================================================================================
// ECitMatch API types
// ================================================================================================

/// Input for a single citation match query
///
/// Used with the ECitMatch API to find PMIDs from citation information
/// (journal, year, volume, page, author).
///
/// Examples:
///     >>> query = CitationQuery(
///     ...     journal="proc natl acad sci u s a",
///     ...     year="1991",
///     ...     volume="88",
///     ...     first_page="3248",
///     ...     author_name="mann bj",
///     ...     key="Art1",
///     ... )
#[gen_stub_pyclass]
#[pyclass(name = "CitationQuery")]
#[derive(Clone)]
pub struct PyCitationQuery {
    #[pyo3(get)]
    pub journal: String,
    #[pyo3(get)]
    pub year: String,
    #[pyo3(get)]
    pub volume: String,
    #[pyo3(get)]
    pub first_page: String,
    #[pyo3(get)]
    pub author_name: String,
    #[pyo3(get)]
    pub key: String,
}

impl From<&PyCitationQuery> for pubmed::CitationQuery {
    fn from(query: &PyCitationQuery) -> Self {
        pubmed::CitationQuery::new(
            &query.journal,
            &query.year,
            &query.volume,
            &query.first_page,
            &query.author_name,
            &query.key,
        )
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCitationQuery {
    #[new]
    fn new(
        journal: String,
        year: String,
        volume: String,
        first_page: String,
        author_name: String,
        key: String,
    ) -> Self {
        PyCitationQuery {
            journal,
            year,
            volume,
            first_page,
            author_name,
            key,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "CitationQuery(journal='{}', year='{}', key='{}')",
            self.journal, self.year, self.key
        )
    }
}

/// Result of a single citation match from the ECitMatch API
///
/// Attributes:
///     journal: Journal title from the query
///     year: Year from the query
///     volume: Volume from the query
///     first_page: First page from the query
///     author_name: Author name from the query
///     key: User-defined key from the query
///     pmid: Matched PMID (None if not found)
///     status: Match status ("found", "not_found", or "ambiguous")
#[gen_stub_pyclass]
#[pyclass(name = "CitationMatch")]
#[derive(Clone)]
pub struct PyCitationMatch {
    #[pyo3(get)]
    pub journal: String,
    #[pyo3(get)]
    pub year: String,
    #[pyo3(get)]
    pub volume: String,
    #[pyo3(get)]
    pub first_page: String,
    #[pyo3(get)]
    pub author_name: String,
    #[pyo3(get)]
    pub key: String,
    #[pyo3(get)]
    pub pmid: Option<String>,
    #[pyo3(get)]
    pub status: String,
}

impl From<&pubmed::CitationMatch> for PyCitationMatch {
    fn from(m: &pubmed::CitationMatch) -> Self {
        let status = match m.status {
            pubmed::CitationMatchStatus::Found => "found".to_string(),
            pubmed::CitationMatchStatus::NotFound => "not_found".to_string(),
            pubmed::CitationMatchStatus::Ambiguous => "ambiguous".to_string(),
        };
        PyCitationMatch {
            journal: m.journal.clone(),
            year: m.year.clone(),
            volume: m.volume.clone(),
            first_page: m.first_page.clone(),
            author_name: m.author_name.clone(),
            key: m.key.clone(),
            pmid: m.pmid.clone(),
            status,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCitationMatch {
    fn __repr__(&self) -> String {
        format!(
            "CitationMatch(key='{}', pmid={:?}, status='{}')",
            self.key, self.pmid, self.status
        )
    }
}

/// Results from ECitMatch API for batch citation matching
///
/// Attributes:
///     matches: List of CitationMatch results
#[gen_stub_pyclass]
#[pyclass(name = "CitationMatches")]
#[derive(Clone)]
pub struct PyCitationMatches {
    inner_matches: Vec<PyCitationMatch>,
}

impl From<pubmed::CitationMatches> for PyCitationMatches {
    fn from(results: pubmed::CitationMatches) -> Self {
        PyCitationMatches {
            inner_matches: results.matches.iter().map(PyCitationMatch::from).collect(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyCitationMatches {
    /// Get the list of citation match results
    #[getter]
    fn matches(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for m in &self.inner_matches {
            list.append(m.clone())?;
        }
        Ok(list.into())
    }

    /// Get the number of successful matches
    fn found_count(&self) -> usize {
        self.inner_matches
            .iter()
            .filter(|m| m.status == "found")
            .count()
    }

    fn __repr__(&self) -> String {
        format!(
            "CitationMatches(total={}, found={})",
            self.inner_matches.len(),
            self.found_count()
        )
    }

    fn __len__(&self) -> usize {
        self.inner_matches.len()
    }
}

// ================================================================================================
// EGQuery API types
// ================================================================================================

/// Record count for a single NCBI database from the EGQuery API
///
/// Attributes:
///     db_name: Internal database name (e.g., "pubmed", "pmc")
///     menu_name: Human-readable database name (e.g., "PubMed", "PMC")
///     count: Number of matching records
///     status: Query status (e.g., "Ok")
#[gen_stub_pyclass]
#[pyclass(name = "DatabaseCount")]
#[derive(Clone)]
pub struct PyDatabaseCount {
    #[pyo3(get)]
    pub db_name: String,
    #[pyo3(get)]
    pub menu_name: String,
    #[pyo3(get)]
    pub count: u64,
    #[pyo3(get)]
    pub status: String,
}

impl From<&pubmed::DatabaseCount> for PyDatabaseCount {
    fn from(dc: &pubmed::DatabaseCount) -> Self {
        PyDatabaseCount {
            db_name: dc.db_name.clone(),
            menu_name: dc.menu_name.clone(),
            count: dc.count,
            status: dc.status.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDatabaseCount {
    fn __repr__(&self) -> String {
        format!(
            "DatabaseCount(db_name='{}', count={})",
            self.db_name, self.count
        )
    }
}

/// Results from EGQuery API for global database search
///
/// Attributes:
///     term: The query term that was searched
///     results: List of DatabaseCount results for each database
#[gen_stub_pyclass]
#[pyclass(name = "GlobalQueryResults")]
#[derive(Clone)]
pub struct PyGlobalQueryResults {
    #[pyo3(get)]
    pub term: String,
    inner_results: Vec<PyDatabaseCount>,
}

impl From<pubmed::GlobalQueryResults> for PyGlobalQueryResults {
    fn from(results: pubmed::GlobalQueryResults) -> Self {
        PyGlobalQueryResults {
            term: results.term,
            inner_results: results.results.iter().map(PyDatabaseCount::from).collect(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyGlobalQueryResults {
    /// Get the list of database count results
    #[getter]
    fn results(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for r in &self.inner_results {
            list.append(r.clone())?;
        }
        Ok(list.into())
    }

    /// Get results with count > 0
    fn non_zero(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for r in &self.inner_results {
            if r.count > 0 {
                list.append(r.clone())?;
            }
        }
        Ok(list.into())
    }

    /// Get count for a specific database
    fn count_for(&self, db_name: &str) -> Option<u64> {
        self.inner_results
            .iter()
            .find(|r| r.db_name == db_name)
            .map(|r| r.count)
    }

    fn __repr__(&self) -> String {
        format!(
            "GlobalQueryResults(term='{}', databases={})",
            self.term,
            self.inner_results.len()
        )
    }

    fn __len__(&self) -> usize {
        self.inner_results.len()
    }
}
