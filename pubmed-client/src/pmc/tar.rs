use std::{path::Path, str, time::Duration};

use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pmc::models::{ArticleSection, ExtractedFigure, Figure, PmcFullText};
use crate::pmc::parser::parse_pmc_xml;
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;
use reqwest::{Client, Response};
use tracing::debug;

#[cfg(not(target_arch = "wasm32"))]
use {
    flate2::read::GzDecoder,
    futures_util::StreamExt,
    std::{fs, fs::File},
    tar::Archive,
    tokio::{fs as tokio_fs, io::AsyncWriteExt, task},
};

/// TAR extraction client for PMC Open Access articles
#[derive(Clone)]
pub struct PmcTarClient {
    client: Client,
    rate_limiter: RateLimiter,
    pub(crate) config: ClientConfig,
}

impl PmcTarClient {
    /// Create a new PMC TAR client with configuration
    pub fn new(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();

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

        // Create output directory early (before any potential failures)
        let output_path = output_dir.as_ref();
        tokio_fs::create_dir_all(output_path)
            .await
            .map_err(|e| PubMedError::IoError {
                message: format!("Failed to create output directory: {}", e),
            })?;

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
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn extract_figures_with_captions<P: AsRef<Path>>(
        &self,
        pmcid: &str,
        output_dir: P,
    ) -> Result<Vec<ExtractedFigure>> {
        let normalized_pmcid = self.normalize_pmcid(pmcid);

        // Create output directory early (before any potential failures)
        let output_path = output_dir.as_ref();
        tokio_fs::create_dir_all(output_path)
            .await
            .map_err(|e| PubMedError::IoError {
                message: format!("Failed to create output directory: {}", e),
            })?;

        // First, fetch the XML to get figure captions
        let xml_content = self.fetch_xml(&normalized_pmcid).await?;
        let full_text = parse_pmc_xml(&xml_content, &normalized_pmcid)?;

        // Extract the tar.gz file
        let extracted_files = self
            .download_and_extract_tar(&normalized_pmcid, &output_dir)
            .await?;

        // Find and match figures
        let figures = self
            .match_figures_with_files(&full_text, &extracted_files, &output_dir)
            .await?;

        Ok(figures)
    }

    /// Fetch raw XML content from PMC
    #[cfg(not(target_arch = "wasm32"))]
    async fn fetch_xml(&self, pmcid: &str) -> Result<String> {
        // Remove PMC prefix if present and validate
        let clean_pmcid = pmcid.trim_start_matches("PMC");
        if clean_pmcid.is_empty() || !clean_pmcid.chars().all(|c| c.is_ascii_digit()) {
            return Err(PubMedError::InvalidPmid {
                pmid: pmcid.to_string(),
            });
        }

        // Build URL with API parameters
        let mut url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=pmc&id=PMC{clean_pmcid}&retmode=xml"
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

    /// Parse OA API XML response to extract download URL
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_oa_response(&self, xml_content: &str, pmcid: &str) -> Result<String> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        debug!("Parsing OA API XML response: {}", xml_content);

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e))
                    if e.name().as_ref() == b"link" =>
                {
                    debug!("Found link element");
                    // Look for href attribute
                    for attr in e.attributes().flatten() {
                        debug!(
                            "Attribute: {:?} = {:?}",
                            str::from_utf8(attr.key.as_ref()).unwrap_or("invalid"),
                            str::from_utf8(&attr.value).unwrap_or("invalid")
                        );
                        if attr.key.as_ref() == b"href" {
                            let href = str::from_utf8(&attr.value).map_err(|e| {
                                PubMedError::XmlError(format!("Invalid UTF-8 in href: {}", e))
                            })?;
                            debug!("Found href: {}", href);
                            return Ok(href.to_string());
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(PubMedError::XmlError(format!("XML parsing error: {}", e)));
                }
                _ => {}
            }
            buf.clear();
        }

        debug!("No href attribute found in XML response");
        Err(PubMedError::PmcNotAvailableById {
            pmcid: pmcid.to_string(),
        })
    }

    /// Match figures from XML with extracted files
    #[cfg(not(target_arch = "wasm32"))]
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
    #[cfg(not(target_arch = "wasm32"))]
    fn collect_figures_recursive(section: &ArticleSection, figures: &mut Vec<Figure>) {
        figures.extend(section.figures.clone());
        for subsection in &section.subsections {
            Self::collect_figures_recursive(subsection, figures);
        }
    }

    /// Find a matching file for a figure based on ID, label, or filename patterns
    #[cfg(not(target_arch = "wasm32"))]
    pub fn find_matching_file(
        figure: &Figure,
        extracted_files: &[String],
        image_extensions: &[&str],
    ) -> Option<String> {
        // First try to match by figure file_name if available
        if let Some(file_name) = &figure.file_name {
            for file_path in extracted_files {
                if let Some(filename) = Path::new(file_path).file_name() {
                    if filename.to_string_lossy().contains(file_name) {
                        return Some(file_path.clone());
                    }
                }
            }
        }

        // Try to match by figure ID
        for file_path in extracted_files {
            if let Some(filename) = Path::new(file_path).file_name() {
                let filename_str = filename.to_string_lossy().to_lowercase();
                let figure_id_lower = figure.id.to_lowercase();

                // Check if filename contains figure ID and has image extension
                if filename_str.contains(&figure_id_lower) {
                    if let Some(extension) = Path::new(file_path).extension() {
                        let ext_str = extension.to_string_lossy().to_lowercase();
                        if image_extensions.contains(&ext_str.as_str()) {
                            return Some(file_path.clone());
                        }
                    }
                }
            }
        }

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

        None
    }

    /// Get image dimensions using the image crate
    #[cfg(not(target_arch = "wasm32"))]
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
}
