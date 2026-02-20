//! PubMed client for Python bindings
//!
//! This module provides Python wrappers for the PubMed client.

use pyo3::prelude::*;
use pyo3_stub_gen::PyStubType;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::sync::Arc;

use pubmed_client::PubMedClient as RustPubMedClient;

use crate::config::PyClientConfig;
use crate::query::PySearchQuery;
use crate::utils::{get_runtime, to_py_err};

use super::models::{
    PyCitationMatches, PyCitationQuery, PyCitations, PyDatabaseInfo, PyGlobalQueryResults,
    PyPmcLinks, PyPubMedArticle, PyRelatedArticles,
};

// ================================================================================================
// Query Input Type for Union[str, SearchQuery]
// ================================================================================================

/// Enum to handle Python's Union[str, SearchQuery] type
/// This is an internal type and not exposed to Python
#[derive(FromPyObject)]
enum QueryInput {
    /// String query (e.g., "covid-19")
    String(String),
    /// SearchQuery object
    SearchQuery(PySearchQuery),
}

impl QueryInput {
    /// Convert QueryInput to a query string
    fn to_query_string(&self) -> PyResult<String> {
        match self {
            QueryInput::String(s) => Ok(s.clone()),
            QueryInput::SearchQuery(q) => Ok(q.inner.build()),
        }
    }

    /// Get the limit from QueryInput
    ///
    /// - For String input: returns the provided default limit
    /// - For SearchQuery input: returns the query's limit
    fn get_limit(&self, default_limit: usize) -> usize {
        match self {
            QueryInput::String(_) => default_limit,
            QueryInput::SearchQuery(q) => q.inner.get_limit(),
        }
    }
}

// Implement PyStubType for QueryInput to represent it as str | SearchQuery in stubs
impl PyStubType for QueryInput {
    fn type_output() -> pyo3_stub_gen::TypeInfo {
        pyo3_stub_gen::TypeInfo::builtin("str | SearchQuery")
    }
}

// ================================================================================================
// Client Implementation
// ================================================================================================

/// PubMed client for searching and fetching article metadata
///
/// Examples:
///     >>> client = PubMedClient()
///     >>> articles = client.search_and_fetch("covid-19", 10)
///     >>> article = client.fetch_article("31978945")
#[gen_stub_pyclass]
#[pyclass(name = "PubMedClient")]
pub struct PyPubMedClient {
    pub client: Arc<RustPubMedClient>,
}

#[gen_stub_pymethods]
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

    /// Search for articles and return PMIDs only
    ///
    /// This method returns only the list of PMIDs matching the query,
    /// which is faster than fetching full article metadata.
    ///
    /// Args:
    ///     query: Search query (either a string or SearchQuery object)
    ///     limit: Maximum number of PMIDs to return (ignored if query is SearchQuery)
    ///
    /// Returns:
    ///     List of PMIDs as strings
    ///
    /// Examples:
    ///     >>> client = PubMedClient()
    ///     >>> # Using string query
    ///     >>> pmids = client.search_articles("covid-19", 100)
    ///     >>> pmids = client.search_articles("cancer[ti] AND therapy[tiab]", 50)
    ///     >>> # Using SearchQuery object
    ///     >>> query = SearchQuery().query("covid-19").limit(100)
    ///     >>> pmids = client.search_articles(query, 0)  # limit parameter ignored
    #[pyo3(signature = (query, limit))]
    #[pyo3(text_signature = "(query: str | SearchQuery, limit: int) -> list[str]")]
    fn search_articles(
        &self,
        py: Python,
        query: QueryInput,
        limit: usize,
    ) -> PyResult<Vec<String>> {
        let client = self.client.clone();
        let query_string = query.to_query_string()?;
        let actual_limit = query.get_limit(limit);

        py.detach(|| {
            let rt = get_runtime();
            rt.block_on(client.search_articles(&query_string, actual_limit))
                .map_err(to_py_err)
        })
    }

    /// Search for articles and fetch their metadata
    ///
    /// Args:
    ///     query: Search query (either a string or SearchQuery object)
    ///     limit: Maximum number of articles to return (ignored if query is SearchQuery)
    ///
    /// Returns:
    ///     List of PubMedArticle objects
    ///
    /// Examples:
    ///     >>> client = PubMedClient()
    ///     >>> # Using string query
    ///     >>> articles = client.search_and_fetch("covid-19", 10)
    ///     >>> # Using SearchQuery object
    ///     >>> query = SearchQuery().query("cancer").published_after(2020).limit(50)
    ///     >>> articles = client.search_and_fetch(query, 0)  # limit parameter ignored
    #[pyo3(signature = (query, limit))]
    #[pyo3(text_signature = "(query: str | SearchQuery, limit: int) -> list[PubMedArticle]")]
    fn search_and_fetch(
        &self,
        py: Python,
        query: QueryInput,
        limit: usize,
    ) -> PyResult<Vec<PyPubMedArticle>> {
        let client = self.client.clone();
        let query_string = query.to_query_string()?;
        let actual_limit = query.get_limit(limit);

        py.detach(|| {
            let rt = get_runtime();
            let articles = rt
                .block_on(client.search_and_fetch(&query_string, actual_limit))
                .map_err(to_py_err)?;
            Ok(articles.into_iter().map(PyPubMedArticle::from).collect())
        })
    }

    /// Fetch multiple articles by PMIDs in a single batch request
    ///
    /// This is significantly more efficient than fetching articles one by one,
    /// as it sends fewer HTTP requests to the NCBI API. For large numbers of PMIDs,
    /// the request is automatically split into batches of 200.
    ///
    /// Args:
    ///     pmids: List of PubMed IDs as strings
    ///
    /// Returns:
    ///     List of PubMedArticle objects
    ///
    /// Examples:
    ///     >>> client = PubMedClient()
    ///     >>> articles = client.fetch_articles(["31978945", "33515491", "25760099"])
    ///     >>> for article in articles:
    ///     ...     print(f"{article.pmid}: {article.title}")
    #[pyo3(text_signature = "(pmids: list[str]) -> list[PubMedArticle]")]
    fn fetch_articles(&self, py: Python, pmids: Vec<String>) -> PyResult<Vec<PyPubMedArticle>> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            let articles = rt
                .block_on(client.fetch_articles(&pmid_refs))
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
        py.detach(|| {
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
        py.detach(|| {
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
        py.detach(|| {
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
        py.detach(|| {
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
        py.detach(|| {
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
        py.detach(|| {
            let rt = get_runtime();
            let citations = rt
                .block_on(client.get_citations(&pmids))
                .map_err(to_py_err)?;
            Ok(PyCitations::from(citations))
        })
    }

    /// Match citations to PMIDs using the ECitMatch API
    ///
    /// Takes citation information (journal, year, volume, page, author)
    /// and returns corresponding PMIDs. Useful for identifying PMIDs
    /// from reference lists.
    ///
    /// Args:
    ///     citations: List of CitationQuery objects
    ///
    /// Returns:
    ///     CitationMatches object containing match results
    ///
    /// Examples:
    ///     >>> client = PubMedClient()
    ///     >>> queries = [
    ///     ...     CitationQuery("proc natl acad sci u s a", "1991", "88", "3248", "mann bj", "Art1"),
    ///     ...     CitationQuery("science", "1987", "235", "182", "palmenberg ac", "Art2"),
    ///     ... ]
    ///     >>> results = client.match_citations(queries)
    ///     >>> for m in results.matches:
    ///     ...     print(f"{m.key}: {m.pmid} ({m.status})")
    fn match_citations(
        &self,
        py: Python,
        citations: Vec<PyCitationQuery>,
    ) -> PyResult<PyCitationMatches> {
        let client = self.client.clone();
        let rust_citations: Vec<pubmed_client::CitationQuery> =
            citations.iter().map(|c| c.into()).collect();

        py.detach(|| {
            let rt = get_runtime();
            let results = rt
                .block_on(client.match_citations(&rust_citations))
                .map_err(to_py_err)?;
            Ok(PyCitationMatches::from(results))
        })
    }

    /// Query all NCBI databases for record counts using the EGQuery API
    ///
    /// Returns the number of records matching the query in each
    /// Entrez database. Useful for exploratory searches.
    ///
    /// Args:
    ///     term: Search query string
    ///
    /// Returns:
    ///     GlobalQueryResults object containing counts per database
    ///
    /// Examples:
    ///     >>> client = PubMedClient()
    ///     >>> results = client.global_query("asthma")
    ///     >>> for db in results.non_zero():
    ///     ...     print(f"{db.menu_name}: {db.count}")
    fn global_query(&self, py: Python, term: String) -> PyResult<PyGlobalQueryResults> {
        let client = self.client.clone();
        py.detach(|| {
            let rt = get_runtime();
            let results = rt.block_on(client.global_query(&term)).map_err(to_py_err)?;
            Ok(PyGlobalQueryResults::from(results))
        })
    }

    fn __repr__(&self) -> String {
        "PubMedClient()".to_string()
    }
}
