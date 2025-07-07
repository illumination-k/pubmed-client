use std::time::Duration;

use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pmc::models::PmcFullText;
use crate::pmc::parser::PmcXmlParser;
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;
use reqwest::{Client, Response};
use tracing::debug;

#[cfg(not(target_arch = "wasm32"))]
use {
    flate2::read::GzDecoder,
    futures_util::StreamExt,
    std::{fs, fs::File, path::Path},
    tar::Archive,
    tokio::{fs as tokio_fs, io::AsyncWriteExt},
};

/// Client for interacting with PMC (PubMed Central) API
#[derive(Clone)]
pub struct PmcClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
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

        Self {
            client,
            base_url,
            rate_limiter,
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
        let xml_content = self.fetch_xml(pmcid).await?;
        let normalized_pmcid = self.normalize_pmcid(pmcid);
        PmcXmlParser::parse(&xml_content, &normalized_pmcid)
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

    /// Batch check PMC availability for multiple PMIDs
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<(String, Option<String>)>>` containing PMID and optional PMCID pairs
    pub async fn batch_check_pmc_availability(
        &self,
        pmids: &[String],
    ) -> Result<Vec<(String, Option<String>)>> {
        let mut results = Vec::new();

        for pmid in pmids {
            let pmcid = self.check_pmc_availability(pmid).await?;
            results.push((pmid.clone(), pmcid));
        }

        Ok(results)
    }

    /// Batch fetch full text for multiple PMCIDs
    ///
    /// # Arguments
    ///
    /// * `pmcids` - List of PMC IDs
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<Result<PmcFullText>>>` containing results for each PMCID
    pub async fn batch_fetch_full_text(
        &self,
        pmcids: &[String],
    ) -> Result<Vec<Result<PmcFullText>>> {
        let mut results = Vec::new();

        for pmcid in pmcids {
            let result = self.fetch_full_text(pmcid).await;
            results.push(result);
        }

        Ok(results)
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
        let normalized_pmcid = self.normalize_pmcid(pmcid);

        // Validate PMCID format
        let clean_pmcid = normalized_pmcid.trim_start_matches("PMC");
        if clean_pmcid.is_empty() || !clean_pmcid.chars().all(|c| c.is_ascii_digit()) {
            return Err(PubMedError::InvalidPmid {
                pmid: pmcid.to_string(),
            });
        }

        // Build OA API URL
        let mut url = format!(
            "https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id={}&format=tgz",
            normalized_pmcid
        );

        // Add API parameters if available
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        debug!("Downloading tar.gz from OA API: {}", url);

        // Download the tar.gz file
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

        // Check if the response contains an error message
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("text/html") || content_type.contains("text/plain") {
            // Likely an error response, check the content
            let error_text = response.text().await?;
            if error_text.contains("error") || error_text.contains("Error") {
                return Err(PubMedError::PmcNotAvailableById {
                    pmcid: pmcid.to_string(),
                });
            }
            // If we get here, it's likely still an error but we consumed the response
            return Err(PubMedError::PmcNotAvailableById {
                pmcid: pmcid.to_string(),
            });
        }

        // Create output directory if it doesn't exist
        let output_path = output_dir.as_ref();
        tokio_fs::create_dir_all(output_path)
            .await
            .map_err(|e| PubMedError::IoError {
                message: format!("Failed to create output directory: {}", e),
            })?;

        // Stream the response to a temporary file
        let temp_file_path = output_path.join(format!("{}.tar.gz", normalized_pmcid));
        let mut temp_file =
            tokio_fs::File::create(&temp_file_path)
                .await
                .map_err(|e| PubMedError::IoError {
                    message: format!("Failed to create temporary file: {}", e),
                })?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(PubMedError::from)?;
            temp_file
                .write_all(&chunk)
                .await
                .map_err(|e| PubMedError::IoError {
                    message: format!("Failed to write to temporary file: {}", e),
                })?;
        }

        temp_file.flush().await.map_err(|e| PubMedError::IoError {
            message: format!("Failed to flush temporary file: {}", e),
        })?;

        debug!("Downloaded tar.gz to: {}", temp_file_path.display());

        // Extract the tar.gz file
        let extracted_files = self
            .extract_tar_gz(&temp_file_path, &output_path.to_path_buf())
            .await?;

        // Clean up temporary file
        tokio_fs::remove_file(&temp_file_path)
            .await
            .map_err(|e| PubMedError::IoError {
                message: format!("Failed to remove temporary file: {}", e),
            })?;

        Ok(extracted_files)
    }

    /// Extract tar.gz file to the specified directory
    ///
    /// # Arguments
    ///
    /// * `tar_path` - Path to the tar.gz file
    /// * `output_dir` - Directory to extract contents to
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing the list of extracted file paths
    #[cfg(not(target_arch = "wasm32"))]
    async fn extract_tar_gz<P: AsRef<Path>>(
        &self,
        tar_path: P,
        output_dir: P,
    ) -> Result<Vec<String>> {
        let tar_path = tar_path.as_ref();
        let output_dir = output_dir.as_ref();

        // Read the tar.gz file
        let tar_file = File::open(tar_path).map_err(|e| PubMedError::IoError {
            message: format!("Failed to open tar.gz file: {}", e),
        })?;

        let tar_gz = GzDecoder::new(tar_file);
        let mut archive = Archive::new(tar_gz);

        let mut extracted_files = Vec::new();

        // Extract all entries
        for entry in archive.entries().map_err(|e| PubMedError::IoError {
            message: format!("Failed to read tar entries: {}", e),
        })? {
            let mut entry = entry.map_err(|e| PubMedError::IoError {
                message: format!("Failed to read tar entry: {}", e),
            })?;

            let path = entry.path().map_err(|e| PubMedError::IoError {
                message: format!("Failed to get entry path: {}", e),
            })?;

            let output_path = output_dir.join(&path);

            // Create parent directories if they don't exist
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|e| PubMedError::IoError {
                    message: format!("Failed to create parent directories: {}", e),
                })?;
            }

            // Extract the entry
            entry
                .unpack(&output_path)
                .map_err(|e| PubMedError::IoError {
                    message: format!("Failed to extract entry: {}", e),
                })?;

            extracted_files.push(output_path.to_string_lossy().to_string());
            debug!("Extracted: {}", output_path.display());
        }

        Ok(extracted_files)
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
