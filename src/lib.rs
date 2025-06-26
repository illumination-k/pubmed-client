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
//! - **Markdown Export**: Convert PMC articles to well-formatted Markdown
//! - **Async Support**: Built on tokio for async/await support
//! - **Error Handling**: Comprehensive error types for robust error handling
//! - **Type Safety**: Strongly typed data structures for all API responses
//!
//! ## Quick Start
//!
//! ### Searching for Articles
//!
//! ```no_run
//! use pubmed_client_rs::PubMedClient;
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
//! use pubmed_client_rs::PmcClient;
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
//!
//! ### Converting PMC Articles to Markdown
//!
//! ```no_run
//! use pubmed_client_rs::{PmcClient, PmcMarkdownConverter, HeadingStyle, ReferenceStyle};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = PmcClient::new();
//!
//!     // Fetch and parse a PMC article
//!     if let Ok(full_text) = client.fetch_full_text("PMC1234567").await {
//!         // Create a markdown converter with custom configuration
//!         let converter = PmcMarkdownConverter::new()
//!             .with_include_metadata(true)
//!             .with_include_toc(true)
//!             .with_heading_style(HeadingStyle::ATX)
//!             .with_reference_style(ReferenceStyle::Numbered);
//!
//!         // Convert to markdown
//!         let markdown = converter.convert(&full_text);
//!         println!("{}", markdown);
//!
//!         // Or save to file
//!         std::fs::write("article.md", markdown)?;
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod pmc;
pub mod pubmed;
pub mod rate_limit;
pub mod retry;

// Re-export main types for convenience
pub use config::ClientConfig;
pub use error::{PubMedError, Result};
pub use pmc::{
    Affiliation, ArticleSection, Author, Figure, FundingInfo, HeadingStyle, JournalInfo,
    MarkdownConfig, PmcClient, PmcFullText, PmcMarkdownConverter, PmcXmlParser, Reference,
    ReferenceStyle, Table,
};
pub use pubmed::{
    Affiliation as PubMedAffiliation, ArticleType, Author as PubMedAuthor, Citations, DatabaseInfo,
    FieldInfo, Language, LinkInfo, PmcLinks, PubMedArticle, PubMedClient, RelatedArticles,
    SearchQuery,
};
pub use rate_limit::RateLimiter;

/// Convenience client that combines both PubMed and PMC functionality
#[derive(Clone)]
pub struct Client {
    /// PubMed client for metadata
    pub pubmed: PubMedClient,
    /// PMC client for full text
    pub pmc: PmcClient,
}

impl Client {
    /// Create a new combined client with default configuration
    ///
    /// Uses default NCBI rate limiting (3 requests/second) and no API key.
    /// For production use, consider using `with_config()` to set an API key.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::Client;
    ///
    /// let client = Client::new();
    /// ```
    pub fn new() -> Self {
        let config = ClientConfig::new();
        Self::with_config(config)
    }

    /// Create a new combined client with custom configuration
    ///
    /// Both PubMed and PMC clients will use the same configuration
    /// for consistent rate limiting and API key usage.
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration including rate limits, API key, etc.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::{Client, ClientConfig};
    ///
    /// let config = ClientConfig::new()
    ///     .with_api_key("your_api_key_here")
    ///     .with_email("researcher@university.edu");
    ///
    /// let client = Client::with_config(config);
    /// ```
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            pubmed: PubMedClient::with_config(config.clone()),
            pmc: PmcClient::with_config(config),
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
    /// use pubmed_client_rs::Client;
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
    /// Returns a vector of tuples containing (`PubMedArticle`, `Option<PmcFullText>`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::Client;
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

    /// Get list of all available NCBI databases
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing names of all available databases
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::Client;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new();
    ///     let databases = client.get_database_list().await?;
    ///     println!("Available databases: {:?}", databases);
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_database_list(&self) -> Result<Vec<String>> {
        self.pubmed.get_database_list().await
    }

    /// Get detailed information about a specific database
    ///
    /// # Arguments
    ///
    /// * `database` - Name of the database (e.g., "pubmed", "pmc", "books")
    ///
    /// # Returns
    ///
    /// Returns a `Result<DatabaseInfo>` containing detailed database information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::Client;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new();
    ///     let db_info = client.get_database_info("pubmed").await?;
    ///     println!("Database: {}", db_info.name);
    ///     println!("Description: {}", db_info.description);
    ///     println!("Fields: {}", db_info.fields.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_database_info(&self, database: &str) -> Result<DatabaseInfo> {
        self.pubmed.get_database_info(database).await
    }

    /// Get related articles for given PMIDs
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to find related articles for
    ///
    /// # Returns
    ///
    /// Returns a `Result<RelatedArticles>` containing related article information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::Client;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new();
    ///     let related = client.get_related_articles(&[31978945]).await?;
    ///     println!("Found {} related articles", related.related_pmids.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_related_articles(&self, pmids: &[u32]) -> Result<RelatedArticles> {
        self.pubmed.get_related_articles(pmids).await
    }

    /// Get PMC links for given PMIDs (full-text availability)
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to check for PMC availability
    ///
    /// # Returns
    ///
    /// Returns a `Result<PmcLinks>` containing PMC IDs with full text available
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::Client;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new();
    ///     let pmc_links = client.get_pmc_links(&[31978945]).await?;
    ///     println!("Found {} PMC articles", pmc_links.pmc_ids.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_pmc_links(&self, pmids: &[u32]) -> Result<PmcLinks> {
        self.pubmed.get_pmc_links(pmids).await
    }

    /// Get citing articles for given PMIDs
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to find citing articles for
    ///
    /// # Returns
    ///
    /// Returns a `Result<Citations>` containing citing article information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::Client;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::new();
    ///     let citations = client.get_citations(&[31978945]).await?;
    ///     println!("Found {} citing articles", citations.citing_pmids.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_citations(&self, pmids: &[u32]) -> Result<Citations> {
        self.pubmed.get_citations(pmids).await
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
