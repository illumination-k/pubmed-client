//! Query builder module for Python bindings
//!
//! This module provides Python wrappers for the SearchQuery builder.

use pubmed_client::pubmed::SearchQuery;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// ================================================================================================
// SearchQuery Builder
// ================================================================================================

/// Python wrapper for SearchQuery
///
/// Builder for constructing PubMed search queries programmatically.
///
/// Examples:
///     >>> query = SearchQuery().query("covid-19").limit(10)
///     >>> query_string = query.build()
///     >>> print(query_string)
///     covid-19
#[pyclass(name = "SearchQuery")]
#[derive(Clone)]
pub struct PySearchQuery {
    pub inner: SearchQuery,
}

#[pymethods]
impl PySearchQuery {
    /// Create a new empty search query builder
    ///
    /// Returns:
    ///     SearchQuery: New query builder instance
    ///
    /// Example:
    ///     >>> query = SearchQuery()
    #[new]
    fn new() -> Self {
        PySearchQuery {
            inner: SearchQuery::new(),
        }
    }

    /// Add a search term to the query
    ///
    /// Terms are accumulated (not replaced) and will be space-separated in the final query.
    /// None and empty strings (after trimming) are silently filtered out.
    ///
    /// Args:
    ///     term: Search term string (None or empty strings are filtered)
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("covid-19").query("treatment")
    ///     >>> query.build()
    ///     'covid-19 treatment'
    #[pyo3(signature = (term=None))]
    fn query(mut slf: PyRefMut<Self>, term: Option<String>) -> PyRefMut<Self> {
        if let Some(t) = term {
            let trimmed = t.trim();
            if !trimmed.is_empty() {
                slf.inner = slf.inner.clone().query(trimmed);
            }
        }
        slf
    }

    /// Add multiple search terms at once
    ///
    /// Each term is processed like query(). None items and empty strings are filtered out.
    ///
    /// Args:
    ///     terms: List of search term strings
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Example:
    ///     >>> query = SearchQuery().terms(["covid-19", "vaccine", "efficacy"])
    ///     >>> query.build()
    ///     'covid-19 vaccine efficacy'
    #[pyo3(signature = (terms=None))]
    fn terms(mut slf: PyRefMut<Self>, terms: Option<Vec<Option<String>>>) -> PyRefMut<Self> {
        if let Some(term_list) = terms {
            for term_opt in term_list {
                if let Some(t) = term_opt {
                    let trimmed = t.trim();
                    if !trimmed.is_empty() {
                        slf.inner = slf.inner.clone().query(trimmed);
                    }
                }
            }
        }
        slf
    }

    /// Set the maximum number of results to return
    ///
    /// Validates that limit is >0 and ≤10,000. None is treated as "use default" (20).
    ///
    /// Args:
    ///     limit: Maximum number of results (None = use default of 20)
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If limit ≤ 0 or limit > 10,000
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("cancer").limit(50)
    #[pyo3(signature = (limit=None))]
    fn limit(mut slf: PyRefMut<Self>, limit: Option<usize>) -> PyResult<PyRefMut<Self>> {
        if let Some(lim) = limit {
            // Validate limit range
            if lim == 0 {
                return Err(PyValueError::new_err("Limit must be greater than 0"));
            }
            if lim > 10000 {
                return Err(PyValueError::new_err("Limit should not exceed 10,000"));
            }
            slf.inner = slf.inner.clone().limit(lim);
        }
        // None is treated as "unset" - uses default of 20 during execution
        Ok(slf)
    }

    /// Build the final PubMed query string
    ///
    /// Terms are joined with space separators (PubMed's default OR logic).
    ///
    /// Returns:
    ///     str: Query string for PubMed E-utilities API
    ///
    /// Raises:
    ///     ValueError: If no search terms have been added
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("covid-19").query("treatment")
    ///     >>> query.build()
    ///     'covid-19 treatment'
    fn build(&self) -> PyResult<String> {
        // Build the query string
        let query_string = self.inner.build();

        // Check if query is empty (no terms added)
        if query_string.trim().is_empty() {
            return Err(PyValueError::new_err(
                "Cannot build query: no search terms provided",
            ));
        }

        Ok(query_string)
    }

    /// String representation for debugging
    fn __repr__(&self) -> String {
        "SearchQuery()".to_string()
    }
}

// Temporary test class to debug module registration
#[pyclass(name = "TestQuery")]
pub struct PyTestQuery {}

#[pymethods]
impl PyTestQuery {
    #[new]
    fn new() -> Self {
        PyTestQuery {}
    }
}
