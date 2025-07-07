//! Query validation and optimization methods

use super::SearchQuery;
use crate::error::{PubMedError, Result};

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
            return Err(PubMedError::InvalidQuery(
                "Query cannot be empty".to_string(),
            ));
        }

        // Validate limit is reasonable
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err(PubMedError::InvalidQuery(
                    "Limit must be greater than 0".to_string(),
                ));
            }
            if limit > 10000 {
                return Err(PubMedError::InvalidQuery(
                    "Limit should not exceed 10,000 for performance reasons".to_string(),
                ));
            }
        }

        // Check for potentially problematic patterns
        let query_string = self.build();
        if query_string.len() > 4000 {
            return Err(PubMedError::InvalidQuery(
                "Query string is too long (>4000 characters)".to_string(),
            ));
        }

        // Check for unbalanced parentheses
        let open_parens = query_string.matches('(').count();
        let close_parens = query_string.matches(')').count();
        if open_parens != close_parens {
            return Err(PubMedError::InvalidQuery(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_query() {
        let query = SearchQuery::new();
        assert!(query.validate().is_err());

        if let Err(e) = query.validate() {
            assert!(e.to_string().contains("Query cannot be empty"));
        }
    }

    #[test]
    fn test_validate_valid_query() {
        let query = SearchQuery::new().query("covid-19");
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_limit() {
        let query = SearchQuery::new().query("test").limit(0);
        assert!(query.validate().is_err());

        if let Err(e) = query.validate() {
            assert!(e.to_string().contains("Limit must be greater than 0"));
        }
    }

    #[test]
    fn test_validate_excessive_limit() {
        let query = SearchQuery::new().query("test").limit(20000);
        assert!(query.validate().is_err());

        if let Err(e) = query.validate() {
            assert!(e.to_string().contains("Limit should not exceed 10,000"));
        }
    }

    #[test]
    fn test_validate_reasonable_limit() {
        let query = SearchQuery::new().query("test").limit(100);
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validate_max_reasonable_limit() {
        let query = SearchQuery::new().query("test").limit(10000);
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validate_very_long_query() {
        let long_term = "a".repeat(4001);
        let query = SearchQuery::new().query(long_term);
        assert!(query.validate().is_err());

        if let Err(e) = query.validate() {
            assert!(e.to_string().contains("Query string is too long"));
        }
    }

    #[test]
    fn test_validate_unbalanced_parentheses() {
        let query1 = SearchQuery::new()
            .query("covid")
            .and(SearchQuery::new().query("vaccine"));
        // Manually create unbalanced parentheses by manipulating internal state
        let mut broken_query = query1.clone();
        broken_query.terms = vec!["((test".to_string()];

        assert!(broken_query.validate().is_err());
        if let Err(e) = broken_query.validate() {
            assert!(e.to_string().contains("Unbalanced parentheses"));
        }
    }

    #[test]
    fn test_validate_balanced_parentheses() {
        let query = SearchQuery::new()
            .query("covid")
            .and(SearchQuery::new().query("vaccine"))
            .group();
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_optimize_removes_duplicates() {
        let mut query = SearchQuery::new();
        query.terms = vec![
            "covid".to_string(),
            "vaccine".to_string(),
            "covid".to_string(),
        ];
        query.filters = vec![
            "test[filter]".to_string(),
            "other[filter]".to_string(),
            "test[filter]".to_string(),
        ];

        let optimized = query.optimize();
        assert_eq!(optimized.terms.len(), 2);
        assert_eq!(optimized.filters.len(), 2);
        assert!(optimized.terms.contains(&"covid".to_string()));
        assert!(optimized.terms.contains(&"vaccine".to_string()));
    }

    #[test]
    fn test_optimize_removes_empty_strings() {
        let mut query = SearchQuery::new();
        query.terms = vec![
            "covid".to_string(),
            "  ".to_string(),
            "vaccine".to_string(),
            "".to_string(),
        ];
        query.filters = vec![
            "test[filter]".to_string(),
            "   ".to_string(),
            "other[filter]".to_string(),
        ];

        let optimized = query.optimize();
        assert_eq!(optimized.terms.len(), 2);
        assert_eq!(optimized.filters.len(), 2);
        assert!(optimized.terms.contains(&"covid".to_string()));
        assert!(optimized.terms.contains(&"vaccine".to_string()));
    }

    #[test]
    fn test_optimize_sorts_terms_and_filters() {
        let mut query = SearchQuery::new();
        query.terms = vec![
            "zebra".to_string(),
            "apple".to_string(),
            "banana".to_string(),
        ];
        query.filters = vec![
            "z[filter]".to_string(),
            "a[filter]".to_string(),
            "b[filter]".to_string(),
        ];

        let optimized = query.optimize();
        assert_eq!(
            optimized.terms,
            vec![
                "apple".to_string(),
                "banana".to_string(),
                "zebra".to_string()
            ]
        );
        assert_eq!(
            optimized.filters,
            vec![
                "a[filter]".to_string(),
                "b[filter]".to_string(),
                "z[filter]".to_string()
            ]
        );
    }

    #[test]
    fn test_get_stats_basic() {
        let query = SearchQuery::new()
            .query("covid")
            .query("vaccine")
            .mesh_term("Neoplasms")
            .author("Smith");

        let (term_count, filter_count, complexity) = query.get_stats();
        assert_eq!(term_count, 2); // covid, vaccine
        assert_eq!(filter_count, 2); // mesh term, author
        assert!(complexity > 0);
    }

    #[test]
    fn test_get_stats_empty_query() {
        let query = SearchQuery::new();
        let (term_count, filter_count, complexity) = query.get_stats();
        assert_eq!(term_count, 0);
        assert_eq!(filter_count, 0);
        assert_eq!(complexity, 1); // Base complexity
    }

    #[test]
    fn test_get_stats_complex_query() {
        let query1 = SearchQuery::new().query("covid");
        let query2 = SearchQuery::new().query("vaccine");
        let complex_query = query1.and(query2).or(SearchQuery::new().query("treatment"));

        let (_term_count, _filter_count, complexity) = complex_query.get_stats();
        assert!(complexity > 3); // Should be higher due to boolean operations
    }

    #[test]
    fn test_validate_with_filters_only() {
        let query = SearchQuery::new().mesh_term("Neoplasms");
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validate_with_terms_only() {
        let query = SearchQuery::new().query("covid");
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_optimize_preserves_limit() {
        let query = SearchQuery::new().query("test").limit(100);

        let optimized = query.optimize();
        assert_eq!(optimized.get_limit(), 100);
    }

    #[test]
    fn test_complexity_calculation() {
        // Test AND operation
        let and_query = SearchQuery::new()
            .query("a")
            .and(SearchQuery::new().query("b"));
        let (_, _, and_complexity) = and_query.get_stats();

        // Test OR operation
        let or_query = SearchQuery::new()
            .query("a")
            .or(SearchQuery::new().query("b"));
        let (_, _, or_complexity) = or_query.get_stats();

        // OR should have higher complexity than AND
        assert!(or_complexity >= and_complexity);
    }

    #[test]
    fn test_stats_with_nested_queries() {
        let nested = SearchQuery::new()
            .query("a")
            .and(SearchQuery::new().query("b"))
            .group();

        let (_term_count, _filter_count, complexity) = nested.get_stats();
        assert!(complexity > 2); // Should account for grouping and AND
    }
}
