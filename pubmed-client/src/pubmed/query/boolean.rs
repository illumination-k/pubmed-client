//! Boolean logic methods for combining and manipulating search queries

use super::SearchQuery;

impl SearchQuery {
    /// Combine this query with another using AND logic
    ///
    /// # Arguments
    ///
    /// * `other` - Another SearchQuery to combine with
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query1 = SearchQuery::new().query("covid-19");
    /// let query2 = SearchQuery::new().query("vaccine");
    /// let combined = query1.and(query2);
    /// ```
    pub fn and(mut self, other: SearchQuery) -> Self {
        // Combine the queries by wrapping each in parentheses
        let self_query = self.build();
        let other_query = other.build();

        if !self_query.is_empty() && !other_query.is_empty() {
            // Create a new query with the combined result
            let combined_query = format!("({self_query}) AND ({other_query})");
            self.terms = vec![combined_query];
            self.filters = Vec::new();
        } else if !other_query.is_empty() {
            self.terms = vec![other_query];
            self.filters = Vec::new();
        }

        // Use the higher limit if set
        if other.limit.is_some() && (self.limit.is_none() || other.limit > self.limit) {
            self.limit = other.limit;
        }

        self
    }

    /// Combine this query with another using OR logic
    ///
    /// # Arguments
    ///
    /// * `other` - Another SearchQuery to combine with
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query1 = SearchQuery::new().query("diabetes");
    /// let query2 = SearchQuery::new().query("hypertension");
    /// let combined = query1.or(query2);
    /// ```
    pub fn or(mut self, other: SearchQuery) -> Self {
        // Combine the queries by wrapping each in parentheses
        let self_query = self.build();
        let other_query = other.build();

        if !self_query.is_empty() && !other_query.is_empty() {
            // Create a new query with the combined result
            let combined_query = format!("({self_query}) OR ({other_query})");
            self.terms = vec![combined_query];
            self.filters = Vec::new();
        } else if !other_query.is_empty() {
            self.terms = vec![other_query];
            self.filters = Vec::new();
        }

        // Use the higher limit if set
        if other.limit.is_some() && (self.limit.is_none() || other.limit > self.limit) {
            self.limit = other.limit;
        }

        self
    }

    /// Negate this query using NOT logic
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .negate();
    /// ```
    pub fn negate(mut self) -> Self {
        let self_query = self.build();

        if !self_query.is_empty() {
            let negated_query = format!("NOT ({self_query})");
            self.terms = vec![negated_query];
            self.filters = Vec::new();
        }

        self
    }

    /// Exclude articles matching the given query
    ///
    /// # Arguments
    ///
    /// * `excluded` - SearchQuery representing articles to exclude
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let base_query = SearchQuery::new().query("cancer treatment");
    /// let exclude_query = SearchQuery::new().query("animal studies");
    /// let filtered = base_query.exclude(exclude_query);
    /// ```
    pub fn exclude(mut self, excluded: SearchQuery) -> Self {
        let self_query = self.build();
        let excluded_query = excluded.build();

        if !self_query.is_empty() && !excluded_query.is_empty() {
            let combined_query = format!("({self_query}) NOT ({excluded_query})");
            self.terms = vec![combined_query];
            self.filters = Vec::new();
        }

        self
    }

    /// Add parentheses around the current query for grouping
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .or(SearchQuery::new().query("tumor"))
    ///     .group();
    /// ```
    pub fn group(mut self) -> Self {
        let self_query = self.build();

        if !self_query.is_empty() {
            let grouped_query = format!("({self_query})");
            self.terms = vec![grouped_query];
            self.filters = Vec::new();
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_and_operation() {
        let query1 = SearchQuery::new().query("covid-19");
        let query2 = SearchQuery::new().query("vaccine");
        let combined = query1.and(query2);
        assert_eq!(combined.build(), "(covid-19) AND (vaccine)");
    }

    #[test]
    fn test_or_operation() {
        let query1 = SearchQuery::new().query("diabetes");
        let query2 = SearchQuery::new().query("hypertension");
        let combined = query1.or(query2);
        assert_eq!(combined.build(), "(diabetes) OR (hypertension)");
    }

    #[test]
    fn test_negate_operation() {
        let query = SearchQuery::new().query("cancer").negate();
        assert_eq!(query.build(), "NOT (cancer)");
    }

    #[test]
    fn test_exclude_operation() {
        let base_query = SearchQuery::new().query("cancer treatment");
        let exclude_query = SearchQuery::new().query("animal studies");
        let filtered = base_query.exclude(exclude_query);
        assert_eq!(filtered.build(), "(cancer treatment) NOT (animal studies)");
    }

    #[test]
    fn test_group_operation() {
        let query = SearchQuery::new().query("cancer").group();
        assert_eq!(query.build(), "(cancer)");
    }

    #[test]
    fn test_complex_boolean_chain() {
        let ai_query = SearchQuery::new().query("machine learning");
        let medicine_query = SearchQuery::new().query("medicine");
        let exclude_query = SearchQuery::new().query("veterinary");

        let final_query = ai_query.and(medicine_query).exclude(exclude_query).group();

        assert_eq!(
            final_query.build(),
            "(((machine learning) AND (medicine)) NOT (veterinary))"
        );
    }

    #[test]
    fn test_and_with_empty_queries() {
        let query1 = SearchQuery::new();
        let query2 = SearchQuery::new().query("test");
        let combined = query1.and(query2);
        assert_eq!(combined.build(), "test");
    }

    #[test]
    fn test_or_with_empty_queries() {
        let query1 = SearchQuery::new().query("test");
        let query2 = SearchQuery::new();
        let combined = query1.or(query2);
        assert_eq!(combined.build(), "test");
    }

    #[test]
    fn test_limit_preservation_in_boolean_ops() {
        let query1 = SearchQuery::new().query("covid").limit(10);
        let query2 = SearchQuery::new().query("vaccine").limit(50);
        let combined = query1.and(query2);
        assert_eq!(combined.get_limit(), 50); // Should use higher limit
    }

    #[test]
    fn test_negate_empty_query() {
        let query = SearchQuery::new().negate();
        assert_eq!(query.build(), "");
    }

    #[test]
    fn test_exclude_empty_base() {
        let base_query = SearchQuery::new();
        let exclude_query = SearchQuery::new().query("test");
        let filtered = base_query.exclude(exclude_query);
        assert_eq!(filtered.build(), "");
    }

    #[test]
    fn test_exclude_empty_excluded() {
        let base_query = SearchQuery::new().query("test");
        let exclude_query = SearchQuery::new();
        let filtered = base_query.exclude(exclude_query);
        assert_eq!(filtered.build(), "test");
    }

    #[test]
    fn test_deep_boolean_nesting() {
        let q1 = SearchQuery::new().query("a");
        let q2 = SearchQuery::new().query("b");
        let q3 = SearchQuery::new().query("c");
        let q4 = SearchQuery::new().query("d");

        let nested = q1.and(q2).or(q3.and(q4));
        assert_eq!(nested.build(), "((a) AND (b)) OR ((c) AND (d))");
    }

    #[test]
    fn test_group_empty_query() {
        let query = SearchQuery::new().group();
        assert_eq!(query.build(), "");
    }
}
