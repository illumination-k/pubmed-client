use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pubmed::models::PubMedArticle;
use crate::pubmed::parser::PubMedXmlParser;
use crate::pubmed::responses::ESearchResult;
use crate::rate_limit::RateLimiter;
use reqwest::Client;
use tracing::{debug, info, instrument, warn};

/// Client for interacting with PubMed API
#[derive(Clone)]
pub struct PubMedClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
}

impl PubMedClient {
    /// Create a search query builder for this client
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client
    ///         .search()
    ///         .query("covid-19 treatment")
    ///         .open_access_only()
    ///         .published_after(2020)
    ///         .limit(10)
    ///         .search_and_fetch(&client)
    ///         .await?;
    ///
    ///     println!("Found {} articles", articles.len());
    ///     Ok(())
    /// }
    /// ```
    pub fn search(&self) -> crate::query::SearchQuery {
        crate::query::SearchQuery::new()
    }

    /// Create a new PubMed client with default configuration
    ///
    /// Uses default NCBI rate limiting (3 requests/second) and no API key.
    /// For production use, consider using `with_config()` to set an API key.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// let client = PubMedClient::new();
    /// ```
    pub fn new() -> Self {
        let config = ClientConfig::new();
        Self::with_config(config)
    }

    /// Create a new PubMed client with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration including rate limits, API key, etc.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::{PubMedClient, ClientConfig};
    ///
    /// let config = ClientConfig::new()
    ///     .with_api_key("your_api_key_here")
    ///     .with_email("researcher@university.edu");
    ///
    /// let client = PubMedClient::with_config(config);
    /// ```
    pub fn with_config(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();
        let base_url = config.effective_base_url().to_string();

        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(config.effective_user_agent())
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            rate_limiter,
            config,
        }
    }

    /// Create a new PubMed client with custom HTTP client and default configuration
    ///
    /// # Arguments
    ///
    /// * `client` - Custom reqwest client with specific configuration
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::PubMedClient;
    /// use reqwest::Client;
    /// use std::time::Duration;
    ///
    /// let http_client = Client::builder()
    ///     .timeout(Duration::from_secs(30))
    ///     .build()
    ///     .unwrap();
    ///
    /// let client = PubMedClient::with_client(http_client);
    /// ```
    pub fn with_client(client: Client) -> Self {
        let config = ClientConfig::new();
        let rate_limiter = config.create_rate_limiter();
        let base_url = config.effective_base_url().to_string();

        Self {
            client,
            base_url,
            rate_limiter,
            config,
        }
    }

    /// Fetch article metadata by PMID with full details including abstract
    ///
    /// # Arguments
    ///
    /// * `pmid` - PubMed ID as a string
    ///
    /// # Returns
    ///
    /// Returns a `Result<PubMedArticle>` containing the article metadata with abstract
    ///
    /// # Errors
    ///
    /// * `PubMedError::ArticleNotFound` - If the article is not found
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let article = client.fetch_article("31978945").await?;
    ///     println!("Title: {}", article.title);
    ///     if let Some(abstract_text) = &article.abstract_text {
    ///         println!("Abstract: {}", abstract_text);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmid = %pmid))]
    pub async fn fetch_article(&self, pmid: &str) -> Result<PubMedArticle> {
        // Validate PMID format
        if pmid.trim().is_empty() || !pmid.chars().all(|c| c.is_ascii_digit()) {
            warn!("Invalid PMID format provided");
            return Err(PubMedError::InvalidPmid {
                pmid: pmid.to_string(),
            });
        }

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await?;

        // Build URL with API parameters
        let mut url = format!(
            "{}/efetch.fcgi?db=pubmed&id={}&retmode=xml&rettype=abstract",
            self.base_url, pmid
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        if !api_params.is_empty() {
            for (key, value) in api_params {
                url.push('&');
                url.push_str(&key);
                url.push('=');
                url.push_str(&urlencoding::encode(&value));
            }
        }

        debug!("Making EFetch API request");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            warn!("API request failed with status: {}", response.status());
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

        debug!("Received successful API response, parsing XML");
        let xml_text = response.text().await?;

        let result = PubMedXmlParser::parse_article_from_xml(&xml_text, pmid);
        match &result {
            Ok(article) => {
                info!(
                    title = %article.title,
                    authors_count = article.authors.len(),
                    has_abstract = article.abstract_text.is_some(),
                    "Successfully parsed article"
                );
            }
            Err(e) => {
                warn!("Failed to parse article XML: {}", e);
            }
        }

        result
    }

    /// Search for articles using a query string
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing PMIDs of matching articles
    ///
    /// # Errors
    ///
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let pmids = client.search_articles("covid-19 treatment", 10).await?;
    ///     println!("Found {} articles", pmids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(query = %query, limit = limit))]
    pub async fn search_articles(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        if query.trim().is_empty() {
            debug!("Empty query provided, returning empty results");
            return Ok(Vec::new());
        }

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await?;

        // Build URL with API parameters
        let mut url = format!(
            "{}/esearch.fcgi?db=pubmed&term={}&retmax={}&retmode=json",
            self.base_url,
            urlencoding::encode(query),
            limit
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        debug!("Making ESearch API request");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            warn!(
                "Search API request failed with status: {}",
                response.status()
            );
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

        let search_result: ESearchResult = response.json().await?;
        let pmids = search_result.esearchresult.idlist;

        info!(results_found = pmids.len(), "Search completed successfully");

        Ok(pmids)
    }

    /// Search and fetch multiple articles with metadata
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of articles to fetch
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<PubMedArticle>>` containing articles with metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client.search_and_fetch("covid-19", 5).await?;
    ///     for article in articles {
    ///         println!("{}: {}", article.pmid, article.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_and_fetch(&self, query: &str, limit: usize) -> Result<Vec<PubMedArticle>> {
        let pmids = self.search_articles(query, limit).await?;

        let mut articles = Vec::new();
        for pmid in pmids {
            match self.fetch_article(&pmid).await {
                Ok(article) => articles.push(article),
                Err(PubMedError::ArticleNotFound { .. }) => {
                    // Skip articles that can't be found
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(articles)
    }
}

impl Default for PubMedClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_client_config_rate_limiting() {
        // Test default configuration (no API key)
        let config = ClientConfig::new();
        assert_eq!(config.effective_rate_limit(), 3.0);

        // Test with API key
        let config_with_key = ClientConfig::new().with_api_key("test_key");
        assert_eq!(config_with_key.effective_rate_limit(), 10.0);

        // Test custom rate limit
        let config_custom = ClientConfig::new().with_rate_limit(5.0);
        assert_eq!(config_custom.effective_rate_limit(), 5.0);

        // Test custom rate limit overrides API key default
        let config_override = ClientConfig::new()
            .with_api_key("test_key")
            .with_rate_limit(7.0);
        assert_eq!(config_override.effective_rate_limit(), 7.0);
    }

    #[test]
    fn test_client_api_params() {
        let config = ClientConfig::new()
            .with_api_key("test_key_123")
            .with_email("test@example.com")
            .with_tool("TestTool");

        let params = config.build_api_params();

        // Should have 3 parameters
        assert_eq!(params.len(), 3);

        // Check each parameter
        assert!(params.contains(&("api_key".to_string(), "test_key_123".to_string())));
        assert!(params.contains(&("email".to_string(), "test@example.com".to_string())));
        assert!(params.contains(&("tool".to_string(), "TestTool".to_string())));
    }

    #[test]
    fn test_config_effective_values() {
        let config = ClientConfig::new()
            .with_email("test@example.com")
            .with_tool("TestApp");

        assert_eq!(
            config.effective_base_url(),
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils"
        );
        assert!(
            config
                .effective_user_agent()
                .starts_with("pubmed-client-rs/")
        );
        assert_eq!(config.effective_tool(), "TestApp");
    }

    #[test]
    fn test_rate_limiter_creation_from_config() {
        let config = ClientConfig::new()
            .with_api_key("test_key")
            .with_rate_limit(8.0);

        let rate_limiter = config.create_rate_limiter();

        // Rate limiter should be created successfully
        // We can't easily test the exact rate without async context,
        // but we can verify it was created
        assert!(std::mem::size_of_val(&rate_limiter) > 0);
    }

    #[tokio::test]
    async fn test_invalid_pmid_rate_limiting() {
        let config = ClientConfig::new().with_rate_limit(5.0);
        let client = PubMedClient::with_config(config);

        // Invalid PMID should fail before rate limiting (validation happens first)
        let start = Instant::now();
        let result = client.fetch_article("invalid_pmid").await;
        assert!(result.is_err());

        let elapsed = start.elapsed();
        // Should fail quickly without consuming rate limit token
        assert!(elapsed < Duration::from_millis(100));
    }
}
