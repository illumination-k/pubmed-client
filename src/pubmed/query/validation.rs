//! Query validation and optimization methods

use super::SearchQuery;
use crate::error::Result;

impl SearchQuery {
    /// Validate the query structure and parameters
    ///
    /// # Returns
    ///
    /// Returns an error if the query is invalid, Ok(()) otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new().query("covid-19");
    /// assert!(query.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        // Check if query is completely empty
        if self.terms.is_empty() && self.filters.is_empty() {
            return Err(crate::error::PubMedError::InvalidQuery(
                "Query cannot be empty".to_string(),
            ));
        }

        // Validate limit is reasonable
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err(crate::error::PubMedError::InvalidQuery(
                    "Limit must be greater than 0".to_string(),
                ));
            }
            if limit > 10000 {
                return Err(crate::error::PubMedError::InvalidQuery(
                    "Limit should not exceed 10,000 for performance reasons".to_string(),
                ));
            }
        }

        // Check for potentially problematic patterns
        let query_string = self.build();
        if query_string.len() > 4000 {
            return Err(crate::error::PubMedError::InvalidQuery(
                "Query string is too long (>4000 characters)".to_string(),
            ));
        }

        // Check for unbalanced parentheses
        let open_parens = query_string.matches('(').count();
        let close_parens = query_string.matches(')').count();
        if open_parens != close_parens {
            return Err(crate::error::PubMedError::InvalidQuery(
                "Unbalanced parentheses in query".to_string(),
            ));
        }

        Ok(())
    }

    /// Optimize the query for better performance
    ///
    /// # Returns
    ///
    /// Returns an optimized version of the query
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let optimized = SearchQuery::new()
    ///     .query("covid-19")
    ///     .published_after(2020)
    ///     .optimize();
    /// ```
    pub fn optimize(mut self) -> Self {
        // Remove duplicate filters
        self.filters.sort();
        self.filters.dedup();

        // Remove duplicate terms
        self.terms.sort();
        self.terms.dedup();

        // Remove empty terms and filters
        self.terms.retain(|term| !term.trim().is_empty());
        self.filters.retain(|filter| !filter.trim().is_empty());

        self
    }

    /// Get query statistics and information
    ///
    /// # Returns
    ///
    /// Returns a tuple of (term_count, filter_count, estimated_complexity)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .published_after(2020)
    ///     .free_full_text();
    ///
    /// let (terms, filters, complexity) = query.get_stats();
    /// ```
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let term_count = self.terms.len();
        let filter_count = self.filters.len();

        // Estimate complexity based on query structure
        let query_string = self.build();
        let complexity = query_string.matches(" AND ").count()
            + query_string.matches(" OR ").count() * 2
            + query_string.matches(" NOT ").count() * 2
            + query_string.matches('(').count()
            + 1; // Base complexity

        (term_count, filter_count, complexity)
    }
}
