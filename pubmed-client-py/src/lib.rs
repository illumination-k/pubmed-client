// Python bindings for pubmed-client-rs using PyO3
//! Python bindings for PubMed and PMC API client
//!
//! This module provides Python bindings for the Rust-based PubMed client library.

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::sync::Arc;
use tokio::runtime::Runtime;

use ::pubmed_client::{
    config::ClientConfig, pmc, pubmed, Client, PmcClient, PmcFullText, PubMedArticle,
    PubMedClient as RustPubMedClient,
};

// ================================================================================================
// Runtime Management
// ================================================================================================

/// Get or create a Tokio runtime for blocking operations
fn get_runtime() -> Runtime {
    Runtime::new().expect("Failed to create Tokio runtime")
}

// ================================================================================================
// Error Handling
// ================================================================================================

/// Convert Rust errors to Python exceptions
fn to_py_err(err: ::pubmed_client::error::PubMedError) -> PyErr {
    PyErr::new::<PyException, _>(format!("{}", err))
}

// ================================================================================================
// PubMed Data Models
// ================================================================================================

/// Python wrapper for Author affiliation
#[pyclass(name = "Affiliation")]
#[derive(Clone)]
struct PyAffiliation {
    #[pyo3(get)]
    institution: Option<String>,
    #[pyo3(get)]
    department: Option<String>,
    #[pyo3(get)]
    address: Option<String>,
    #[pyo3(get)]
    country: Option<String>,
    #[pyo3(get)]
    email: Option<String>,
}

impl From<&pubmed::Affiliation> for PyAffiliation {
    fn from(affiliation: &pubmed::Affiliation) -> Self {
        PyAffiliation {
            institution: affiliation.institution.clone(),
            department: affiliation.department.clone(),
            address: affiliation.address.clone(),
            country: affiliation.country.clone(),
            email: affiliation.email.clone(),
        }
    }
}

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
#[pyclass(name = "Author")]
#[derive(Clone)]
struct PyAuthor {
    #[pyo3(get)]
    last_name: Option<String>,
    #[pyo3(get)]
    fore_name: Option<String>,
    #[pyo3(get)]
    first_name: Option<String>,
    #[pyo3(get)]
    middle_name: Option<String>,
    #[pyo3(get)]
    initials: Option<String>,
    #[pyo3(get)]
    suffix: Option<String>,
    #[pyo3(get)]
    full_name: String,
    #[pyo3(get)]
    orcid: Option<String>,
    #[pyo3(get)]
    is_corresponding: bool,
    inner: Arc<pubmed::Author>,
}

impl From<&pubmed::Author> for PyAuthor {
    fn from(author: &pubmed::Author) -> Self {
        PyAuthor {
            last_name: author.last_name.clone(),
            fore_name: author.fore_name.clone(),
            first_name: author.first_name.clone(),
            middle_name: author.middle_name.clone(),
            initials: author.initials.clone(),
            suffix: author.suffix.clone(),
            full_name: author.full_name.clone(),
            orcid: author.orcid.clone(),
            is_corresponding: author.is_corresponding,
            inner: Arc::new(author.clone()),
        }
    }
}

#[pymethods]
impl PyAuthor {
    /// Get list of affiliations
    fn affiliations(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for affiliation in &self.inner.affiliations {
            let py_affiliation = PyAffiliation::from(affiliation);
            list.append(py_affiliation)?;
        }
        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!("Author(full_name='{}')", self.full_name)
    }
}

/// Python wrapper for PubMedArticle
#[pyclass(name = "PubMedArticle")]
#[derive(Clone)]
struct PyPubMedArticle {
    #[pyo3(get)]
    pmid: String,
    #[pyo3(get)]
    title: String,
    #[pyo3(get)]
    journal: String,
    #[pyo3(get)]
    pub_date: String,
    #[pyo3(get)]
    doi: Option<String>,
    #[pyo3(get)]
    pmc_id: Option<String>,
    #[pyo3(get)]
    abstract_text: Option<String>,
    #[pyo3(get)]
    author_count: u32,
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

#[pymethods]
impl PyPubMedArticle {
    /// Get list of authors
    fn authors(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for author in &self.inner.authors {
            let py_author = PyAuthor::from(author);
            list.append(py_author)?;
        }
        Ok(list.into())
    }

    /// Get article types
    fn article_types(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::new(py, &self.inner.article_types)?;
        Ok(list.into())
    }

    /// Get keywords
    fn keywords(&self, py: Python) -> PyResult<PyObject> {
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
#[pyclass(name = "RelatedArticles")]
#[derive(Clone)]
struct PyRelatedArticles {
    #[pyo3(get)]
    source_pmids: Vec<u32>,
    #[pyo3(get)]
    related_pmids: Vec<u32>,
    #[pyo3(get)]
    link_type: String,
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
#[pyclass(name = "PmcLinks")]
#[derive(Clone)]
struct PyPmcLinks {
    #[pyo3(get)]
    source_pmids: Vec<u32>,
    #[pyo3(get)]
    pmc_ids: Vec<String>,
}

impl From<pubmed::PmcLinks> for PyPmcLinks {
    fn from(links: pubmed::PmcLinks) -> Self {
        PyPmcLinks {
            source_pmids: links.source_pmids,
            pmc_ids: links.pmc_ids,
        }
    }
}

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
#[pyclass(name = "Citations")]
#[derive(Clone)]
struct PyCitations {
    #[pyo3(get)]
    source_pmids: Vec<u32>,
    #[pyo3(get)]
    citing_pmids: Vec<u32>,
}

impl From<pubmed::Citations> for PyCitations {
    fn from(citations: pubmed::Citations) -> Self {
        PyCitations {
            source_pmids: citations.source_pmids,
            citing_pmids: citations.citing_pmids,
        }
    }
}

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
#[pyclass(name = "DatabaseInfo")]
#[derive(Clone)]
struct PyDatabaseInfo {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    menu_name: String,
    #[pyo3(get)]
    description: String,
    #[pyo3(get)]
    build: Option<String>,
    #[pyo3(get)]
    count: Option<u64>,
    #[pyo3(get)]
    last_update: Option<String>,
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
// PMC Data Models
// ================================================================================================

/// Python wrapper for PMC Affiliation
#[pyclass(name = "PmcAffiliation")]
#[derive(Clone)]
struct PyPmcAffiliation {
    #[pyo3(get)]
    id: Option<String>,
    #[pyo3(get)]
    institution: String,
    #[pyo3(get)]
    department: Option<String>,
    #[pyo3(get)]
    address: Option<String>,
    #[pyo3(get)]
    country: Option<String>,
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

#[pymethods]
impl PyPmcAffiliation {
    fn __repr__(&self) -> String {
        format!("PmcAffiliation(institution='{}')", self.institution)
    }
}

/// Python wrapper for PMC Author
#[pyclass(name = "PmcAuthor")]
#[derive(Clone)]
struct PyPmcAuthor {
    #[pyo3(get)]
    given_names: Option<String>,
    #[pyo3(get)]
    surname: Option<String>,
    #[pyo3(get)]
    full_name: String,
    #[pyo3(get)]
    orcid: Option<String>,
    #[pyo3(get)]
    email: Option<String>,
    #[pyo3(get)]
    is_corresponding: bool,
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

#[pymethods]
impl PyPmcAuthor {
    /// Get list of affiliations
    fn affiliations(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for affiliation in &self.inner.affiliations {
            let py_affiliation = PyPmcAffiliation::from(affiliation);
            list.append(py_affiliation)?;
        }
        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!("PmcAuthor(full_name='{}')", self.full_name)
    }
}

/// Python wrapper for Figure
#[pyclass(name = "Figure")]
#[derive(Clone)]
struct PyFigure {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    label: Option<String>,
    #[pyo3(get)]
    caption: String,
    #[pyo3(get)]
    alt_text: Option<String>,
    #[pyo3(get)]
    fig_type: Option<String>,
    #[pyo3(get)]
    file_path: Option<String>,
    #[pyo3(get)]
    file_name: Option<String>,
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

#[pymethods]
impl PyFigure {
    fn __repr__(&self) -> String {
        format!("Figure(id='{}', label={:?})", self.id, self.label)
    }
}

/// Python wrapper for Table
#[pyclass(name = "Table")]
#[derive(Clone)]
struct PyTable {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    label: Option<String>,
    #[pyo3(get)]
    caption: String,
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

#[pymethods]
impl PyTable {
    fn __repr__(&self) -> String {
        format!("Table(id='{}', label={:?})", self.id, self.label)
    }
}

/// Python wrapper for Reference
#[pyclass(name = "Reference")]
#[derive(Clone)]
struct PyReference {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    title: Option<String>,
    #[pyo3(get)]
    journal: Option<String>,
    #[pyo3(get)]
    year: Option<String>,
    #[pyo3(get)]
    pmid: Option<String>,
    #[pyo3(get)]
    doi: Option<String>,
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

#[pymethods]
impl PyReference {
    fn __repr__(&self) -> String {
        format!("Reference(id='{}')", self.id)
    }
}

/// Python wrapper for ArticleSection
#[pyclass(name = "ArticleSection")]
#[derive(Clone)]
struct PyArticleSection {
    #[pyo3(get)]
    title: Option<String>,
    #[pyo3(get)]
    content: String,
    #[pyo3(get)]
    section_type: Option<String>,
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

#[pymethods]
impl PyArticleSection {
    fn __repr__(&self) -> String {
        format!("ArticleSection(title={:?})", self.title)
    }
}

/// Python wrapper for PmcFullText
#[pyclass(name = "PmcFullText")]
#[derive(Clone)]
struct PyPmcFullText {
    #[pyo3(get)]
    pmcid: String,
    #[pyo3(get)]
    pmid: Option<String>,
    #[pyo3(get)]
    title: String,
    #[pyo3(get)]
    doi: Option<String>,
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

#[pymethods]
impl PyPmcFullText {
    /// Get list of authors
    fn authors(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for author in &self.inner.authors {
            let py_author = PyPmcAuthor::from(author);
            list.append(py_author)?;
        }
        Ok(list.into())
    }

    /// Get list of sections
    fn sections(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for section in &self.inner.sections {
            let py_section = PyArticleSection::from(section);
            list.append(py_section)?;
        }
        Ok(list.into())
    }

    /// Get list of all figures from all sections
    fn figures(&self, py: Python) -> PyResult<PyObject> {
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
    fn tables(&self, py: Python) -> PyResult<PyObject> {
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
    fn references(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty(py);
        for reference in &self.inner.references {
            let py_reference = PyReference::from(reference);
            list.append(py_reference)?;
        }
        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PmcFullText(pmcid='{}', title='{}')",
            self.pmcid, self.title
        )
    }
}

// ================================================================================================
// Configuration
// ================================================================================================

/// Python wrapper for ClientConfig
///
/// Configuration for PubMed and PMC clients.
///
/// Examples:
///     >>> config = ClientConfig()
///     >>> config.with_api_key("your_api_key").with_email("you@example.com")
///     >>> client = Client.with_config(config)
#[pyclass(name = "ClientConfig")]
#[derive(Clone)]
struct PyClientConfig {
    inner: ClientConfig,
}

#[pymethods]
impl PyClientConfig {
    /// Create a new configuration with default settings
    #[new]
    fn new() -> Self {
        PyClientConfig {
            inner: ClientConfig::new(),
        }
    }

    /// Set the NCBI API key for increased rate limits (10 req/sec instead of 3)
    fn with_api_key(mut slf: PyRefMut<Self>, api_key: String) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_api_key(&api_key);
        slf
    }

    /// Set the email address for identification (recommended by NCBI)
    fn with_email(mut slf: PyRefMut<Self>, email: String) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_email(&email);
        slf
    }

    /// Set the tool name for identification (default: "pubmed-client-py")
    fn with_tool(mut slf: PyRefMut<Self>, tool: String) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_tool(&tool);
        slf
    }

    /// Set custom rate limit in requests per second
    fn with_rate_limit(mut slf: PyRefMut<Self>, rate_limit: f64) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_rate_limit(rate_limit);
        slf
    }

    /// Set HTTP request timeout in seconds
    fn with_timeout_seconds(mut slf: PyRefMut<Self>, timeout_seconds: u64) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_timeout_seconds(timeout_seconds);
        slf
    }

    /// Enable default response caching
    fn with_cache(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_cache();
        slf
    }

    fn __repr__(&self) -> String {
        "ClientConfig(...)".to_string()
    }
}

// ================================================================================================
// Client Implementations
// ================================================================================================

/// PubMed client for searching and fetching article metadata
///
/// Examples:
///     >>> client = PubMedClient()
///     >>> articles = client.search_and_fetch("covid-19", 10)
///     >>> article = client.fetch_article("31978945")
#[pyclass(name = "PubMedClient")]
struct PyPubMedClient {
    client: Arc<RustPubMedClient>,
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

/// PMC client for fetching full-text articles
///
/// Examples:
///     >>> client = PmcClient()
///     >>> full_text = client.fetch_full_text("PMC7906746")
///     >>> pmcid = client.check_pmc_availability("31978945")
#[pyclass(name = "PmcClient")]
struct PyPmcClient {
    client: Arc<PmcClient>,
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
#[pyclass(name = "Client")]
struct PyClient {
    client: Arc<Client>,
}

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
        py.allow_threads(|| {
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
        py.allow_threads(|| {
            let rt = get_runtime();
            rt.block_on(client.get_database_list()).map_err(to_py_err)
        })
    }

    /// Get detailed information about a specific database
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

    /// Get PMC links for given PMIDs
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
        "Client()".to_string()
    }
}

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
    m.add_class::<PyTable>()?;
    m.add_class::<PyReference>()?;
    m.add_class::<PyArticleSection>()?;
    m.add_class::<PyPmcFullText>()?;

    // Add clients
    m.add_class::<PyPubMedClient>()?;
    m.add_class::<PyPmcClient>()?;
    m.add_class::<PyClient>()?;

    Ok(())
}
