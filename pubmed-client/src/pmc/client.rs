use std::time::Duration;

use crate::cache::{create_cache, PmcCache};
use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pmc::models::{ExtractedFigure, PmcFullText};
use crate::pmc::parser::parse_pmc_xml;
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;
use reqwest::{Client, Response};
use tracing::{debug, info};

#[cfg(not(target_arch = "wasm32"))]
use {crate::pmc::tar::PmcTarClient, std::path::Path};

/// Client for interacting with PMC (PubMed Central) API
#[derive(Clone)]
pub struct PmcClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
    #[cfg(not(target_arch = "wasm32"))]
    tar_client: PmcTarClient,
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
    /// use pubmed_client_rs::PmcClient;
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
    pub fn get_tar_client_config(&self) -> &ClientConfig {
        &self.tar_client.config
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
    /// use pubmed_client_rs::{PmcClient, ClientConfig};
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
            client,
            base_url,
            rate_limiter,
            #[cfg(not(target_arch = "wasm32"))]
            tar_client: PmcTarClient::new(config.clone()),
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
    /// use pubmed_client_rs::PmcClient;
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
            client,
            base_url,
            rate_limiter,
            #[cfg(not(target_arch = "wasm32"))]
            tar_client: PmcTarClient::new(config.clone()),
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
    /// Returns a `Result<PmcFullText>` containing the structured full text
    ///
    /// # Errors
    ///
    /// * `PubMedError::PmcNotAvailable` - If PMC full text is not available
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::XmlError` - If XML parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let full_text = client.fetch_full_text("PMC7906746").await?;
    ///     println!("Title: {}", full_text.title);
    ///     println!("Sections: {}", full_text.sections.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_full_text(&self, pmcid: &str) -> Result<PmcFullText> {
        let normalized_pmcid = self.normalize_pmcid(pmcid);
        let cache_key = format!("pmc:{}", normalized_pmcid);

        // Check cache first if available
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(&cache_key).await {
                info!(pmcid = %normalized_pmcid, "Cache hit for PMC full text");
                return Ok(cached);
            }
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
        // Remove PMC prefix if present and validate
        let clean_pmcid = pmcid.trim_start_matches("PMC");
        if clean_pmcid.is_empty() || !clean_pmcid.chars().all(|c| c.is_ascii_digit()) {
            return Err(PubMedError::InvalidPmid {
                pmid: pmcid.to_string(),
            });
        }

        // Build URL with API parameters
        let mut url = format!(
            "{}/efetch.fcgi?db=pmc&id=PMC{clean_pmcid}&retmode=xml",
            self.base_url
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        let response = self.make_request(&url).await?;

        if !response.status().is_success() {
            return Err(PubMedError::ApiError {
                status: response.status().as_u16(),
                message: response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string(),
            });
        }

        let xml_content = response.text().await?;

        // Check if the response contains an error
        if xml_content.contains("<ERROR>") {
            return Err(PubMedError::PmcNotAvailableById {
                pmcid: pmcid.to_string(),
            });
        }

        Ok(xml_content)
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
    /// use pubmed_client_rs::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     if let Some(pmcid) = client.check_pmc_availability("33515491").await? {
    ///         println!("PMC available: {}", pmcid);
    ///         let full_text = client.fetch_full_text(&pmcid).await?;
    ///         println!("Title: {}", full_text.title);
    ///     } else {
    ///         println!("PMC not available");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_pmc_availability(&self, pmid: &str) -> Result<Option<String>> {
        // Validate PMID format
        if pmid.trim().is_empty() || !pmid.chars().all(|c| c.is_ascii_digit()) {
            return Err(PubMedError::InvalidPmid {
                pmid: pmid.to_string(),
            });
        }

        // Build URL with API parameters
        let mut url = format!(
            "{}/elink.fcgi?dbfrom=pubmed&db=pmc&id={pmid}&retmode=json",
            self.base_url
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        let response = self.make_request(&url).await?;

        if !response.status().is_success() {
            return Err(PubMedError::ApiError {
                status: response.status().as_u16(),
                message: response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string(),
            });
        }

        let link_result: serde_json::Value = response.json().await?;

        // Extract PMCID from response
        if let Some(linksets) = link_result["linksets"].as_array() {
            for linkset in linksets {
                if let Some(linksetdbs) = linkset["linksetdbs"].as_array() {
                    for linksetdb in linksetdbs {
                        if linksetdb["dbto"] == "pmc" {
                            if let Some(links) = linksetdb["links"].as_array() {
                                if let Some(pmcid) = links.first() {
                                    return Ok(Some(format!("PMC{pmcid}")));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Download and extract tar.gz file for a PMC article using the OA API
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    /// * `output_dir` - Directory to extract the tar.gz contents to
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing the list of extracted file paths
    ///
    /// # Errors
    ///
    /// * `PubMedError::InvalidPmid` - If the PMCID format is invalid
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::IoError` - If file operations fail
    /// * `PubMedError::PmcNotAvailable` - If the article is not available in OA
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PmcClient;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let output_dir = Path::new("./extracted_articles");
    ///     let files = client.download_and_extract_tar("PMC7906746", output_dir).await?;
    ///
    ///     for file in files {
    ///         println!("Extracted: {}", file);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn download_and_extract_tar<P: AsRef<Path>>(
        &self,
        pmcid: &str,
        output_dir: P,
    ) -> Result<Vec<String>> {
        self.tar_client
            .download_and_extract_tar(pmcid, output_dir)
            .await
    }

    /// Download, extract tar.gz file, and match figures with their captions from XML
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    /// * `output_dir` - Directory to extract the tar.gz contents to
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<ExtractedFigure>>` containing figures with both XML metadata and file paths
    ///
    /// # Errors
    ///
    /// * `PubMedError::InvalidPmid` - If the PMCID format is invalid
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::IoError` - If file operations fail
    /// * `PubMedError::PmcNotAvailable` - If the article is not available in OA
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PmcClient;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let output_dir = Path::new("./extracted_articles");
    ///     let figures = client.extract_figures_with_captions("PMC7906746", output_dir).await?;
    ///
    ///     for figure in figures {
    ///         println!("Figure {}: {}", figure.figure.id, figure.figure.caption);
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
        self.tar_client
            .extract_figures_with_captions(pmcid, output_dir)
            .await
    }

    /// Clear all cached PMC data
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PmcClient;
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
    /// use pubmed_client_rs::PmcClient;
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

    /// Normalize PMCID format (ensure it starts with "PMC")
    fn normalize_pmcid(&self, pmcid: &str) -> String {
        if pmcid.starts_with("PMC") {
            pmcid.to_string()
        } else {
            format!("PMC{pmcid}")
        }
    }

    /// Internal helper method for making HTTP requests with retry logic
    async fn make_request(&self, url: &str) -> Result<Response> {
        with_retry(
            || async {
                self.rate_limiter.acquire().await?;
                debug!("Making API request to: {url}");
                let response = self
                    .client
                    .get(url)
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
        .await
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
