//! Core SearchQuery builder with basic functionality

use crate::error::Result;
use crate::pubmed::{PubMedArticle, PubMedClient};

/// Builder for constructing PubMed search queries
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub(crate) terms: Vec<String>,
    pub(crate) filters: Vec<String>,
    pub(crate) limit: Option<usize>,
}

impl SearchQuery {
    /// Create a new search query builder
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new();
    /// ```
    pub fn new() -> Self {
        Self {
            terms: Vec::new(),
            filters: Vec::new(),
            limit: None,
        }
    }

    /// Add search terms
    ///
    /// # Arguments
    ///
    /// * `terms` - Search terms to add
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("covid-19 treatment");
    /// ```
    pub fn query<S: Into<String>>(mut self, terms: S) -> Self {
        self.terms.push(terms.into());
        self
    }

    /// Add multiple search terms
    ///
    /// # Arguments
    ///
    /// * `terms` - Multiple search terms
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .terms(&["covid-19", "treatment", "vaccine"]);
    /// ```
    pub fn terms<S: AsRef<str>>(mut self, terms: &[S]) -> Self {
        for term in terms {
            self.terms.push(term.as_ref().to_string());
        }
        self
    }

    /// Set the maximum number of results to return
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .limit(100);
    /// ```
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Build the final query string
    ///
    /// # Returns
    ///
    /// A PubMed query string that can be used with E-utilities
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query_string = SearchQuery::new()
    ///     .query("covid-19")
    ///     .build();
    ///
    /// assert_eq!(query_string, "covid-19");
    /// ```
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        // Add search terms
        if !self.terms.is_empty() {
            parts.push(self.terms.join(" "));
        }

        // Add filters
        parts.extend(self.filters.clone());

        parts.join(" AND ")
    }

    /// Get the limit for this query
    pub fn get_limit(&self) -> usize {
        self.limit.unwrap_or(20)
    }

    /// Execute the search using the provided PubMed client
    ///
    /// # Arguments
    ///
    /// * `client` - PubMed client to use for the search
    ///
    /// # Returns
    ///
    /// Returns a list of PMIDs matching the query
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::{PubMedClient, pubmed::SearchQuery};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let pmids = SearchQuery::new()
    ///         .query("covid-19")
    ///         .limit(10)
    ///         .search(&client)
    ///         .await?;
    ///
    ///     println!("Found {} articles", pmids.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn search(&self, client: &PubMedClient) -> Result<Vec<String>> {
        let query_string = self.build();
        client
            .search_articles(&query_string, self.get_limit())
            .await
    }

    /// Execute the search and fetch full article metadata
    ///
    /// # Arguments
    ///
    /// * `client` - PubMed client to use for the search
    ///
    /// # Returns
    ///
    /// Returns a list of PubMed articles with metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::{PubMedClient, pubmed::SearchQuery};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = SearchQuery::new()
    ///         .query("machine learning medicine")
    ///         .limit(5)
    ///         .search_and_fetch(&client)
    ///         .await?;
    ///
    ///     for article in articles {
    ///         println!("{}: {}", article.pmid, article.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_and_fetch(&self, client: &PubMedClient) -> Result<Vec<PubMedArticle>> {
        let query_string = self.build();
        client
            .search_and_fetch(&query_string, self.get_limit())
            .await
    }
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_query() {
        let query = SearchQuery::new();
        assert_eq!(query.build(), "");
        assert_eq!(query.get_limit(), 20);
    }

    #[test]
    fn test_default_query() {
        let query = SearchQuery::default();
        assert_eq!(query.build(), "");
        assert_eq!(query.get_limit(), 20);
    }

    #[test]
    fn test_single_query_term() {
        let query = SearchQuery::new().query("covid-19");
        assert_eq!(query.build(), "covid-19");
    }

    #[test]
    fn test_multiple_query_calls() {
        let query = SearchQuery::new().query("covid-19").query("treatment");
        assert_eq!(query.build(), "covid-19 treatment");
    }

    #[test]
    fn test_terms_method() {
        let terms = ["covid-19", "vaccine", "efficacy"];
        let query = SearchQuery::new().terms(&terms);
        assert_eq!(query.build(), "covid-19 vaccine efficacy");
    }

    #[test]
    fn test_empty_terms_array() {
        let terms: &[&str] = &[];
        let query = SearchQuery::new().terms(terms);
        assert_eq!(query.build(), "");
    }

    #[test]
    fn test_limit_setting() {
        let query = SearchQuery::new().limit(100);
        assert_eq!(query.get_limit(), 100);
    }

    #[test]
    fn test_limit_with_query() {
        let query = SearchQuery::new().query("cancer").limit(50);
        assert_eq!(query.build(), "cancer");
        assert_eq!(query.get_limit(), 50);
    }

    #[test]
    fn test_string_and_str_inputs() {
        let query1 = SearchQuery::new().query("test");
        let query2 = SearchQuery::new().query("test".to_string());
        assert_eq!(query1.build(), query2.build());
    }

    #[test]
    fn test_empty_query_build() {
        let query = SearchQuery::new();
        assert_eq!(query.build(), "");
    }

    #[test]
    fn test_terms_and_filters_combined() {
        let mut query = SearchQuery::new();
        query.terms.push("cancer".to_string());
        query.filters.push("test[filter]".to_string());
        assert_eq!(query.build(), "cancer AND test[filter]");
    }

    #[test]
    fn test_only_filters_no_terms() {
        let mut query = SearchQuery::new();
        query.filters.push("test1[filter]".to_string());
        query.filters.push("test2[filter]".to_string());
        assert_eq!(query.build(), "test1[filter] AND test2[filter]");
    }

    #[test]
    fn test_limit_edge_values() {
        let query = SearchQuery::new().limit(0);
        assert_eq!(query.get_limit(), 0);

        let query = SearchQuery::new().limit(usize::MAX);
        assert_eq!(query.get_limit(), usize::MAX);
    }

    #[test]
    fn test_chaining_methods() {
        let query = SearchQuery::new()
            .query("test")
            .limit(10)
            .query("more")
            .limit(20); // Should override previous limit

        assert_eq!(query.get_limit(), 20);
        assert_eq!(query.build(), "test more");
    }
}
