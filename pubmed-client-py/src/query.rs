//! Query builder module for Python bindings
//!
//! This module provides Python wrappers for the SearchQuery builder.

use pubmed_client::pubmed::ArticleType;
use pubmed_client::pubmed::SearchQuery;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};

// ================================================================================================
// Helper Functions
// ================================================================================================

/// Validate year is within reasonable range for biomedical publications
///
/// # Arguments
/// * `year` - Year to validate (should be 1800-3000)
///
/// # Errors
/// Returns `PyValueError` if year is outside the valid range
fn validate_year(year: u32) -> PyResult<()> {
    if !(1800..=3000).contains(&year) {
        return Err(PyValueError::new_err(format!(
            "Year must be between 1800 and 3000, got: {}",
            year
        )));
    }
    Ok(())
}

/// Convert string to ArticleType enum with case-insensitive matching
///
/// # Arguments
/// * `s` - Article type string (e.g., "Clinical Trial", "review", "RCT")
///
/// # Errors
/// Returns `PyValueError` if the article type is not recognized
fn str_to_article_type(s: &str) -> PyResult<ArticleType> {
    let normalized = s.trim().to_lowercase();

    match normalized.as_str() {
        "clinical trial" => Ok(ArticleType::ClinicalTrial),
        "review" => Ok(ArticleType::Review),
        "systematic review" => Ok(ArticleType::SystematicReview),
        "meta-analysis" | "meta analysis" => Ok(ArticleType::MetaAnalysis),
        "case reports" | "case report" => Ok(ArticleType::CaseReport),
        "randomized controlled trial" | "rct" => Ok(ArticleType::RandomizedControlledTrial),
        "observational study" => Ok(ArticleType::ObservationalStudy),
        _ => Err(PyValueError::new_err(format!(
            "Invalid article type: '{}'. Supported types: Clinical Trial, Review, Systematic Review, Meta-Analysis, Case Reports, Randomized Controlled Trial, Observational Study",
            s
        ))),
    }
}

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
#[gen_stub_pyclass]
#[pyclass(name = "SearchQuery")]
#[derive(Clone)]
pub struct PySearchQuery {
    pub inner: SearchQuery,
}

#[gen_stub_pymethods]
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
            for t in term_list.into_iter().flatten() {
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    slf.inner = slf.inner.clone().query(trimmed);
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

    /// Get the limit for this query
    ///
    /// Returns the configured limit or the default of 20 if not set.
    ///
    /// Returns:
    ///     int: Maximum number of results (default: 20)
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("cancer").limit(100)
    ///     >>> query.get_limit()
    ///     100
    ///     >>> query2 = SearchQuery().query("diabetes")
    ///     >>> query2.get_limit()
    ///     20
    fn get_limit(&self) -> usize {
        self.inner.get_limit()
    }

    /// String representation for debugging
    fn __repr__(&self) -> String {
        "SearchQuery()".to_string()
    }

    // ============================================================================================
    // Date Filtering Methods (User Story 1)
    // ============================================================================================

    /// Filter to articles published in a specific year
    ///
    /// Args:
    ///     year: Year to filter by (must be between 1800 and 3000)
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If year is outside the valid range (1800-3000)
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("covid-19").published_in_year(2024)
    ///     >>> query.build()
    ///     'covid-19 AND 2024[pdat]'
    fn published_in_year(mut slf: PyRefMut<Self>, year: u32) -> PyResult<PyRefMut<Self>> {
        validate_year(year)?;
        slf.inner = slf.inner.clone().published_in_year(year);
        Ok(slf)
    }

    /// Filter by publication date range
    ///
    /// Filters articles published between start_year and end_year (inclusive).
    /// If end_year is None, filters from start_year onwards (up to year 3000).
    ///
    /// Args:
    ///     start_year: Start year (inclusive, must be 1800-3000)
    ///     end_year: End year (inclusive, optional, must be 1800-3000 if provided)
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If years are outside valid range or start_year > end_year
    ///
    /// Example:
    ///     >>> # Filter to 2020-2024
    ///     >>> query = SearchQuery().query("cancer").published_between(2020, 2024)
    ///     >>> query.build()
    ///     'cancer AND 2020:2024[pdat]'
    ///
    ///     >>> # Filter from 2020 onwards
    ///     >>> query = SearchQuery().query("treatment").published_between(2020, None)
    ///     >>> query.build()
    ///     'treatment AND 2020:3000[pdat]'
    #[pyo3(signature = (start_year, end_year=None))]
    fn published_between(
        mut slf: PyRefMut<Self>,
        start_year: u32,
        end_year: Option<u32>,
    ) -> PyResult<PyRefMut<Self>> {
        validate_year(start_year)?;

        // Validate end_year if provided
        if let Some(end) = end_year {
            validate_year(end)?;
            if start_year > end {
                return Err(PyValueError::new_err(format!(
                    "Start year ({}) must be <= end year ({})",
                    start_year, end
                )));
            }
        }

        slf.inner = slf.inner.clone().published_between(start_year, end_year);
        Ok(slf)
    }

    /// Filter to articles published after a specific year
    ///
    /// Equivalent to published_between(year, None).
    ///
    /// Args:
    ///     year: Year after which articles were published (must be 1800-3000)
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If year is outside the valid range (1800-3000)
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("crispr").published_after(2020)
    ///     >>> query.build()
    ///     'crispr AND 2020:3000[pdat]'
    fn published_after(mut slf: PyRefMut<Self>, year: u32) -> PyResult<PyRefMut<Self>> {
        validate_year(year)?;
        slf.inner = slf.inner.clone().published_after(year);
        Ok(slf)
    }

    /// Filter to articles published before a specific year
    ///
    /// Filters articles from 1900 up to and including the specified year.
    ///
    /// Args:
    ///     year: Year before which articles were published (must be 1800-3000)
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If year is outside the valid range (1800-3000)
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("genome").published_before(2020)
    ///     >>> query.build()
    ///     'genome AND 1900:2020[pdat]'
    fn published_before(mut slf: PyRefMut<Self>, year: u32) -> PyResult<PyRefMut<Self>> {
        validate_year(year)?;
        slf.inner = slf.inner.clone().published_before(year);
        Ok(slf)
    }

    // ============================================================================================
    // Article Type Filtering Methods (User Story 2)
    // ============================================================================================

    /// Filter by a single article type
    ///
    /// Args:
    ///     type_name: Article type name (case-insensitive)
    ///         Supported types: "Clinical Trial", "Review", "Systematic Review",
    ///         "Meta-Analysis", "Case Reports", "Randomized Controlled Trial" (or "RCT"),
    ///         "Observational Study"
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If article type is not recognized
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("cancer").article_type("Clinical Trial")
    ///     >>> query.build()
    ///     'cancer AND Clinical Trial[pt]'
    fn article_type(mut slf: PyRefMut<Self>, type_name: String) -> PyResult<PyRefMut<Self>> {
        let article_type = str_to_article_type(&type_name)?;
        slf.inner = slf.inner.clone().article_type(article_type);
        Ok(slf)
    }

    /// Filter by multiple article types (OR logic)
    ///
    /// When multiple types are provided, they are combined with OR logic.
    /// Empty list is silently ignored (no filter added).
    ///
    /// Args:
    ///     types: List of article type names (case-insensitive)
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If any article type is not recognized
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("treatment").article_types(["RCT", "Meta-Analysis"])
    ///     >>> query.build()
    ///     'treatment AND (Randomized Controlled Trial[pt] OR Meta-Analysis[pt])'
    fn article_types(mut slf: PyRefMut<Self>, types: Vec<String>) -> PyResult<PyRefMut<Self>> {
        if types.is_empty() {
            // Empty list is silently ignored
            return Ok(slf);
        }

        // Convert all string types to ArticleType enums
        let article_types: Result<Vec<ArticleType>, PyErr> =
            types.iter().map(|s| str_to_article_type(s)).collect();

        let article_types = article_types?;

        // Call the Rust method with the converted types
        slf.inner = slf.inner.clone().article_types(&article_types);
        Ok(slf)
    }

    // ============================================================================================
    // Open Access Filtering Methods (User Story 3)
    // ============================================================================================

    /// Filter to articles with free full text (open access)
    ///
    /// This includes articles that are freely available from PubMed Central
    /// and other open access sources.
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("cancer").free_full_text_only()
    ///     >>> query.build()
    ///     'cancer AND free full text[sb]'
    fn free_full_text_only(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().free_full_text_only();
        slf
    }

    /// Filter to articles with full text links
    ///
    /// This includes both free full text and subscription-based full text articles.
    /// Use free_full_text_only() if you only want open access articles.
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("diabetes").full_text_only()
    ///     >>> query.build()
    ///     'diabetes AND full text[sb]'
    fn full_text_only(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().full_text_only();
        slf
    }

    /// Filter to articles with PMC full text
    ///
    /// This filters to articles that have full text available in PubMed Central (PMC).
    ///
    /// Returns:
    ///     SearchQuery: Self for method chaining
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("genomics").pmc_only()
    ///     >>> query.build()
    ///     'genomics AND pmc[sb]'
    fn pmc_only(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().pmc_only();
        slf
    }

    // ============================================================================================
    // Boolean Logic Methods (User Story 4 - AND, OR, NOT operations)
    // ============================================================================================

    /// Combine this query with another using AND logic
    ///
    /// Combines two queries by wrapping each in parentheses and joining with AND.
    /// If either query is empty, returns the non-empty query.
    /// The result uses the higher limit of the two queries.
    ///
    /// Args:
    ///     other: Another SearchQuery to combine with
    ///
    /// Returns:
    ///     SearchQuery: New query with combined logic
    ///
    /// Example:
    ///     >>> q1 = SearchQuery().query("covid-19")
    ///     >>> q2 = SearchQuery().query("vaccine")
    ///     >>> combined = q1.and_(q2)
    ///     >>> combined.build()
    ///     '(covid-19) AND (vaccine)'
    ///
    ///     >>> # Complex chaining
    ///     >>> result = SearchQuery().query("cancer") \\
    ///     ...     .and_(SearchQuery().query("treatment")) \\
    ///     ...     .and_(SearchQuery().query("2024[pdat]"))
    ///     >>> result.build()
    ///     '((cancer) AND (treatment)) AND (2024[pdat])'
    #[pyo3(name = "and_")]
    fn py_and(slf: PyRefMut<Self>, other: PyRef<Self>) -> Self {
        let combined_inner = slf.inner.clone().and(other.inner.clone());
        PySearchQuery {
            inner: combined_inner,
        }
    }

    /// Combine this query with another using OR logic
    ///
    /// Combines two queries by wrapping each in parentheses and joining with OR.
    /// If either query is empty, returns the non-empty query.
    /// The result uses the higher limit of the two queries.
    ///
    /// Args:
    ///     other: Another SearchQuery to combine with
    ///
    /// Returns:
    ///     SearchQuery: New query with combined logic
    ///
    /// Example:
    ///     >>> q1 = SearchQuery().query("diabetes")
    ///     >>> q2 = SearchQuery().query("hypertension")
    ///     >>> combined = q1.or_(q2)
    ///     >>> combined.build()
    ///     '(diabetes) OR (hypertension)'
    ///
    ///     >>> # Find articles about either condition
    ///     >>> result = SearchQuery().query("cancer") \\
    ///     ...     .or_(SearchQuery().query("tumor")) \\
    ///     ...     .or_(SearchQuery().query("oncology"))
    ///     >>> result.build()
    ///     '((cancer) OR (tumor)) OR (oncology)'
    #[pyo3(name = "or_")]
    fn py_or(slf: PyRefMut<Self>, other: PyRef<Self>) -> Self {
        let combined_inner = slf.inner.clone().or(other.inner.clone());
        PySearchQuery {
            inner: combined_inner,
        }
    }

    /// Negate this query using NOT logic
    ///
    /// Wraps the current query with NOT operator.
    /// This is typically used in combination with other queries to exclude results.
    /// Returns an empty query if the current query is empty.
    ///
    /// Returns:
    ///     SearchQuery: New query with NOT logic
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("cancer").negate()
    ///     >>> query.build()
    ///     'NOT (cancer)'
    ///
    ///     >>> # More practical: exclude from search results
    ///     >>> base = SearchQuery().query("treatment")
    ///     >>> excluded = SearchQuery().query("animal studies").negate()
    ///     >>> # (Note: use exclude() method for this pattern)
    fn negate(slf: PyRefMut<Self>) -> Self {
        let negated_inner = slf.inner.clone().negate();
        PySearchQuery {
            inner: negated_inner,
        }
    }

    /// Exclude articles matching the given query
    ///
    /// Excludes results from this query that match the excluded query.
    /// This is the recommended way to filter out unwanted results.
    /// If either query is empty, returns the base query unchanged.
    ///
    /// Args:
    ///     excluded: SearchQuery representing articles to exclude
    ///
    /// Returns:
    ///     SearchQuery: New query with exclusion logic
    ///
    /// Example:
    ///     >>> base = SearchQuery().query("cancer treatment")
    ///     >>> exclude = SearchQuery().query("animal studies")
    ///     >>> filtered = base.exclude(exclude)
    ///     >>> filtered.build()
    ///     '(cancer treatment) NOT (animal studies)'
    ///
    ///     >>> # Exclude multiple types of studies
    ///     >>> human_only = SearchQuery().query("therapy") \\
    ///     ...     .exclude(SearchQuery().query("animal studies")) \\
    ///     ...     .exclude(SearchQuery().query("in vitro"))
    fn exclude(slf: PyRefMut<Self>, excluded: PyRef<Self>) -> Self {
        let filtered_inner = slf.inner.clone().exclude(excluded.inner.clone());
        PySearchQuery {
            inner: filtered_inner,
        }
    }

    /// Add parentheses around the current query for grouping
    ///
    /// Wraps the query in parentheses to control operator precedence in complex queries.
    /// Returns an empty query if the current query is empty.
    ///
    /// Returns:
    ///     SearchQuery: New query wrapped in parentheses
    ///
    /// Example:
    ///     >>> query = SearchQuery().query("cancer").or_(SearchQuery().query("tumor")).group()
    ///     >>> query.build()
    ///     '((cancer) OR (tumor))'
    ///
    ///     >>> # Controlling precedence
    ///     >>> q1 = SearchQuery().query("a").or_(SearchQuery().query("b")).group()
    ///     >>> q2 = SearchQuery().query("c").or_(SearchQuery().query("d")).group()
    ///     >>> result = q1.and_(q2)
    ///     >>> result.build()
    ///     '(((a) OR (b))) AND (((c) OR (d)))'
    fn group(slf: PyRefMut<Self>) -> Self {
        let grouped_inner = slf.inner.clone().group();
        PySearchQuery {
            inner: grouped_inner,
        }
    }
}
