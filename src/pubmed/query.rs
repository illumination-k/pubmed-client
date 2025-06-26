//! Query builder for constructing PubMed search queries with filters

use super::{PubMedArticle, PubMedClient};
use crate::error::Result;

/// Article types that can be filtered in PubMed searches
#[derive(Debug, Clone, PartialEq)]
pub enum ArticleType {
    /// Clinical trials
    ClinicalTrial,
    /// Review articles
    Review,
    /// Systematic reviews
    SystematicReview,
    /// Meta-analysis
    MetaAnalysis,
    /// Case reports
    CaseReport,
    /// Randomized controlled trials
    RandomizedControlledTrial,
    /// Observational studies
    ObservationalStudy,
}

impl ArticleType {
    fn to_query_string(&self) -> &'static str {
        match self {
            ArticleType::ClinicalTrial => "Clinical Trial[pt]",
            ArticleType::Review => "Review[pt]",
            ArticleType::SystematicReview => "Systematic Review[pt]",
            ArticleType::MetaAnalysis => "Meta-Analysis[pt]",
            ArticleType::CaseReport => "Case Reports[pt]",
            ArticleType::RandomizedControlledTrial => "Randomized Controlled Trial[pt]",
            ArticleType::ObservationalStudy => "Observational Study[pt]",
        }
    }
}

/// Language options for filtering articles
#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    English,
    Japanese,
    German,
    French,
    Spanish,
    Italian,
    Chinese,
    Other(String),
}

impl Language {
    fn to_query_string(&self) -> String {
        match self {
            Language::English => "English[lang]".to_string(),
            Language::Japanese => "Japanese[lang]".to_string(),
            Language::German => "German[lang]".to_string(),
            Language::French => "French[lang]".to_string(),
            Language::Spanish => "Spanish[lang]".to_string(),
            Language::Italian => "Italian[lang]".to_string(),
            Language::Chinese => "Chinese[lang]".to_string(),
            Language::Other(lang) => format!("{}[lang]", lang),
        }
    }
}

/// Builder for constructing PubMed search queries
#[derive(Debug, Clone)]
pub struct SearchQuery {
    terms: Vec<String>,
    filters: Vec<String>,
    limit: Option<usize>,
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

    /// Filter to open access articles only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .open_access_only();
    /// ```
    pub fn open_access_only(mut self) -> Self {
        self.filters.push("free full text[sb]".to_string());
        self
    }

    /// Filter to articles with free full text
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("diabetes")
    ///     .free_full_text();
    /// ```
    pub fn free_full_text(mut self) -> Self {
        self.filters.push("free full text[sb]".to_string());
        self
    }

    /// Filter to articles with any full text (including subscription-based)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .has_full_text();
    /// ```
    pub fn has_full_text(mut self) -> Self {
        self.filters.push("full text[sb]".to_string());
        self
    }

    /// Filter to articles with abstracts
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("genetics")
    ///     .has_abstract();
    /// ```
    pub fn has_abstract(mut self) -> Self {
        self.filters.push("hasabstract".to_string());
        self
    }

    /// Filter by article types
    ///
    /// # Arguments
    ///
    /// * `types` - Article types to include
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::{SearchQuery, ArticleType};
    ///
    /// let query = SearchQuery::new()
    ///     .query("hypertension")
    ///     .article_types(&[ArticleType::ClinicalTrial, ArticleType::Review]);
    /// ```
    pub fn article_types(mut self, types: &[ArticleType]) -> Self {
        if !types.is_empty() {
            let type_filters: Vec<String> = types
                .iter()
                .map(|t| t.to_query_string().to_string())
                .collect();

            if type_filters.len() == 1 {
                self.filters.push(type_filters[0].clone());
            } else {
                // Multiple types: (type1[pt] OR type2[pt] OR ...)
                let combined = format!("({})", type_filters.join(" OR "));
                self.filters.push(combined);
            }
        }
        self
    }

    /// Filter by language
    ///
    /// # Arguments
    ///
    /// * `language` - Language to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::{SearchQuery, Language};
    ///
    /// let query = SearchQuery::new()
    ///     .query("stem cells")
    ///     .language(Language::English);
    /// ```
    pub fn language(mut self, language: Language) -> Self {
        self.filters.push(language.to_query_string());
        self
    }

    /// Filter by publication date range
    ///
    /// # Arguments
    ///
    /// * `start_year` - Start year (inclusive)
    /// * `end_year` - End year (inclusive, optional)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("immunotherapy")
    ///     .date_range(2020, Some(2023));
    /// ```
    pub fn date_range(mut self, start_year: u32, end_year: Option<u32>) -> Self {
        let date_filter = match end_year {
            Some(end) => format!("{}:{}[pdat]", start_year, end),
            None => format!("{}:3000[pdat]", start_year), // Far future date
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter to articles published after a specific year
    ///
    /// # Arguments
    ///
    /// * `year` - Year after which articles were published
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("crispr")
    ///     .published_after(2020);
    /// ```
    pub fn published_after(self, year: u32) -> Self {
        self.date_range(year, None)
    }

    /// Filter to articles published before a specific year
    ///
    /// # Arguments
    ///
    /// * `year` - Year before which articles were published
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("genome sequencing")
    ///     .published_before(2020);
    /// ```
    pub fn published_before(mut self, year: u32) -> Self {
        let date_filter = format!("1900:{}[pdat]", year);
        self.filters.push(date_filter);
        self
    }

    /// Set maximum number of results
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results to return
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("alzheimer")
    ///     .limit(50);
    /// ```
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add a custom filter
    ///
    /// # Arguments
    ///
    /// * `filter` - Custom PubMed search filter
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .custom_filter("humans[mh]");
    /// ```
    pub fn custom_filter<S: Into<String>>(mut self, filter: S) -> Self {
        self.filters.push(filter.into());
        self
    }

    /// Filter by MeSH major topic
    ///
    /// # Arguments
    ///
    /// * `mesh_term` - MeSH term to filter by as a major topic
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_major_topic("Diabetes Mellitus, Type 2");
    /// ```
    pub fn mesh_major_topic<S: Into<String>>(mut self, mesh_term: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Major Topic]", mesh_term.into()));
        self
    }

    /// Filter by MeSH term
    ///
    /// # Arguments
    ///
    /// * `mesh_term` - MeSH term to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_term("Neoplasms");
    /// ```
    pub fn mesh_term<S: Into<String>>(mut self, mesh_term: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Terms]", mesh_term.into()));
        self
    }

    /// Filter by multiple MeSH terms
    ///
    /// # Arguments
    ///
    /// * `mesh_terms` - MeSH terms to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_terms(&["Neoplasms", "Antineoplastic Agents"]);
    /// ```
    pub fn mesh_terms<S: AsRef<str>>(mut self, mesh_terms: &[S]) -> Self {
        for term in mesh_terms {
            self = self.mesh_term(term.as_ref());
        }
        self
    }

    /// Filter by MeSH subheading
    ///
    /// # Arguments
    ///
    /// * `subheading` - MeSH subheading to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_term("Diabetes Mellitus")
    ///     .mesh_subheading("drug therapy");
    /// ```
    pub fn mesh_subheading<S: Into<String>>(mut self, subheading: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Subheading]", subheading.into()));
        self
    }

    /// Filter to clinical trials only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("diabetes treatment")
    ///     .clinical_trials_only();
    /// ```
    pub fn clinical_trials_only(mut self) -> Self {
        self.filters.push("Clinical Trial[pt]".to_string());
        self
    }

    /// Filter by first author
    ///
    /// # Arguments
    ///
    /// * `author` - First author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer treatment")
    ///     .first_author("Smith J");
    /// ```
    pub fn first_author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters
            .push(format!("{}[First Author]", author.into()));
        self
    }

    /// Filter by last author
    ///
    /// # Arguments
    ///
    /// * `author` - Last author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("genomics")
    ///     .last_author("Johnson M");
    /// ```
    pub fn last_author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters.push(format!("{}[Last Author]", author.into()));
        self
    }

    /// Filter by any author
    ///
    /// # Arguments
    ///
    /// * `author` - Author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .author("Williams K");
    /// ```
    pub fn author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters.push(format!("{}[Author]", author.into()));
        self
    }

    /// Filter by institution/affiliation
    ///
    /// # Arguments
    ///
    /// * `institution` - Institution name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cardiology research")
    ///     .affiliation("Harvard Medical School");
    /// ```
    pub fn affiliation<S: Into<String>>(mut self, institution: S) -> Self {
        self.filters
            .push(format!("{}[Affiliation]", institution.into()));
        self
    }

    /// Filter by ORCID identifier
    ///
    /// # Arguments
    ///
    /// * `orcid_id` - ORCID identifier to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("computational biology")
    ///     .orcid("0000-0001-2345-6789");
    /// ```
    pub fn orcid<S: Into<String>>(mut self, orcid_id: S) -> Self {
        self.filters
            .push(format!("{}[Author - Identifier]", orcid_id.into()));
        self
    }

    /// Build the final query string
    ///
    /// # Returns
    ///
    /// Returns the constructed PubMed query string
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query_string = SearchQuery::new()
    ///     .query("covid-19")
    ///     .open_access_only()
    ///     .published_after(2020)
    ///     .build();
    ///
    /// assert!(query_string.contains("covid-19"));
    /// assert!(query_string.contains("free full text[sb]"));
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
    ///         .open_access_only()
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
    ///         .free_full_text()
    ///         .published_after(2022)
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
    fn test_basic_query_builder() {
        let query = SearchQuery::new().query("covid-19").build();

        assert_eq!(query, "covid-19");
    }

    #[test]
    fn test_query_with_open_access_filter() {
        let query = SearchQuery::new()
            .query("cancer")
            .open_access_only()
            .build();

        assert_eq!(query, "cancer AND free full text[sb]");
    }

    #[test]
    fn test_complex_query() {
        let query = SearchQuery::new()
            .query("machine learning")
            .free_full_text()
            .published_after(2020)
            .article_types(&[ArticleType::Review, ArticleType::SystematicReview])
            .language(Language::English)
            .build();

        let expected = "machine learning AND free full text[sb] AND 2020:3000[pdat] AND (Review[pt] OR Systematic Review[pt]) AND English[lang]";
        assert_eq!(query, expected);
    }

    #[test]
    fn test_date_range() {
        let query = SearchQuery::new()
            .query("diabetes")
            .date_range(2020, Some(2023))
            .build();

        assert_eq!(query, "diabetes AND 2020:2023[pdat]");
    }

    #[test]
    fn test_multiple_terms() {
        let query = SearchQuery::new()
            .terms(&["cancer", "treatment", "immunotherapy"])
            .build();

        assert_eq!(query, "cancer treatment immunotherapy");
    }

    #[test]
    fn test_custom_filter() {
        let query = SearchQuery::new()
            .query("genetics")
            .custom_filter("humans[mh]")
            .build();

        assert_eq!(query, "genetics AND humans[mh]");
    }

    #[test]
    fn test_empty_query() {
        let query = SearchQuery::new().build();
        assert_eq!(query, "");
    }

    #[test]
    fn test_limit() {
        let query = SearchQuery::new().query("test").limit(50);

        assert_eq!(query.get_limit(), 50);
    }

    #[test]
    fn test_default_limit() {
        let query = SearchQuery::new().query("test");

        assert_eq!(query.get_limit(), 20);
    }

    #[test]
    fn test_mesh_major_topic() {
        let query = SearchQuery::new()
            .mesh_major_topic("Diabetes Mellitus")
            .build();

        assert_eq!(query, "Diabetes Mellitus[MeSH Major Topic]");
    }

    #[test]
    fn test_mesh_term() {
        let query = SearchQuery::new().mesh_term("Hypertension").build();

        assert_eq!(query, "Hypertension[MeSH Terms]");
    }

    #[test]
    fn test_mesh_terms_multiple() {
        let query = SearchQuery::new()
            .mesh_terms(&["Cancer", "Chemotherapy"])
            .build();

        assert_eq!(query, "Cancer[MeSH Terms] AND Chemotherapy[MeSH Terms]");
    }

    #[test]
    fn test_mesh_subheading() {
        let query = SearchQuery::new().mesh_subheading("drug therapy").build();

        assert_eq!(query, "drug therapy[MeSH Subheading]");
    }

    #[test]
    fn test_clinical_trials_only() {
        let query = SearchQuery::new()
            .query("treatment")
            .clinical_trials_only()
            .build();

        assert_eq!(query, "treatment AND Clinical Trial[pt]");
    }

    #[test]
    fn test_mesh_complex_query() {
        let query = SearchQuery::new()
            .mesh_major_topic("COVID-19")
            .mesh_subheading("prevention & control")
            .published_after(2022)
            .free_full_text()
            .build();

        assert_eq!(
            query,
            "COVID-19[MeSH Major Topic] AND prevention & control[MeSH Subheading] AND 2022:3000[pdat] AND free full text[sb]"
        );
    }

    #[test]
    fn test_first_author_filter() {
        let query = SearchQuery::new()
            .query("cancer treatment")
            .first_author("Smith J")
            .build();

        assert_eq!(query, "cancer treatment AND Smith J[First Author]");
    }

    #[test]
    fn test_last_author_filter() {
        let query = SearchQuery::new()
            .query("genomics")
            .last_author("Johnson M")
            .build();

        assert_eq!(query, "genomics AND Johnson M[Last Author]");
    }

    #[test]
    fn test_author_filter() {
        let query = SearchQuery::new()
            .query("machine learning")
            .author("Williams K")
            .build();

        assert_eq!(query, "machine learning AND Williams K[Author]");
    }

    #[test]
    fn test_affiliation_filter() {
        let query = SearchQuery::new()
            .query("cardiology research")
            .affiliation("Harvard Medical School")
            .build();

        assert_eq!(
            query,
            "cardiology research AND Harvard Medical School[Affiliation]"
        );
    }

    #[test]
    fn test_orcid_filter() {
        let query = SearchQuery::new()
            .query("computational biology")
            .orcid("0000-0001-2345-6789")
            .build();

        assert_eq!(
            query,
            "computational biology AND 0000-0001-2345-6789[Author - Identifier]"
        );
    }

    #[test]
    fn test_author_complex_query() {
        let query = SearchQuery::new()
            .query("diabetes treatment")
            .first_author("Smith J")
            .affiliation("Harvard Medical School")
            .published_after(2020)
            .free_full_text()
            .build();

        assert_eq!(
            query,
            "diabetes treatment AND Smith J[First Author] AND Harvard Medical School[Affiliation] AND 2020:3000[pdat] AND free full text[sb]"
        );
    }
}
