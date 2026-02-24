mod citmatch;
mod egquery;
mod einfo;
mod elink;
mod espell;
mod history;
mod summary;

use std::time::Duration;

use crate::common::PubMedId;
use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pubmed::models::PubMedArticle;
use crate::pubmed::parser::parse_articles_from_xml;
use crate::pubmed::query::SortOrder;
use crate::pubmed::responses::ESearchResult;
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;
use reqwest::{Client, Response};
use tracing::{debug, info, instrument, warn};

/// Client for interacting with PubMed API
#[derive(Clone)]
pub struct PubMedClient {
    client: Client,
    pub(crate) base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
}

impl PubMedClient {
    /// Create a search query builder for this client
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client
    ///         .search()
    ///         .query("covid-19 treatment")
    ///         .free_full_text_only()
    ///         .published_after(2020)
    ///         .limit(10)
    ///         .search_and_fetch(&client)
    ///         .await?;
    ///
    ///     println!("Found {} articles", articles.len());
    ///     Ok(())
    /// }
    /// ```
    pub fn search(&self) -> super::query::SearchQuery {
        super::query::SearchQuery::new()
    }

    /// Create a new PubMed client with default configuration
    ///
    /// Uses default NCBI rate limiting (3 requests/second) and no API key.
    /// For production use, consider using `with_config()` to set an API key.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::PubMedClient;
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
    /// use pubmed_client::{PubMedClient, ClientConfig};
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

        let client = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                Client::builder()
                    .user_agent(config.effective_user_agent())
                    .timeout(Duration::from_secs(config.timeout.as_secs()))
                    .build()
                    .expect("Failed to create HTTP client")
            }

            #[cfg(target_arch = "wasm32")]
            {
                Client::builder()
                    .user_agent(config.effective_user_agent())
                    .build()
                    .expect("Failed to create HTTP client")
            }
        };

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
    /// use pubmed_client::PubMedClient;
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

    /// Get a reference to the client configuration
    pub(crate) fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Get a reference to the rate limiter
    pub(crate) fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }

    /// Get a reference to the HTTP client
    pub(crate) fn http_client(&self) -> &Client {
        &self.client
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
    /// use pubmed_client::PubMedClient;
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
        let mut articles = self.fetch_articles(&[pmid]).await?;

        if articles.len() == 1 {
            Ok(articles.remove(0))
        } else {
            // Try to find by PMID in case batch returned extra/different articles
            let idx = articles.iter().position(|a| a.pmid == pmid);
            match idx {
                Some(i) => Ok(articles.remove(i)),
                None => Err(PubMedError::ArticleNotFound {
                    pmid: pmid.to_string(),
                }),
            }
        }
    }

    /// Search for articles using a query string
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of results to return
    /// * `sort` - Optional sort order for results
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
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let pmids = client.search_articles("covid-19 treatment", 10, None).await?;
    ///     println!("Found {} articles", pmids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self, sort), fields(query = %query, limit = limit))]
    pub async fn search_articles(
        &self,
        query: &str,
        limit: usize,
        sort: Option<&SortOrder>,
    ) -> Result<Vec<String>> {
        // PubMed limits: retstart cannot exceed 9998, and retmax is capped at 9999
        // This means we can only retrieve the first 9,999 results (indices 0-9998)
        const MAX_RETRIEVABLE: usize = 9999;

        if limit > MAX_RETRIEVABLE {
            return Err(PubMedError::SearchLimitExceeded {
                requested: limit,
                maximum: MAX_RETRIEVABLE,
            });
        }

        if query.trim().is_empty() {
            debug!("Empty query provided, returning empty results");
            return Ok(Vec::new());
        }

        let mut url = format!(
            "{}/esearch.fcgi?db=pubmed&term={}&retmax={}&retstart={}&retmode=json",
            self.base_url,
            urlencoding::encode(query),
            limit,
            0
        );

        if let Some(sort_order) = sort {
            url.push_str(&format!("&sort={}", sort_order.as_api_param()));
        }

        debug!("Making initial ESearch API request");
        let response = self.make_request(&url).await?;

        let search_result: ESearchResult = response.json().await?;

        // Check for API error response (NCBI sometimes returns 200 OK with ERROR field)
        if let Some(error_msg) = &search_result.esearchresult.error {
            return Err(PubMedError::ApiError {
                status: 200,
                message: format!("NCBI ESearch API error: {}", error_msg),
            });
        }

        let total_count: usize = search_result
            .esearchresult
            .count
            .as_ref()
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);

        if total_count >= limit {
            warn!(
                "Total results ({}) exceed or equal requested limit ({}). Only the first {} results can be retrieved.",
                total_count, limit, MAX_RETRIEVABLE
            );
        }

        Ok(search_result.esearchresult.idlist)
    }

    /// Fetch multiple articles by PMIDs in a single batch request
    ///
    /// This method sends a single EFetch request with multiple PMIDs (comma-separated),
    /// which is significantly more efficient than fetching articles one by one.
    /// For large numbers of PMIDs, the request is automatically split into batches.
    ///
    /// # Arguments
    ///
    /// * `pmids` - Slice of PubMed IDs as strings
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<PubMedArticle>>` containing articles with metadata.
    /// Articles that fail to parse are skipped (logged via tracing).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client.fetch_articles(&["31978945", "33515491", "25760099"]).await?;
    ///     for article in &articles {
    ///         println!("{}: {}", article.pmid, article.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn fetch_articles(&self, pmids: &[&str]) -> Result<Vec<PubMedArticle>> {
        if pmids.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all PMIDs upfront
        let validated: Vec<u32> = pmids
            .iter()
            .map(|pmid| PubMedId::parse(pmid).map(|p| p.as_u32()))
            .collect::<Result<Vec<_>>>()?;

        // NCBI recommends batches of up to 200 IDs per request
        const BATCH_SIZE: usize = 200;

        let mut all_articles = Vec::with_capacity(pmids.len());

        for chunk in validated.chunks(BATCH_SIZE) {
            let id_list: String = chunk
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let url = format!(
                "{}/efetch.fcgi?db=pubmed&id={}&retmode=xml&rettype=abstract",
                self.base_url, id_list
            );

            debug!(batch_size = chunk.len(), "Making batch EFetch API request");
            let response = self.make_request(&url).await?;
            let xml_text = response.text().await?;

            if xml_text.trim().is_empty() {
                continue;
            }

            let articles = parse_articles_from_xml(&xml_text)?;
            info!(
                requested = chunk.len(),
                parsed = articles.len(),
                "Batch fetch completed"
            );
            all_articles.extend(articles);
        }

        Ok(all_articles)
    }

    /// Search and fetch multiple articles with metadata
    ///
    /// Uses batch fetching internally for efficient retrieval.
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
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client.search_and_fetch("covid-19", 5, None).await?;
    ///     for article in articles {
    ///         println!("{}: {}", article.pmid, article.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_and_fetch(
        &self,
        query: &str,
        limit: usize,
        sort: Option<&SortOrder>,
    ) -> Result<Vec<PubMedArticle>> {
        let pmids = self.search_articles(query, limit, sort).await?;

        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        self.fetch_articles(&pmid_refs).await
    }

    /// Internal helper method for making HTTP requests with retry logic.
    /// Automatically appends API parameters (api_key, email, tool) to the URL.
    pub(crate) async fn make_request(&self, url: &str) -> Result<Response> {
        // Build final URL with API parameters
        let mut final_url = url.to_string();
        let api_params = self.config.build_api_params();

        if !api_params.is_empty() {
            // Check if URL already has query parameters
            let separator = if url.contains('?') { '&' } else { '?' };
            final_url.push(separator);

            // Append API parameters
            let param_strings: Vec<String> = api_params
                .into_iter()
                .map(|(key, value)| format!("{}={}", key, urlencoding::encode(&value)))
                .collect();
            final_url.push_str(&param_strings.join("&"));
        }

        let response = with_retry(
            || async {
                self.rate_limiter.acquire().await?;
                debug!("Making API request to: {}", final_url);
                let response = self
                    .client
                    .get(&final_url)
                    .send()
                    .await
                    .map_err(PubMedError::from)?;

                // Check if response has server error status and convert to retryable error
                if response.status().is_server_error() || response.status().as_u16() == 429 {
                    return Err(PubMedError::ApiError {
                        status: response.status().as_u16(),
                        message: response
                            .status()
                            .canonical_reason()
                            .unwrap_or("Unknown error")
                            .to_string(),
                    });
                }

                Ok(response)
            },
            &self.config.retry_config,
            "NCBI API request",
        )
        .await?;

        // Check for any non-success status (client errors, etc.)
        if !response.status().is_success() {
            warn!("API request failed with status: {}", response.status());
            return Err(PubMedError::ApiError {
                status: response.status().as_u16(),
                message: response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string(),
            });
        }

        Ok(response)
    }
}

impl Default for PubMedClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        mem,
        time::{Duration, Instant},
    };

    use super::*;

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
        assert!(config.effective_user_agent().starts_with("pubmed-client/"));
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
        assert!(mem::size_of_val(&rate_limiter) > 0);
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

    #[tokio::test]
    async fn test_fetch_articles_empty_input() {
        let client = PubMedClient::new();

        let result = client.fetch_articles(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_fetch_articles_invalid_pmid() {
        let client = PubMedClient::new();

        let result = client.fetch_articles(&["not_a_number"]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_articles_validates_all_pmids_before_request() {
        let client = PubMedClient::new();

        // Mix of valid and invalid - should fail on validation before any network request
        let start = Instant::now();
        let result = client
            .fetch_articles(&["31978945", "invalid", "33515491"])
            .await;
        assert!(result.is_err());

        // Should fail quickly (validation only, no network)
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(100));
    }
}
