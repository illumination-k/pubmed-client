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
            let combined_query = format!("({}) AND ({})", self_query, other_query);
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
            let combined_query = format!("({}) OR ({})", self_query, other_query);
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
            let negated_query = format!("NOT ({})", self_query);
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
            let combined_query = format!("({}) NOT ({})", self_query, excluded_query);
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
            let grouped_query = format!("({})", self_query);
            self.terms = vec![grouped_query];
            self.filters = Vec::new();
        }

        self
    }
}
