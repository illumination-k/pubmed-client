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
