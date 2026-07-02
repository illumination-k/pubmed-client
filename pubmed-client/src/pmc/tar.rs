use std::{path::Path, str, time::Duration};

use crate::common::PmcId;
use crate::config::ClientConfig;
use crate::error::{ParseError, PubMedError, Result};
use crate::pmc::common;
use crate::pmc::extracted::ExtractedFigure;
use crate::pmc::parser::parse_pmc_xml;
use crate::rate_limit::RateLimiter;
use crate::request::RequestExecutor;
use pubmed_parser::pmc::{Figure, PmcArticle, Section};
use reqwest::Client;
use tracing::{debug, warn};

#[cfg(not(target_arch = "wasm32"))]
use {
    flate2::read::GzDecoder,
    futures_util::StreamExt,
    std::{fs, fs::File},
    tar::Archive,
    tempfile::NamedTempFile,
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

        Self {
            client,
            rate_limiter,
            config,
        }
    }

    /// Create a TAR client sharing an existing HTTP client and rate limiter.
    ///
    /// Used by `PmcClient` to avoid duplicating the HTTP client and rate limiter.
    pub(crate) fn with_shared(
        client: Client,
        rate_limiter: RateLimiter,
        config: ClientConfig,
    ) -> Self {
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
    /// * `ParseError::InvalidPmid` - If the PMCID format is invalid
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `ParseError::IoError` - If file operations fail
    /// * `ParseError::PmcNotAvailable` - If the article is not available in OA
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::pmc::tar::PmcTarClient;
    /// use pubmed_client::ClientConfig;
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
        let pmc_id = PmcId::parse(pmcid)?;
        let normalized_pmcid = pmc_id.as_str();

        let output_path = output_dir.as_ref();
        tokio_fs::create_dir_all(output_path)
            .await
            .map_err(|e| ParseError::IoError {
                message: format!("Failed to create output directory: {}", e),
            })?;

        // Prefer the PMC OA Cloud (AWS S3) service. NCBI is retiring the PMC FTP
        // service and the legacy `oa_package` tar.gz bundles in August 2026; the
        // cloud service serves each article's XML, media, and supplementary files
        // as individual per-article objects instead.
        match self
            .download_cloud_files(&normalized_pmcid, output_path)
            .await
        {
            Ok(files) if !files.is_empty() => return Ok(files),
            Ok(_) => {
                debug!(
                    pmcid = %normalized_pmcid,
                    "PMC OA Cloud returned no files; falling back to legacy OA package"
                );
            }
            Err(e) => {
                warn!(
                    pmcid = %normalized_pmcid,
                    error = %e,
                    "PMC OA Cloud download failed; falling back to legacy OA package"
                );
            }
        }

        // Fallback: legacy OA package tar.gz via the OA Web Service API.
        // NOTE: this path relies on the deprecated FTP/oa_package bundles and
        // will stop working once NCBI removes them (August 2026).
        let download_url = self.resolve_download_url(&normalized_pmcid, pmcid).await?;
        let temp_file = self.stream_to_temp_file(&download_url, output_path).await?;

        let extracted_files = self.extract_tar_gz(temp_file.path(), output_path).await?;

        Ok(extracted_files)
    }

    /// Download an article's files from the PMC OA Cloud (AWS S3) service.
    ///
    /// Lists the objects under the article's prefix in the `pmc-oa-opendata`
    /// bucket, selects the latest version folder, and downloads each file into
    /// `output_dir`. Returns the list of local file paths (empty if the article
    /// is not present in the cloud bucket, signalling the caller to fall back to
    /// the legacy OA package).
    #[cfg(not(target_arch = "wasm32"))]
    async fn download_cloud_files(
        &self,
        normalized_pmcid: &str,
        output_dir: &Path,
    ) -> Result<Vec<String>> {
        let keys = self.list_cloud_object_keys(normalized_pmcid).await?;
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let base_url = self.config.effective_oa_cloud_base_url();
        let mut downloaded = Vec::with_capacity(keys.len());

        for key in keys {
            // The object filename is the last path segment of the S3 key,
            // e.g. `PMC7906746.1/gr1_lrg.jpg` -> `gr1_lrg.jpg`.
            let Some(file_name) = key.rsplit('/').next().filter(|s| !s.is_empty()) else {
                continue;
            };

            let url = format!("{}/{}", base_url.trim_end_matches('/'), key);
            let response = self.executor().get(&url).await?;
            let bytes = response.bytes().await.map_err(PubMedError::from)?;

            let output_path = output_dir.join(file_name);
            tokio_fs::write(&output_path, &bytes)
                .await
                .map_err(|e| ParseError::IoError {
                    message: format!("Failed to write cloud file {}: {}", file_name, e),
                })?;

            debug!("Downloaded cloud file: {}", output_path.display());
            downloaded.push(output_path.to_string_lossy().to_string());
        }

        Ok(downloaded)
    }

    /// List the S3 object keys for an article's latest version in the OA Cloud.
    ///
    /// Queries the bucket's ListObjectsV2 endpoint with the article prefix
    /// (`<PMCID>.`, the trailing dot preventing matches against longer PMCIDs),
    /// then keeps only the keys belonging to the highest version folder
    /// (`<PMCID>.<n>/`).
    #[cfg(not(target_arch = "wasm32"))]
    async fn list_cloud_object_keys(&self, normalized_pmcid: &str) -> Result<Vec<String>> {
        let base_url = self.config.effective_oa_cloud_base_url();
        // The trailing dot restricts the prefix to `<PMCID>.<version>/...`,
        // so e.g. `PMC790674` does not also match `PMC7906740`.
        let url = format!(
            "{}/?list-type=2&prefix={}.",
            base_url.trim_end_matches('/'),
            normalized_pmcid
        );

        debug!("Listing PMC OA Cloud objects: {}", url);
        let response = self.executor().get(&url).await?;
        let body = response.text().await?;

        let keys = Self::parse_cloud_listing(&body)?;
        Ok(Self::select_latest_version_keys(keys))
    }

    /// Parse the `<Key>` entries from an S3 ListObjectsV2 XML response.
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_cloud_listing(xml_content: &str) -> Result<Vec<String>> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut keys = Vec::new();
        let mut in_key = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"Key" => in_key = true,
                Ok(Event::End(ref e)) if e.name().as_ref() == b"Key" => in_key = false,
                Ok(Event::Text(ref e)) if in_key => {
                    let text = e
                        .unescape()
                        .map_err(|err| {
                            ParseError::XmlError(format!("Invalid UTF-8 in S3 Key: {}", err))
                        })?
                        .to_string();
                    // Skip folder-marker keys (zero-byte objects ending in `/`).
                    if !text.ends_with('/') {
                        keys.push(text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(
                        ParseError::XmlError(format!("Failed to parse S3 listing: {}", e)).into(),
                    );
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(keys)
    }

    /// From a flat list of keys, keep only those under the highest version folder.
    ///
    /// Keys look like `PMC7906746.1/PMC7906746.1.xml`; the version is the integer
    /// after the last `.` of the leading `<folder>/` segment. When multiple
    /// versions are present, only the latest is retained.
    #[cfg(not(target_arch = "wasm32"))]
    fn select_latest_version_keys(keys: Vec<String>) -> Vec<String> {
        fn version_of(key: &str) -> Option<u32> {
            let folder = key.split('/').next()?;
            folder.rsplit('.').next()?.parse::<u32>().ok()
        }

        let Some(latest) = keys.iter().filter_map(|k| version_of(k)).max() else {
            return keys;
        };

        keys.into_iter()
            .filter(|k| version_of(k) == Some(latest))
            .collect()
    }

    /// Convert a legacy OA package FTP link into a working HTTPS URL.
    ///
    /// The OA Web Service API still advertises `ftp://ftp.ncbi.nlm.nih.gov/...`
    /// links. FTP itself is unsupported here, and as of April 2026 NCBI moved the
    /// legacy `oa_package` bundles under a `deprecated/` prefix (to be removed in
    /// August 2026), so the plain host swap now 404s. Rewrite the `pub/pmc/` path
    /// to `pub/pmc/deprecated/` so the fallback keeps working during the
    /// transition; any other FTP link just gets an `ftp` -> `https` host swap.
    #[cfg(not(target_arch = "wasm32"))]
    fn oa_package_url_to_https(url: &str) -> String {
        const FTP_PMC_PREFIX: &str = "ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/";
        const FTP_HOST_PREFIX: &str = "ftp://ftp.ncbi.nlm.nih.gov/";

        if let Some(rest) = url.strip_prefix(FTP_PMC_PREFIX) {
            if rest.starts_with("deprecated/") {
                format!("https://ftp.ncbi.nlm.nih.gov/pub/pmc/{rest}")
            } else {
                format!("https://ftp.ncbi.nlm.nih.gov/pub/pmc/deprecated/{rest}")
            }
        } else if let Some(rest) = url.strip_prefix(FTP_HOST_PREFIX) {
            format!("https://ftp.ncbi.nlm.nih.gov/{rest}")
        } else {
            url.to_string()
        }
    }

    /// Resolve the actual tar.gz download URL via the OA API.
    ///
    /// The OA API may return an XML document containing the real download link,
    /// or it may serve the tar.gz directly.
    #[cfg(not(target_arch = "wasm32"))]
    async fn resolve_download_url(
        &self,
        normalized_pmcid: &str,
        original_pmcid: &str,
    ) -> Result<String> {
        let url = self.executor().build_url(
            "https://www.ncbi.nlm.nih.gov/pmc/utils/oa",
            "oa.fcgi",
            &[("id", normalized_pmcid), ("format", "tgz")],
        )?;

        debug!("Downloading tar.gz from OA API: {}", url);

        let response = self.executor().get(&url).await?;

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        debug!("OA API response content-type: {}", content_type);

        if content_type.contains("text/xml") || content_type.contains("application/xml") {
            let xml_content = response.text().await?;
            debug!("OA API returned XML, parsing for download URL");
            let parsed_url = self.parse_oa_response(&xml_content, original_pmcid)?;
            Ok(Self::oa_package_url_to_https(&parsed_url))
        } else if content_type.contains("application/x-gzip")
            || content_type.contains("application/gzip")
        {
            Ok(url)
        } else {
            let error_text = response.text().await?;
            if error_text.contains("error") || error_text.contains("Error") {
                return Err(ParseError::PmcNotAvailable {
                    id: original_pmcid.to_string(),
                }
                .into());
            }
            Err(ParseError::PmcNotAvailable {
                id: original_pmcid.to_string(),
            }
            .into())
        }
    }

    /// Stream a tar.gz response into a temporary file with RAII cleanup.
    ///
    /// The returned `NamedTempFile` is automatically deleted when dropped,
    /// ensuring no leftover files on any error path.
    #[cfg(not(target_arch = "wasm32"))]
    async fn stream_to_temp_file(
        &self,
        download_url: &str,
        output_dir: &Path,
    ) -> Result<NamedTempFile> {
        let tar_response = self.executor().get(download_url).await?;

        let temp_file = NamedTempFile::new_in(output_dir).map_err(|e| ParseError::IoError {
            message: format!("Failed to create temporary file: {}", e),
        })?;

        let temp_path = temp_file.path().to_path_buf();
        let mut async_file =
            tokio_fs::File::create(&temp_path)
                .await
                .map_err(|e| ParseError::IoError {
                    message: format!("Failed to open temporary file for writing: {}", e),
                })?;

        let mut stream = tar_response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(PubMedError::from)?;
            async_file
                .write_all(&chunk)
                .await
                .map_err(|e| ParseError::IoError {
                    message: format!("Failed to write to temporary file: {}", e),
                })?;
        }

        async_file.flush().await.map_err(|e| ParseError::IoError {
            message: format!("Failed to flush temporary file: {}", e),
        })?;

        debug!("Downloaded tar.gz to: {}", temp_path.display());

        Ok(temp_file)
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
    /// * `ParseError::InvalidPmid` - If the PMCID format is invalid
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `ParseError::IoError` - If file operations fail
    /// * `ParseError::PmcNotAvailable` - If the article is not available in OA
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::pmc::tar::PmcTarClient;
    /// use pubmed_client::ClientConfig;
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
        let normalized_pmcid = common::normalize_pmcid(pmcid);

        let output_path = output_dir.as_ref();
        tokio_fs::create_dir_all(output_path)
            .await
            .map_err(|e| ParseError::IoError {
                message: format!("Failed to create output directory: {}", e),
            })?;

        let xml_content = common::fetch_pmc_xml(
            &self.executor(),
            self.config.effective_base_url(),
            &normalized_pmcid,
        )
        .await?;
        let full_text = parse_pmc_xml(&xml_content, &normalized_pmcid)?;

        let extracted_files = self
            .download_and_extract_tar(&normalized_pmcid, &output_dir)
            .await?;

        let figures = self
            .match_figures_with_files(&full_text, &extracted_files, &output_dir)
            .await?;

        Ok(figures)
    }

    /// Parse OA API XML response to extract download URL
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_oa_response(&self, xml_content: &str, pmcid: &str) -> Result<String> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

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
                    for attr in e.attributes().flatten() {
                        debug!(
                            "Attribute: {:?} = {:?}",
                            str::from_utf8(attr.key.as_ref()).unwrap_or("invalid"),
                            str::from_utf8(&attr.value).unwrap_or("invalid")
                        );
                        if attr.key.as_ref() == b"href" {
                            let href = str::from_utf8(&attr.value).map_err(|e| {
                                ParseError::XmlError(format!("Invalid UTF-8 in href: {}", e))
                            })?;
                            debug!("Found href: {}", href);
                            return Ok(href.to_string());
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(ParseError::XmlError(format!("XML parsing error: {}", e)).into());
                }
                _ => {}
            }
            buf.clear();
        }

        debug!("No href attribute found in XML response");
        Err(ParseError::PmcNotAvailable {
            id: pmcid.to_string(),
        }
        .into())
    }

    /// Match figures from XML with extracted files
    #[cfg(not(target_arch = "wasm32"))]
    async fn match_figures_with_files<P: AsRef<Path>>(
        &self,
        full_text: &PmcArticle,
        extracted_files: &[String],
        output_dir: P,
    ) -> Result<Vec<ExtractedFigure>> {
        let output_path = output_dir.as_ref();
        let mut matched_figures = Vec::new();

        let mut all_figures = Vec::new();
        for section in full_text.sections() {
            Self::collect_figures_recursive(section, &mut all_figures);
        }

        let image_extensions = [
            "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
        ];

        for figure in all_figures {
            let matching_file =
                Self::find_matching_file(&figure, extracted_files, &image_extensions);

            if let Some(file_path) = matching_file {
                let absolute_path =
                    if file_path.starts_with(&output_path.to_string_lossy().to_string()) {
                        file_path.clone()
                    } else {
                        output_path.join(&file_path).to_string_lossy().to_string()
                    };

                let file_size = tokio_fs::metadata(&absolute_path)
                    .await
                    .map(|m| m.len())
                    .ok();

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
    fn collect_figures_recursive(section: &Section, figures: &mut Vec<Figure>) {
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
        if let Some(file_name) = &figure.graphic_href {
            for file_path in extracted_files {
                if let Some(filename) = Path::new(file_path).file_name()
                    && filename.to_string_lossy().contains(file_name)
                {
                    return Some(file_path.clone());
                }
            }
        }

        for file_path in extracted_files {
            if let Some(filename) = Path::new(file_path).file_name() {
                let filename_str = filename.to_string_lossy().to_lowercase();
                let figure_id_lower = figure.id.to_lowercase();

                if filename_str.contains(&figure_id_lower)
                    && let Some(extension) = Path::new(file_path).extension()
                {
                    let ext_str = extension.to_string_lossy().to_lowercase();
                    if image_extensions.contains(&ext_str.as_str()) {
                        return Some(file_path.clone());
                    }
                }
            }
        }

        if let Some(label) = &figure.label {
            let label_clean = label.to_lowercase().replace([' ', '.'], "");
            for file_path in extracted_files {
                if let Some(filename) = Path::new(file_path).file_name() {
                    let filename_str = filename.to_string_lossy().to_lowercase();
                    if filename_str.contains(&label_clean)
                        && let Some(extension) = Path::new(file_path).extension()
                    {
                        let ext_str = extension.to_string_lossy().to_lowercase();
                        if image_extensions.contains(&ext_str.as_str()) {
                            return Some(file_path.clone());
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

        let tar_file = File::open(tar_path).map_err(|e| ParseError::IoError {
            message: format!("Failed to open tar.gz file: {}", e),
        })?;

        let tar_gz = GzDecoder::new(tar_file);
        let mut archive = Archive::new(tar_gz);

        let mut extracted_files = Vec::new();

        for entry in archive.entries().map_err(|e| ParseError::IoError {
            message: format!("Failed to read tar entries: {}", e),
        })? {
            let mut entry = entry.map_err(|e| ParseError::IoError {
                message: format!("Failed to read tar entry: {}", e),
            })?;

            let path = entry.path().map_err(|e| ParseError::IoError {
                message: format!("Failed to get entry path: {}", e),
            })?;

            let output_path = output_dir.join(&path);

            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(|e| ParseError::IoError {
                    message: format!("Failed to create parent directories: {}", e),
                })?;
            }

            entry
                .unpack(&output_path)
                .map_err(|e| ParseError::IoError {
                    message: format!("Failed to extract entry: {}", e),
                })?;

            extracted_files.push(output_path.to_string_lossy().to_string());
            debug!("Extracted: {}", output_path.display());
        }

        Ok(extracted_files)
    }

    fn executor(&self) -> RequestExecutor<'_> {
        RequestExecutor::new(&self.client, &self.rate_limiter, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pmcid() {
        assert_eq!(common::normalize_pmcid("1234567"), "PMC1234567");
        assert_eq!(common::normalize_pmcid("PMC1234567"), "PMC1234567");
    }

    #[test]
    fn test_client_creation() {
        let config = ClientConfig::new();
        let _client = PmcTarClient::new(config);
    }

    #[test]
    fn test_with_shared_creation() {
        let config = ClientConfig::new();
        let rate_limiter = config.create_rate_limiter();
        let client = Client::new();
        let _tar_client = PmcTarClient::with_shared(client, rate_limiter, config);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_oa_package_url_to_https_rewrites_to_deprecated() {
        // Legacy OA package links must be rewritten to the deprecated HTTPS path,
        // since NCBI moved the bundles there ahead of the August 2026 removal.
        let ftp = "ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_package/f1/69/PMC7906746.tar.gz";
        assert_eq!(
            PmcTarClient::oa_package_url_to_https(ftp),
            "https://ftp.ncbi.nlm.nih.gov/pub/pmc/deprecated/oa_package/f1/69/PMC7906746.tar.gz"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_oa_package_url_to_https_idempotent_on_deprecated() {
        let already =
            "ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/deprecated/oa_package/f1/69/PMC7906746.tar.gz";
        assert_eq!(
            PmcTarClient::oa_package_url_to_https(already),
            "https://ftp.ncbi.nlm.nih.gov/pub/pmc/deprecated/oa_package/f1/69/PMC7906746.tar.gz"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_oa_package_url_to_https_other_ftp_host_swap() {
        let other = "ftp://ftp.ncbi.nlm.nih.gov/other/path/file.tar.gz";
        assert_eq!(
            PmcTarClient::oa_package_url_to_https(other),
            "https://ftp.ncbi.nlm.nih.gov/other/path/file.tar.gz"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_oa_package_url_to_https_passthrough_non_ftp() {
        let https = "https://example.com/file.tar.gz";
        assert_eq!(PmcTarClient::oa_package_url_to_https(https), https);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_parse_cloud_listing_extracts_keys() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>pmc-oa-opendata</Name><Prefix>PMC7906746.</Prefix><KeyCount>5</KeyCount>
<Contents><Key>PMC7906746.1/PMC7906746.1.json</Key><Size>1</Size></Contents>
<Contents><Key>PMC7906746.1/PMC7906746.1.xml</Key><Size>1</Size></Contents>
<Contents><Key>PMC7906746.1/gr1_lrg.jpg</Key><Size>1</Size></Contents>
</ListBucketResult>"#;

        let keys = PmcTarClient::parse_cloud_listing(xml).unwrap();
        assert_eq!(
            keys,
            vec![
                "PMC7906746.1/PMC7906746.1.json".to_string(),
                "PMC7906746.1/PMC7906746.1.xml".to_string(),
                "PMC7906746.1/gr1_lrg.jpg".to_string(),
            ]
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_parse_cloud_listing_skips_folder_markers() {
        let xml = r#"<ListBucketResult><Contents><Key>PMC1.1/</Key></Contents><Contents><Key>PMC1.1/PMC1.1.xml</Key></Contents></ListBucketResult>"#;
        let keys = PmcTarClient::parse_cloud_listing(xml).unwrap();
        assert_eq!(keys, vec!["PMC1.1/PMC1.1.xml".to_string()]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_select_latest_version_keys_picks_highest() {
        let keys = vec![
            "PMC1.1/PMC1.1.xml".to_string(),
            "PMC1.1/gr1.jpg".to_string(),
            "PMC1.2/PMC1.2.xml".to_string(),
            "PMC1.2/gr1.jpg".to_string(),
        ];
        let latest = PmcTarClient::select_latest_version_keys(keys);
        assert_eq!(
            latest,
            vec![
                "PMC1.2/PMC1.2.xml".to_string(),
                "PMC1.2/gr1.jpg".to_string(),
            ]
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_select_latest_version_keys_empty() {
        assert!(PmcTarClient::select_latest_version_keys(vec![]).is_empty());
    }
}
