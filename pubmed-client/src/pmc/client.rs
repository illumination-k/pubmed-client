use std::time::Duration;

use crate::cache::{create_cache, PmcCache};
use crate::common::{PmcId, PubMedId};
use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pmc::models::{ExtractedFigure, OaSubsetInfo, PmcFullText};
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
        // Validate and parse PMC ID
        let pmc_id = PmcId::parse(pmcid)?;
        let normalized_pmcid = pmc_id.as_str();
        let numeric_part = pmc_id.numeric_part();

        // Build URL with API parameters
        let mut url = format!(
            "{}/efetch.fcgi?db=pmc&id=PMC{numeric_part}&retmode=xml",
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
                pmcid: normalized_pmcid,
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
        // Validate and parse PMID
        let pmid_obj = PubMedId::parse(pmid)?;
        let pmid_value = pmid_obj.as_u32();

        // Build URL with API parameters
        let mut url = format!(
            "{}/elink.fcgi?dbfrom=pubmed&db=pmc&id={pmid_value}&retmode=json",
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
    /// use pubmed_client_rs::PmcClient;
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
        // Validate and parse PMC ID
        let pmc_id = PmcId::parse(pmcid)?;
        let normalized_pmcid = pmc_id.as_str();

        // OA API endpoint (different from eutils)
        let url = format!(
            "https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id={}",
            normalized_pmcid
        );

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

        // Parse the OA API XML response
        self.parse_oa_response(&xml_content, &normalized_pmcid)
    }

    /// Parse OA API XML response
    fn parse_oa_response(&self, xml: &str, pmcid: &str) -> Result<OaSubsetInfo> {
        // Check for error response first
        // Example: <error code="idIsNotOpenAccess">identifier 'PMC8550608' is not Open Access</error>
        if let Some(error_start) = xml.find("<error") {
            if let Some(error_end) = xml[error_start..].find("</error>") {
                let error_tag = &xml[error_start..error_start + error_end];

                // Extract error code
                let error_code = extract_xml_attribute(error_tag, "code");

                // Extract error message (content between > and </error>)
                let error_message = if let Some(content_start) = error_tag.find('>') {
                    error_tag[content_start + 1..].trim().to_string()
                } else {
                    String::new()
                };

                return Ok(OaSubsetInfo::not_available(
                    pmcid.to_string(),
                    error_code.unwrap_or_else(|| "unknown".to_string()),
                    error_message,
                ));
            }
        }

        // Parse successful response with records
        // Example: <record id="PMC7906746" citation="..." license="none" retracted="no">
        //            <link format="tgz" updated="..." href="ftp://..." />
        //          </record>
        if let Some(record_start) = xml.find("<record") {
            if let Some(record_end) = xml[record_start..].find("</record>") {
                let record_tag = &xml[record_start..record_start + record_end];

                let mut info = OaSubsetInfo::available(pmcid.to_string());

                // Extract record attributes
                info.citation = extract_xml_attribute(record_tag, "citation");
                info.license = extract_xml_attribute(record_tag, "license");

                // Check retracted status
                if let Some(retracted) = extract_xml_attribute(record_tag, "retracted") {
                    info.retracted = retracted == "yes";
                }

                // Extract link information
                if let Some(link_start) = record_tag.find("<link") {
                    if let Some(link_end) = record_tag[link_start..].find("/>") {
                        let link_tag = &record_tag[link_start..link_start + link_end];

                        info.download_format = extract_xml_attribute(link_tag, "format");
                        info.updated = extract_xml_attribute(link_tag, "updated");
                        info.download_link = extract_xml_attribute(link_tag, "href");
                    }
                }

                return Ok(info);
            }
        }

        // If we get here, the response format is unexpected
        debug!(pmcid = %pmcid, xml_snippet = %xml.chars().take(200).collect::<String>(), "Unexpected OA API response format");
        Ok(OaSubsetInfo::not_available(
            pmcid.to_string(),
            "parseError".to_string(),
            "Could not parse OA API response".to_string(),
        ))
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
        // Use PmcId for validation and normalization
        // If parsing fails, fall back to the old behavior for backwards compatibility
        PmcId::parse(pmcid)
            .map(|id| id.as_str())
            .unwrap_or_else(|_| {
                if pmcid.starts_with("PMC") {
                    pmcid.to_string()
                } else {
                    format!("PMC{pmcid}")
                }
            })
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

/// Helper function to extract XML attribute value
fn extract_xml_attribute(tag: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{attr_name}=\"");
    if let Some(start) = tag.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = tag[value_start..].find('"') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }
    None
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

    #[test]
    fn test_extract_xml_attribute() {
        let tag =
            r#"<record id="PMC7906746" citation="Test Citation" license="none" retracted="no">"#;

        assert_eq!(
            extract_xml_attribute(tag, "id"),
            Some("PMC7906746".to_string())
        );
        assert_eq!(
            extract_xml_attribute(tag, "citation"),
            Some("Test Citation".to_string())
        );
        assert_eq!(
            extract_xml_attribute(tag, "license"),
            Some("none".to_string())
        );
        assert_eq!(
            extract_xml_attribute(tag, "retracted"),
            Some("no".to_string())
        );
        assert_eq!(extract_xml_attribute(tag, "nonexistent"), None);
    }

    #[test]
    fn test_parse_oa_response_not_open_access() {
        let client = PmcClient::new();
        let xml = r#"<OA><responseDate>2026-01-02 10:45:24</responseDate><request>https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC8550608</request><error code="idIsNotOpenAccess">identifier 'PMC8550608' is not Open Access</error></OA>"#;

        let result = client.parse_oa_response(xml, "PMC8550608").unwrap();

        assert!(!result.is_oa_subset);
        assert_eq!(result.pmcid, "PMC8550608");
        assert_eq!(result.error_code, Some("idIsNotOpenAccess".to_string()));
        assert!(result
            .error_message
            .as_ref()
            .unwrap()
            .contains("is not Open Access"));
        assert!(result.download_link.is_none());
    }

    #[test]
    fn test_parse_oa_response_open_access() {
        let client = PmcClient::new();
        let xml = r#"<OA><responseDate>2026-01-02 10:45:39</responseDate><request id="PMC7906746">https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC7906746</request><records returned-count="1" total-count="1"><record id="PMC7906746" citation="Lancet. 2021 Jan 27 6-12 February; 397(10273):452-455" license="none" retracted="no"><link format="tgz" updated="2022-12-16 07:10:15" href="ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_package/f1/69/PMC7906746.tar.gz" /></record></records></OA>"#;

        let result = client.parse_oa_response(xml, "PMC7906746").unwrap();

        assert!(result.is_oa_subset);
        assert_eq!(result.pmcid, "PMC7906746");
        assert_eq!(
            result.citation,
            Some("Lancet. 2021 Jan 27 6-12 February; 397(10273):452-455".to_string())
        );
        assert_eq!(result.license, Some("none".to_string()));
        assert!(!result.retracted);
        assert_eq!(result.download_format, Some("tgz".to_string()));
        assert_eq!(result.updated, Some("2022-12-16 07:10:15".to_string()));
        assert_eq!(
            result.download_link,
            Some(
                "ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_package/f1/69/PMC7906746.tar.gz".to_string()
            )
        );
        assert!(result.error_code.is_none());
    }

    #[test]
    fn test_parse_oa_response_retracted() {
        let client = PmcClient::new();
        let xml = r#"<OA><records><record id="PMC1234567" citation="Test" license="cc-by" retracted="yes"><link format="tgz" href="ftp://test.com/file.tar.gz" /></record></records></OA>"#;

        let result = client.parse_oa_response(xml, "PMC1234567").unwrap();

        assert!(result.is_oa_subset);
        assert!(result.retracted);
        assert_eq!(result.license, Some("cc-by".to_string()));
    }
}
