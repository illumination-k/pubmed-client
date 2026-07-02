use std::time::Duration;

use crate::cache::{PmcCache, create_cache};
use crate::common::PubMedId;
use crate::config::ClientConfig;
use crate::error::Result;
use crate::pmc::extracted::ExtractedFigure;
use crate::pmc::oa_api;
use crate::pmc::oa_api::OaSubsetInfo;
use crate::pmc::parser::parse_pmc_xml;
use crate::rate_limit::RateLimiter;
use crate::request::RequestExecutor;
use pubmed_parser::pmc::PmcArticle;
use reqwest::Client;
use tracing::info;

#[cfg(not(target_arch = "wasm32"))]
use {crate::pmc::cloud::PmcCloudClient, std::path::Path};

use super::common;

/// Client for interacting with PMC (PubMed Central) API
#[derive(Clone)]
pub struct PmcClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
    #[cfg(not(target_arch = "wasm32"))]
    cloud_client: PmcCloudClient,
    cache: Option<PmcCache>,
}

impl PmcClient {
    /// Create a new PMC client with default configuration
    ///
    /// Uses default NCBI rate limiting (3 requests/second) and no API key.
    /// For production use, consider using `with_config()` to set an API key.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::PmcClient;
    ///
    /// let client = PmcClient::new();
    /// ```
    pub fn new() -> Self {
        let config = ClientConfig::new();
        Self::with_config(config)
    }

    pub fn get_pmc_config(&self) -> &ClientConfig {
        &self.config
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_cloud_client_config(&self) -> &ClientConfig {
        &self.cloud_client.config
    }

    /// Create a new PMC client with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration including rate limits, API key, etc.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::{PmcClient, ClientConfig};
    ///
    /// let config = ClientConfig::new()
    ///     .with_api_key("your_api_key_here")
    ///     .with_email("researcher@university.edu");
    ///
    /// let client = PmcClient::with_config(config);
    /// ```
    pub fn with_config(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();
        let base_url = config.effective_base_url().to_string();

        // reqwest's client builder only fails if the TLS backend cannot be
        // initialized — an unrecoverable process-level environment error — so
        // this infallible public constructor is allowed to `expect` here.
        #[allow(clippy::expect_used)]
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

        let cache = config.cache_config.as_ref().map(create_cache);

        Self {
            #[cfg(not(target_arch = "wasm32"))]
            cloud_client: PmcCloudClient::with_shared(
                client.clone(),
                rate_limiter.clone(),
                config.clone(),
            ),
            client,
            base_url,
            rate_limiter,
            cache,
            config,
        }
    }

    /// Create a new PMC client with custom HTTP client and default configuration
    ///
    /// # Arguments
    ///
    /// * `client` - Custom reqwest client with specific configuration
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::PmcClient;
    /// use reqwest::Client;
    /// use std::time::Duration;
    ///
    /// let http_client = Client::builder()
    ///     .timeout(Duration::from_secs(30))
    ///     .build()
    ///     .unwrap();
    ///
    /// let client = PmcClient::with_client(http_client);
    /// ```
    pub fn with_client(client: Client) -> Self {
        let config = ClientConfig::new();
        let rate_limiter = config.create_rate_limiter();
        let base_url = config.effective_base_url().to_string();

        Self {
            #[cfg(not(target_arch = "wasm32"))]
            cloud_client: PmcCloudClient::with_shared(
                client.clone(),
                rate_limiter.clone(),
                config.clone(),
            ),
            client,
            base_url,
            rate_limiter,
            cache: None,
            config,
        }
    }

    /// Set a custom base URL for the PMC API
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for the PMC API
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Fetch full text from PMC using PMCID
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    ///
    /// # Returns
    ///
    /// Returns a `Result<PmcArticle>` containing the structured full text
    ///
    /// # Errors
    ///
    /// * `ParseError::PmcNotAvailable` - If PMC full text is not available
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `ParseError::XmlError` - If XML parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let full_text = client.fetch_full_text("PMC7906746").await?;
    ///     println!("Title: {}", full_text.title().unwrap_or("Untitled"));
    ///     println!("Sections: {}", full_text.sections().len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_full_text(&self, pmcid: &str) -> Result<PmcArticle> {
        let normalized_pmcid = self.normalize_pmcid(pmcid);
        let cache_key = format!("pmc:{}", normalized_pmcid);

        // Check cache first if available
        if let Some(cache) = &self.cache
            && let Some(cached) = cache.get(&cache_key).await
        {
            info!(pmcid = %normalized_pmcid, "Cache hit for PMC full text");
            return Ok(cached);
        }

        // Fetch from API if not cached
        let xml_content = self.fetch_xml(pmcid).await?;
        let full_text = parse_pmc_xml(&xml_content, &normalized_pmcid)?;

        // Store in cache if available
        if let Some(cache) = &self.cache {
            cache.insert(cache_key, full_text.clone()).await;
        }

        Ok(full_text)
    }

    /// Fetch raw XML content from PMC
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    ///
    /// # Returns
    ///
    /// Returns a `Result<String>` containing the raw XML content
    pub async fn fetch_xml(&self, pmcid: &str) -> Result<String> {
        common::fetch_pmc_xml(&self.executor(), &self.base_url, pmcid).await
    }

    /// Check if PMC full text is available for a given PMID
    ///
    /// # Arguments
    ///
    /// * `pmid` - PubMed ID
    ///
    /// # Returns
    ///
    /// Returns `Result<Option<String>>` containing the PMCID if available
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     if let Some(pmcid) = client.check_pmc_availability("33515491").await? {
    ///         println!("PMC available: {}", pmcid);
    ///         let full_text = client.fetch_full_text(&pmcid).await?;
    ///         println!("Title: {}", full_text.title().unwrap_or("Untitled"));
    ///     } else {
    ///         println!("PMC not available");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_pmc_availability(&self, pmid: &str) -> Result<Option<String>> {
        // Validate and parse PMID
        let pmid_obj = PubMedId::parse(pmid)?;
        let pmid_value = pmid_obj.as_u32().to_string();

        let response = self
            .executor()
            .get_endpoint(
                &self.base_url,
                "elink.fcgi",
                &[
                    ("dbfrom", "pubmed"),
                    ("db", "pmc"),
                    ("id", pmid_value.as_str()),
                    ("retmode", "json"),
                ],
            )
            .await?;

        let link_result: serde_json::Value = response.json().await?;

        // Extract PMCID from response
        if let Some(linksets) = link_result["linksets"].as_array() {
            for linkset in linksets {
                if let Some(linksetdbs) = linkset["linksetdbs"].as_array() {
                    for linksetdb in linksetdbs {
                        if linksetdb["dbto"] == "pmc"
                            && let Some(links) = linksetdb["links"].as_array()
                            && let Some(pmcid) = links.first()
                        {
                            return Ok(Some(format!("PMC{pmcid}")));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Check if a PMC article is in the OA (Open Access) subset
    ///
    /// The OA subset contains articles with programmatic access to full-text XML.
    /// Some publishers restrict programmatic access even though the article may be
    /// viewable on the PMC website.
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    ///
    /// # Returns
    ///
    /// Returns `Result<OaSubsetInfo>` containing detailed information about OA availability
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let oa_info = client.is_oa_subset("PMC7906746").await?;
    ///
    ///     if oa_info.is_oa_subset {
    ///         println!("Article is in OA subset");
    ///         if let Some(link) = oa_info.download_link {
    ///             println!("Download: {}", link);
    ///         }
    ///     } else {
    ///         println!("Article is NOT in OA subset");
    ///         if let Some(code) = oa_info.error_code {
    ///             println!("Reason: {}", code);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn is_oa_subset(&self, pmcid: &str) -> Result<OaSubsetInfo> {
        let url = oa_api::build_oa_api_url(pmcid)?;

        let response = self.executor().get(&url).await?;

        let xml_content = response.text().await?;

        // Parse the OA API XML response
        Ok(oa_api::parse_oa_response(&xml_content, pmcid)?)
    }

    /// Download a PMC article's files from the PMC OA Cloud (AWS S3) service
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    /// * `output_dir` - Directory to download the article's files into
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing the list of downloaded file paths
    ///
    /// # Errors
    ///
    /// * `ParseError::InvalidPmid` - If the PMCID format is invalid
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `ParseError::IoError` - If file operations fail
    /// * `ParseError::PmcNotAvailable` - If the article is not available in OA
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let output_dir = Path::new("./extracted_articles");
    ///     let files = client.download_files("PMC7906746", output_dir).await?;
    ///
    ///     for file in files {
    ///         println!("Downloaded: {}", file);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn download_files<P: AsRef<Path>>(
        &self,
        pmcid: &str,
        output_dir: P,
    ) -> Result<Vec<String>> {
        self.cloud_client.download_files(pmcid, output_dir).await
    }

    /// Download the article's files and match figures with their captions from XML
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    /// * `output_dir` - Directory to download the article's files into
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<ExtractedFigure>>` containing figures with both XML metadata and file paths
    ///
    /// # Errors
    ///
    /// * `ParseError::InvalidPmid` - If the PMCID format is invalid
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `ParseError::IoError` - If file operations fail
    /// * `ParseError::PmcNotAvailable` - If the article is not available in OA
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let output_dir = Path::new("./extracted_articles");
    ///     let figures = client.extract_figures_with_captions("PMC7906746", output_dir).await?;
    ///
    ///     for figure in figures {
    ///         println!("Figure {}: {:?}", figure.figure.id, figure.figure.caption);
    ///         println!("File: {}", figure.extracted_file_path);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_figures_with_captions<P: AsRef<Path>>(
        &self,
        pmcid: &str,
        output_dir: P,
    ) -> Result<Vec<ExtractedFigure>> {
        self.cloud_client
            .extract_figures_with_captions(pmcid, output_dir)
            .await
    }

    /// Clear all cached PMC data
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     client.clear_cache().await;
    ///     Ok(())
    /// }
    /// ```
    pub async fn clear_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.clear().await;
            info!("Cleared PMC cache");
        }
    }

    /// Get cache statistics
    ///
    /// Returns the number of items in cache, or 0 if caching is disabled
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::PmcClient;
    ///
    /// let client = PmcClient::new();
    /// let count = client.cache_entry_count();
    /// println!("Cache entries: {}", count);
    /// ```
    pub fn cache_entry_count(&self) -> u64 {
        self.cache.as_ref().map_or(0, |cache| cache.entry_count())
    }

    /// Synchronize cache operations to ensure all pending operations are flushed
    ///
    /// This is useful for testing to ensure cache statistics are accurate
    pub async fn sync_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.sync().await;
        }
    }

    fn normalize_pmcid(&self, pmcid: &str) -> String {
        common::normalize_pmcid(pmcid)
    }

    /// Build a request executor borrowing this client's HTTP client, rate limiter, and config.
    fn executor(&self) -> RequestExecutor<'_> {
        RequestExecutor::new(&self.client, &self.rate_limiter, &self.config)
    }
}

impl Default for PmcClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pmcid() {
        let client = PmcClient::new();

        assert_eq!(client.normalize_pmcid("1234567"), "PMC1234567");
        assert_eq!(client.normalize_pmcid("PMC1234567"), "PMC1234567");
    }

    #[test]
    fn test_client_creation() {
        let client = PmcClient::new();
        assert!(client.base_url.contains("eutils.ncbi.nlm.nih.gov"));
    }

    #[test]
    fn test_custom_base_url() {
        let client = PmcClient::new().with_base_url("https://custom.api.example.com".to_string());
        assert_eq!(client.base_url, "https://custom.api.example.com");
    }
}
