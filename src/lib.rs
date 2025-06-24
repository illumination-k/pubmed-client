//! # PubMed Client
//!
//! A Rust client library for accessing PubMed and PMC (PubMed Central) APIs.
//! This crate provides easy-to-use interfaces for searching, fetching, and parsing
//! biomedical research articles.
//!
//! ## Features
//!
//! - **PubMed API Integration**: Search and fetch article metadata
//! - **PMC Full Text**: Retrieve and parse structured full-text articles
//! - **Async Support**: Built on tokio for async/await support
//! - **Error Handling**: Comprehensive error types for robust error handling
//! - **Type Safety**: Strongly typed data structures for all API responses
//!
//! ## Quick Start
//!
//! ### Searching for Articles
//!
//! ```no_run
//! use pubmed_client::PubMedClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = PubMedClient::new();
//!
//!     // Search for articles with query builder
//!     let articles = client
//!         .search()
//!         .query("covid-19 treatment")
//!         .open_access_only()
//!         .published_after(2020)
//!         .limit(10)
//!         .search_and_fetch(&client)
//!         .await?;
//!
//!     for article in articles {
//!         println!("Title: {}", article.title);
//!         println!("Authors: {}", article.authors.join(", "));
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Fetching Full Text from PMC
//!
//! ```no_run
//! use pubmed_client::PmcClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = PmcClient::new();
//!
//!     // Check if PMC full text is available
//!     if let Some(pmcid) = client.check_pmc_availability("33515491").await? {
//!         // Fetch structured full text
//!         let full_text = client.fetch_full_text(&pmcid).await?;
//!
//!         println!("Title: {}", full_text.title);
//!         println!("Sections: {}", full_text.sections.len());
//!         println!("References: {}", full_text.references.len());
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod pmc;
pub mod pubmed;
pub mod query;

// Re-export main types for convenience
pub use error::{PubMedError, Result};
pub use pmc::{ArticleSection, PmcClient, PmcFullText, Reference};
pub use pubmed::{PubMedArticle, PubMedClient};
pub use query::{ArticleType, Language, SearchQuery};

/// Convenience client that combines both PubMed and PMC functionality
#[derive(Clone)]
pub struct Client {
    /// PubMed client for metadata
    pub pubmed: PubMedClient,
    /// PMC client for full text
    pub pmc: PmcClient,
}

impl Client {
    /// Create a new combined client
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::Client;
    ///
    /// let client = Client::new();
    /// ```
    pub fn new() -> Self {
        Self {
            pubmed: PubMedClient::new(),
            pmc: PmcClient::new(),
        }
    }

    /// Create a new combined client with custom HTTP client
    ///
    /// # Arguments
    ///
    /// * `http_client` - Custom reqwest client with specific configuration
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::Client;
    /// use reqwest::ClientBuilder;
    /// use std::time::Duration;
    ///
    /// let http_client = ClientBuilder::new()
    ///     .timeout(Duration::from_secs(30))
    ///     .build()
    ///     .unwrap();
    ///
    /// let client = Client::with_http_client(http_client);
    /// ```
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self {
            pubmed: PubMedClient::with_client(http_client.clone()),
            pmc: PmcClient::with_client(http_client),
        }
    }

    /// Search for articles and attempt to fetch full text for each
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of articles to process
    ///
    /// # Returns
    ///
    /// Returns a vector of tuples containing (PubMedArticle, Option<PmcFullText>)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::Client;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new();
    ///     let results = client.search_with_full_text("covid-19", 5).await?;
    ///
    ///     for (article, full_text) in results {
    ///         println!("Article: {}", article.title);
    ///         if let Some(ft) = full_text {
    ///             println!("  Full text available with {} sections", ft.sections.len());
    ///         } else {
    ///             println!("  Full text not available");
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_with_full_text(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(PubMedArticle, Option<PmcFullText>)>> {
        let articles = self.pubmed.search_and_fetch(query, limit).await?;
        let mut results = Vec::new();

        for article in articles {
            let full_text = match self.pmc.check_pmc_availability(&article.pmid).await? {
                Some(pmcid) => self.pmc.fetch_full_text(&pmcid).await.ok(),
                None => None,
            };
            results.push((article, full_text));
        }

        Ok(results)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
