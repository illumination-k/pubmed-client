//! PMC data models for Python bindings
//!
//! This module provides Python wrappers for PMC data structures.

use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::sync::Arc;

use pubmed_client::pmc::{self, markdown::PmcMarkdownConverter};
use pubmed_client::{ExtractedFigure, PmcFullText};

// ================================================================================================
// PMC Data Models
// ================================================================================================

/// Python wrapper for PMC Affiliation
#[gen_stub_pyclass]
#[pyclass(name = "PmcAffiliation")]
#[derive(Clone)]
pub struct PyPmcAffiliation {
    #[pyo3(get)]
    pub id: Option<String>,
    #[pyo3(get)]
    pub institution: Option<String>,
    #[pyo3(get)]
    pub department: Option<String>,
    #[pyo3(get)]
    pub address: Option<String>,
    #[pyo3(get)]
    pub country: Option<String>,
}

impl From<&pmc::Affiliation> for PyPmcAffiliation {
    fn from(affiliation: &pmc::Affiliation) -> Self {
        PyPmcAffiliation {
            id: affiliation.id.clone(),
            institution: affiliation.institution.clone(),
            department: affiliation.department.clone(),
            address: affiliation.address.clone(),
            country: affiliation.country.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPmcAffiliation {
    fn __repr__(&self) -> String {
        format!("PmcAffiliation(institution={:?})", self.institution)
    }
}

/// Python wrapper for PMC Author
#[gen_stub_pyclass]
#[pyclass(name = "PmcAuthor")]
#[derive(Clone)]
pub struct PyPmcAuthor {
    #[pyo3(get)]
    pub given_names: Option<String>,
    #[pyo3(get)]
    pub surname: Option<String>,
    #[pyo3(get)]
    pub full_name: String,
    #[pyo3(get)]
    pub orcid: Option<String>,
    #[pyo3(get)]
    pub email: Option<String>,
    #[pyo3(get)]
    pub is_corresponding: bool,
    inner: Arc<pmc::Author>,
}

impl From<&pmc::Author> for PyPmcAuthor {
    fn from(author: &pmc::Author) -> Self {
        PyPmcAuthor {
            given_names: author.given_names.clone(),
            surname: author.surname.clone(),
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
impl PyPmcAuthor {
    /// Get list of affiliations
    fn affiliations(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for affiliation in &self.inner.affiliations {
            let py_affiliation = PyPmcAffiliation::from(affiliation);
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
        format!("PmcAuthor(full_name='{}')", self.full_name)
    }
}

/// Python wrapper for Figure
#[gen_stub_pyclass]
#[pyclass(name = "Figure")]
#[derive(Clone)]
pub struct PyFigure {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub label: Option<String>,
    #[pyo3(get)]
    pub caption: String,
    #[pyo3(get)]
    pub alt_text: Option<String>,
    #[pyo3(get)]
    pub fig_type: Option<String>,
    #[pyo3(get)]
    pub file_path: Option<String>,
    #[pyo3(get)]
    pub file_name: Option<String>,
}

impl From<&pmc::Figure> for PyFigure {
    fn from(figure: &pmc::Figure) -> Self {
        PyFigure {
            id: figure.id.clone(),
            label: figure.label.clone(),
            caption: figure.caption.clone(),
            alt_text: figure.alt_text.clone(),
            fig_type: figure.fig_type.clone(),
            file_path: figure.file_path.clone(),
            file_name: figure.file_name.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyFigure {
    fn __repr__(&self) -> String {
        format!("Figure(id='{}', label={:?})", self.id, self.label)
    }
}

/// Python wrapper for ExtractedFigure
///
/// Represents a figure that has been extracted from a PMC tar.gz archive,
/// combining XML metadata with actual file information.
#[gen_stub_pyclass]
#[pyclass(name = "ExtractedFigure")]
#[derive(Clone)]
pub struct PyExtractedFigure {
    /// Figure metadata from XML (caption, label, etc.)
    #[pyo3(get)]
    pub figure: PyFigure,
    /// Actual file path where the figure was extracted
    #[pyo3(get)]
    pub extracted_file_path: String,
    /// File size in bytes
    #[pyo3(get)]
    pub file_size: Option<u64>,
    /// Image dimensions as (width, height) tuple if available
    #[pyo3(get)]
    pub dimensions: Option<(u32, u32)>,
}

impl From<&ExtractedFigure> for PyExtractedFigure {
    fn from(extracted: &ExtractedFigure) -> Self {
        PyExtractedFigure {
            figure: PyFigure::from(&extracted.figure),
            extracted_file_path: extracted.extracted_file_path.clone(),
            file_size: extracted.file_size,
            dimensions: extracted.dimensions,
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyExtractedFigure {
    fn __repr__(&self) -> String {
        format!(
            "ExtractedFigure(id='{}', file='{}', size={:?})",
            self.figure.id, self.extracted_file_path, self.file_size
        )
    }
}

/// Python wrapper for Table
#[gen_stub_pyclass]
#[pyclass(name = "Table")]
#[derive(Clone)]
pub struct PyTable {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub label: Option<String>,
    #[pyo3(get)]
    pub caption: String,
}

impl From<&pmc::Table> for PyTable {
    fn from(table: &pmc::Table) -> Self {
        PyTable {
            id: table.id.clone(),
            label: table.label.clone(),
            caption: table.caption.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyTable {
    fn __repr__(&self) -> String {
        format!("Table(id='{}', label={:?})", self.id, self.label)
    }
}

/// Python wrapper for Reference
#[gen_stub_pyclass]
#[pyclass(name = "Reference")]
#[derive(Clone)]
pub struct PyReference {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub title: Option<String>,
    #[pyo3(get)]
    pub journal: Option<String>,
    #[pyo3(get)]
    pub year: Option<String>,
    #[pyo3(get)]
    pub pmid: Option<String>,
    #[pyo3(get)]
    pub doi: Option<String>,
}

impl From<&pmc::Reference> for PyReference {
    fn from(reference: &pmc::Reference) -> Self {
        PyReference {
            id: reference.id.clone(),
            title: reference.title.clone(),
            journal: reference.journal.clone(),
            year: reference.year.clone(),
            pmid: reference.pmid.clone(),
            doi: reference.doi.clone(),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyReference {
    fn __repr__(&self) -> String {
        format!("Reference(id='{}')", self.id)
    }
}

/// Python wrapper for ArticleSection
#[gen_stub_pyclass]
#[pyclass(name = "ArticleSection")]
#[derive(Clone)]
pub struct PyArticleSection {
    #[pyo3(get)]
    pub title: Option<String>,
    #[pyo3(get)]
    pub content: String,
    #[pyo3(get)]
    pub section_type: Option<String>,
}

impl From<&pmc::ArticleSection> for PyArticleSection {
    fn from(section: &pmc::ArticleSection) -> Self {
        PyArticleSection {
            title: section.title.clone(),
            content: section.content.clone(),
            section_type: Some(section.section_type.clone()),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyArticleSection {
    fn __repr__(&self) -> String {
        format!("ArticleSection(title={:?})", self.title)
    }
}

/// Python wrapper for PmcFullText
#[gen_stub_pyclass]
#[pyclass(name = "PmcFullText")]
#[derive(Clone)]
pub struct PyPmcFullText {
    #[pyo3(get)]
    pub pmcid: String,
    #[pyo3(get)]
    pub pmid: Option<String>,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub doi: Option<String>,
    inner: Arc<PmcFullText>,
}

impl From<PmcFullText> for PyPmcFullText {
    fn from(full_text: PmcFullText) -> Self {
        PyPmcFullText {
            pmcid: full_text.pmcid.clone(),
            pmid: full_text.pmid.clone(),
            title: full_text.title.clone(),
            doi: full_text.doi.clone(),
            inner: Arc::new(full_text),
        }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl PyPmcFullText {
    /// Get list of authors
    fn authors(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for author in &self.inner.authors {
            let py_author = PyPmcAuthor::from(author);
            list.append(py_author)?;
        }
        Ok(list.into())
    }

    /// Get list of sections
    fn sections(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for section in &self.inner.sections {
            let py_section = PyArticleSection::from(section);
            list.append(py_section)?;
        }
        Ok(list.into())
    }

    /// Get list of all figures from all sections
    fn figures(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        // Collect figures from all sections recursively
        fn collect_figures(section: &pmc::ArticleSection, figures: &mut Vec<pmc::Figure>) {
            figures.extend(section.figures.clone());
            for subsection in &section.subsections {
                collect_figures(subsection, figures);
            }
        }

        let mut all_figures = Vec::new();
        for section in &self.inner.sections {
            collect_figures(section, &mut all_figures);
        }

        for figure in all_figures {
            let py_figure = PyFigure::from(&figure);
            list.append(py_figure)?;
        }
        Ok(list.into())
    }

    /// Get list of all tables from all sections
    fn tables(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        // Collect tables from all sections recursively
        fn collect_tables(section: &pmc::ArticleSection, tables: &mut Vec<pmc::Table>) {
            tables.extend(section.tables.clone());
            for subsection in &section.subsections {
                collect_tables(subsection, tables);
            }
        }

        let mut all_tables = Vec::new();
        for section in &self.inner.sections {
            collect_tables(section, &mut all_tables);
        }

        for table in all_tables {
            let py_table = PyTable::from(&table);
            list.append(py_table)?;
        }
        Ok(list.into())
    }

    /// Get list of references
    fn references(&self, py: Python) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for reference in &self.inner.references {
            let py_reference = PyReference::from(reference);
            list.append(py_reference)?;
        }
        Ok(list.into())
    }

    /// Convert the article to Markdown format
    ///
    /// Returns:
    ///     A Markdown-formatted string representation of the article
    ///
    /// Example:
    ///     >>> full_text = client.pmc.fetch_full_text("PMC7906746")
    ///     >>> markdown = full_text.to_markdown()
    ///     >>> print(markdown)
    fn to_markdown(&self) -> String {
        let converter = PmcMarkdownConverter::new();
        converter.convert(&self.inner)
    }

    fn __repr__(&self) -> String {
        format!(
            "PmcFullText(pmcid='{}', title='{}')",
            self.pmcid, self.title
        )
    }
}
