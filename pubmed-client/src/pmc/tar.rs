//! TAR extraction functionality for PMC Open Access articles
//!
//! This module is only available on non-WASM targets due to file system and
//! compression dependencies.

#![cfg(not(target_arch = "wasm32"))]

use std::{path::Path, str, time::Duration};

use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pmc::models::{ArticleSection, ExtractedFigure, Figure, PmcFullText};
use crate::pmc::parser::PmcXmlParser;
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use quick_xml::de::from_str;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::{fs, fs::File};
use tar::Archive;
use tokio::{fs as tokio_fs, io::AsyncWriteExt, task};
use tracing::debug;

/// OA API XML response structures for deserialization
#[derive(Debug, Deserialize, Serialize)]
struct OaResponse {
    #[serde(rename = "responseDate")]
    response_date: Option<String>,
    request: Option<String>,
    records: Option<OaRecords>,
    error: Option<OaError>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OaRecords {
    #[serde(rename = "returned-count")]
    returned_count: Option<String>,
    #[serde(rename = "total-count")]
    total_count: Option<String>,
    record: Option<OaRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OaRecord {
    #[serde(rename = "@id")]
    id: Option<String>,
    #[serde(rename = "@citation")]
    citation: Option<String>,
    #[serde(rename = "@license")]
    license: Option<String>,
    #[serde(rename = "@retracted")]
    retracted: Option<String>,
    link: Option<OaLink>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OaLink {
    #[serde(rename = "@format")]
    format: Option<String>,
    #[serde(rename = "@updated")]
    updated: Option<String>,
    #[serde(rename = "@href")]
    href: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OaError {
    #[serde(rename = "@code")]
    code: Option<String>,
    #[serde(rename = "$text")]
    message: Option<String>,
}

/// TAR extraction client for PMC Open Access articles
#[derive(Clone)]
pub struct PmcTarClient {
    client: Client,
    rate_limiter: RateLimiter,
    config: ClientConfig,
}

impl PmcTarClient {
    /// Create a new PMC TAR client with configuration
    pub fn new(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();

        let client = Client::builder()
            .user_agent(config.effective_user_agent())
            .timeout(Duration::from_secs(config.timeout.as_secs()))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            rate_limiter,
            config,
        }
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
    /// use pubmed_client_rs::pmc::tar::PmcTarClient;
    /// use pubmed_client_rs::ClientConfig;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = ClientConfig::new();
    ///     let client = PmcTarClient::new(config);
    ///     let output_dir = Path::new("./extracted_articles");
    ///     let files = client.download_and_extract_tar("PMC7906746", output_dir).await?;
    ///
    ///     for file in files {
    ///         println!("Extracted: {}", file);
    ///     }
    ///     Ok(())
    /// }
    /// ```
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

        // Create output directory early (before any potential failures)
        let output_path = output_dir.as_ref();
        tokio_fs::create_dir_all(output_path)
            .await
            .map_err(|e| PubMedError::IoError {
                message: format!("Failed to create output directory: {}", e),
            })?;

        // Build OA API URL
        let url = self.build_oa_api_url(&normalized_pmcid);

        debug!("Downloading tar.gz from OA API: {}", url);

        // Download the OA API response
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

        // Check if the response is XML (OA API response with download link)
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        debug!("OA API response content-type: {}", content_type);

        let download_url =
            if content_type.contains("text/xml") || content_type.contains("application/xml") {
                // Parse XML to extract the actual download URL
                let xml_content = response.text().await?;
                debug!("OA API returned XML, parsing for download URL");
                let parsed_url = self.parse_oa_response(&xml_content, pmcid)?;
                // Convert FTP URLs to HTTPS for HTTP client compatibility
                if parsed_url.starts_with("ftp://ftp.ncbi.nlm.nih.gov/") {
                    parsed_url.replace(
                        "ftp://ftp.ncbi.nlm.nih.gov/",
                        "https://ftp.ncbi.nlm.nih.gov/",
                    )
                } else {
                    parsed_url
                }
            } else if content_type.contains("application/x-gzip")
                || content_type.contains("application/gzip")
            {
                // Direct tar.gz download - use the original URL
                url.clone()
            } else {
                // Check if it's an error response
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
            };

        // Now download the actual tar.gz file
        let tar_response = self.make_request(&download_url).await?;

        if !tar_response.status().is_success() {
            return Err(PubMedError::ApiError {
                status: tar_response.status().as_u16(),
                message: tar_response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string(),
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

        let mut stream = tar_response.bytes_stream();
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
    /// use pubmed_client_rs::pmc::tar::PmcTarClient;
    /// use pubmed_client_rs::ClientConfig;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = ClientConfig::new();
    ///     let client = PmcTarClient::new(config);
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
    pub async fn extract_figures_with_captions<P: AsRef<Path>>(
        &self,
        pmcid: &str,
        output_dir: P,
    ) -> Result<Vec<ExtractedFigure>> {
        let normalized_pmcid = self.normalize_pmcid(pmcid);

        // Create output directory first to ensure it exists even if the download fails
        let output_path = output_dir.as_ref();
        tokio_fs::create_dir_all(output_path)
            .await
            .map_err(|e| PubMedError::IoError {
                message: format!("Failed to create output directory: {}", e),
            })?;

        // Check if PMC is available in OA
        self.check_oa_availability(&normalized_pmcid).await?;

        // Extract the tar.gz file
        let extracted_files = self
            .download_and_extract_tar(&normalized_pmcid, &output_dir)
            .await?;

        // Find and read the NXML file from extracted files instead of making API call
        let xml_content = self
            .read_nxml_from_extracted_files(&extracted_files, &normalized_pmcid)
            .await?;
        let full_text = PmcXmlParser::parse(&xml_content, &normalized_pmcid)?;

        // Find and match figures
        let figures = self
            .match_figures_with_files(&full_text, &extracted_files, &output_dir)
            .await?;

        Ok(figures)
    }

    /// Read NXML file content from extracted tar files
    pub async fn read_nxml_from_extracted_files(
        &self,
        extracted_files: &[String],
        pmcid: &str,
    ) -> Result<String> {
        // Look for .nxml files in the extracted files
        let nxml_file = extracted_files
            .iter()
            .find(|file_path| {
                Path::new(file_path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_lowercase() == "nxml")
                    .unwrap_or(false)
            })
            .ok_or_else(|| PubMedError::PmcNotAvailableById {
                pmcid: pmcid.to_string(),
            })?;

        debug!("Found NXML file: {}", nxml_file);

        // Read the NXML file content
        tokio_fs::read_to_string(nxml_file)
            .await
            .map_err(|e| PubMedError::IoError {
                message: format!("Failed to read NXML file {}: {}", nxml_file, e),
            })
    }

    /// Check if PMC ID is available in Open Access subset
    async fn check_oa_availability(&self, pmcid: &str) -> Result<()> {
        // Validate PMCID format
        let clean_pmcid = pmcid.trim_start_matches("PMC");
        if clean_pmcid.is_empty() || !clean_pmcid.chars().all(|c| c.is_ascii_digit()) {
            return Err(PubMedError::InvalidPmid {
                pmid: pmcid.to_string(),
            });
        }

        // Build OA API URL
        let url = self.build_oa_api_url(pmcid);

        debug!("Checking OA availability: {}", url);

        // Make request to check availability
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

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        debug!("OA API response content-type: {}", content_type);

        if content_type.contains("text/xml") {
            debug!("OA API returned XML, checking for errors");
            let xml_content = response.text().await?;

            // Check for error in XML response
            if xml_content.contains("<error") {
                return Err(PubMedError::PmcNotAvailableById {
                    pmcid: pmcid.to_string(),
                });
            }
        }

        // If we reach here, the PMC ID is available
        Ok(())
    }

    /// Parse OA API XML response to extract download URL using serde deserialization
    pub(crate) fn parse_oa_response(&self, xml_content: &str, pmcid: &str) -> Result<String> {
        debug!("Parsing OA API XML response with serde: {}", xml_content);

        // Deserialize XML to struct
        let oa_response: OaResponse = from_str(xml_content).map_err(|e| {
            PubMedError::XmlError(format!("Failed to deserialize OA response: {}", e))
        })?;

        // Check for error in response
        if let Some(_error) = &oa_response.error {
            return Err(PubMedError::PmcNotAvailableById {
                pmcid: pmcid.to_string(),
            });
        }

        // Extract href from records
        if let Some(records) = &oa_response.records {
            if let Some(record) = &records.record {
                if let Some(link) = &record.link {
                    if let Some(href) = &link.href {
                        debug!("Found href using serde: {}", href);
                        return Ok(href.clone());
                    }
                }
            }
        }

        debug!("No href found in OA response");
        Err(PubMedError::PmcNotAvailableById {
            pmcid: pmcid.to_string(),
        })
    }

    /// Match figures from XML with extracted files
    async fn match_figures_with_files<P: AsRef<Path>>(
        &self,
        full_text: &PmcFullText,
        extracted_files: &[String],
        output_dir: P,
    ) -> Result<Vec<ExtractedFigure>> {
        let output_path = output_dir.as_ref();
        let mut matched_figures = Vec::new();

        // Collect all figures from all sections
        let mut all_figures = Vec::new();
        for section in &full_text.sections {
            Self::collect_figures_recursive(section, &mut all_figures);
        }

        // Common image extensions to look for
        let image_extensions = [
            "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
        ];

        for figure in all_figures {
            // Try to find a matching file for this figure
            let matching_file =
                Self::find_matching_file(&figure, extracted_files, &image_extensions);

            if let Some(file_path) = matching_file {
                let absolute_path =
                    if file_path.starts_with(&output_path.to_string_lossy().to_string()) {
                        file_path.clone()
                    } else {
                        output_path.join(&file_path).to_string_lossy().to_string()
                    };

                // Get file size
                let file_size = tokio_fs::metadata(&absolute_path)
                    .await
                    .map(|m| m.len())
                    .ok();

                // Try to get image dimensions
                let dimensions = Self::get_image_dimensions(&absolute_path).await;

                matched_figures.push(ExtractedFigure {
                    figure: figure.clone(),
                    extracted_file_path: absolute_path,
                    file_size,
                    dimensions,
                });
            }
        }

        Ok(matched_figures)
    }

    /// Recursively collect all figures from sections and subsections
    fn collect_figures_recursive(section: &ArticleSection, figures: &mut Vec<Figure>) {
        figures.extend(section.figures.clone());
        for subsection in &section.subsections {
            Self::collect_figures_recursive(subsection, figures);
        }
    }

    /// Find a matching file for a figure based on ID, label, or filename patterns
    pub fn find_matching_file(
        figure: &Figure,
        extracted_files: &[String],
        image_extensions: &[&str],
    ) -> Option<String> {
        use tracing::debug;

        debug!(
            "Finding matching file for figure: id={}, file_name={:?}, label={:?}",
            figure.id, figure.file_name, figure.label
        );
        debug!(
            "Available files: {:?}",
            extracted_files
                .iter()
                .map(|f| Path::new(f)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy())
                .collect::<Vec<_>>()
        );

        // First try to match by figure file_name if available
        if let Some(file_name) = &figure.file_name {
            debug!("Trying to match by file_name: {}", file_name);
            for file_path in extracted_files {
                if let Some(filename) = Path::new(file_path).file_name() {
                    let filename_str = filename.to_string_lossy();
                    debug!("Checking if '{}' contains '{}'", filename_str, file_name);
                    if filename_str.contains(file_name) {
                        debug!("Found match by file_name: {}", file_path);
                        return Some(file_path.clone());
                    }
                }
            }
            debug!("No match found by file_name");
        } else {
            debug!("No file_name available for matching");
        }

        // Try to match by figure ID
        debug!("Trying to match by figure ID: {}", figure.id);
        for file_path in extracted_files {
            if let Some(filename) = Path::new(file_path).file_name() {
                let filename_str = filename.to_string_lossy().to_lowercase();
                let figure_id_lower = figure.id.to_lowercase();

                // Check if filename contains figure ID and has image extension
                if filename_str.contains(&figure_id_lower) {
                    if let Some(extension) = Path::new(file_path).extension() {
                        let ext_str = extension.to_string_lossy().to_lowercase();
                        if image_extensions.contains(&ext_str.as_str()) {
                            debug!("Found match by figure ID: {}", file_path);
                            return Some(file_path.clone());
                        }
                    }
                }
            }
        }
        debug!("No match found by figure ID");

        // Try to match by label if available
        if let Some(label) = &figure.label {
            let label_clean = label.to_lowercase().replace([' ', '.'], "");
            for file_path in extracted_files {
                if let Some(filename) = Path::new(file_path).file_name() {
                    let filename_str = filename.to_string_lossy().to_lowercase();
                    if filename_str.contains(&label_clean) {
                        if let Some(extension) = Path::new(file_path).extension() {
                            let ext_str = extension.to_string_lossy().to_lowercase();
                            if image_extensions.contains(&ext_str.as_str()) {
                                return Some(file_path.clone());
                            }
                        }
                    }
                }
            }
        }

        debug!("No matching file found for figure: {}", figure.id);
        None
    }

    /// Get image dimensions using the image crate
    async fn get_image_dimensions(file_path: &str) -> Option<(u32, u32)> {
        task::spawn_blocking({
            let file_path = file_path.to_string();
            move || {
                image::open(&file_path)
                    .ok()
                    .map(|img| (img.width(), img.height()))
            }
        })
        .await
        .ok()
        .flatten()
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

    /// Build OA API URL with parameters
    fn build_oa_api_url(&self, pmcid: &str) -> String {
        let mut url = format!(
            "https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id={}&format=tgz",
            pmcid
        );

        // Add API parameters if available
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        url
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pmcid() {
        let config = ClientConfig::new();
        let client = PmcTarClient::new(config);

        assert_eq!(client.normalize_pmcid("1234567"), "PMC1234567");
        assert_eq!(client.normalize_pmcid("PMC1234567"), "PMC1234567");
    }

    #[test]
    fn test_client_creation() {
        let config = ClientConfig::new();
        let _client = PmcTarClient::new(config);
        // Test that client is created successfully
    }

    #[test]
    fn test_parse_oa_response_success() {
        let config = ClientConfig::new();
        let client = PmcTarClient::new(config);

        // Sample successful OA API response
        let xml_response = r#"<OA>
            <responseDate>2025-07-14 01:46:30</responseDate>
            <request id="PMC7906746" format="tgz">https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC7906746;format=tgz;tool=pubmed-client-rs</request>
            <records returned-count="1" total-count="1">
                <record id="PMC7906746" citation="Lancet. 2021 Jan 27 6-12 February; 397(10273):452-455" license="none" retracted="no">
                    <link format="tgz" updated="2022-12-16 07:10:15" href="ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_package/f1/69/PMC7906746.tar.gz" />
                </record>
            </records>
        </OA>"#;

        let result = client.parse_oa_response(xml_response, "PMC7906746");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_package/f1/69/PMC7906746.tar.gz"
        );
    }

    #[test]
    fn test_parse_oa_response_error() {
        let config = ClientConfig::new();
        let client = PmcTarClient::new(config);

        // Sample error OA API response
        let xml_response = r#"<OA>
            <responseDate>2025-07-13 23:54:19</responseDate>
            <request>https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC1474093;format=tgz;tool=pubmed-client-rs</request>
            <error code="idIsNotOpenAccess">identifier 'PMC1474093' is not Open Access</error>
        </OA>"#;

        let result = client.parse_oa_response(xml_response, "PMC1474093");
        assert!(result.is_err());

        assert!(
            matches!(result, Err(PubMedError::PmcNotAvailableById { pmcid }) if pmcid == "PMC1474093"),
            "Expected PmcNotAvailableById error with correct pmcid"
        );
    }

    #[test]
    fn test_parse_oa_response_no_href() {
        let config = ClientConfig::new();
        let client = PmcTarClient::new(config);

        // Response with records but no href
        let xml_response = r#"<OA>
            <responseDate>2025-07-14 01:46:30</responseDate>
            <request id="PMC1234567" format="tgz">test request</request>
            <records returned-count="1" total-count="1">
                <record id="PMC1234567" citation="Test Citation" license="none" retracted="no">
                    <link format="tgz" updated="2022-12-16 07:10:15" />
                </record>
            </records>
        </OA>"#;

        let result = client.parse_oa_response(xml_response, "PMC1234567");
        assert!(result.is_err());

        assert!(
            matches!(result, Err(PubMedError::PmcNotAvailableById { pmcid }) if pmcid == "PMC1234567"),
            "Expected PmcNotAvailableById error for missing href"
        );
    }

    #[test]
    fn test_parse_oa_response_invalid_xml() {
        let config = ClientConfig::new();
        let client = PmcTarClient::new(config);

        // Invalid XML
        let xml_response = r#"<OA><unclosed-tag></OA>"#;

        let result = client.parse_oa_response(xml_response, "PMC1234567");
        assert!(result.is_err());

        assert!(
            matches!(result, Err(PubMedError::XmlError(_))),
            "Expected XmlError for invalid XML"
        );
    }
}
