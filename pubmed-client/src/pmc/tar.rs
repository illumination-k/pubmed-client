use std::{path::Path, time::Duration};

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
use tracing::debug;

#[cfg(not(target_arch = "wasm32"))]
use tokio::{fs as tokio_fs, task};

/// Download client for PMC Open Access articles via the PMC OA Cloud (AWS S3).
///
/// Fetches an article's full-text XML, media, and supplementary files as
/// individual per-article objects from the `pmc-oa-opendata` S3 bucket. This
/// replaces the retired PMC FTP service and its legacy `oa_package` tar.gz
/// bundles (removed by NCBI in August 2026). The struct name is retained for
/// backwards compatibility.
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

    /// Download a PMC article's files from the PMC OA Cloud (AWS S3) service.
    ///
    /// NCBI retired the PMC FTP service and the legacy `oa_package` tar.gz
    /// bundles (August 2026). This downloads each of the article's files
    /// (full-text XML, media, supplementary materials, PDF, etc.) individually
    /// from the `pmc-oa-opendata` S3 bucket into `output_dir`.
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
    /// * `ParseError::PmcNotAvailable` - If the article is not available in the OA Cloud
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
    ///         println!("Downloaded: {}", file);
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

        let files = self
            .download_cloud_files(&normalized_pmcid, output_path)
            .await?;

        if files.is_empty() {
            return Err(ParseError::PmcNotAvailable {
                id: pmcid.to_string(),
            }
            .into());
        }

        Ok(files)
    }

    /// Download an article's files from the PMC OA Cloud (AWS S3) service.
    ///
    /// Lists the objects under the article's prefix in the `pmc-oa-opendata`
    /// bucket, selects the latest version folder, and downloads each file into
    /// `output_dir`. Returns the list of local file paths (empty if the article
    /// is not present in the cloud bucket).
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

    /// Find a matching file for a figure based on ID, label, or filename patterns.
    ///
    /// Three rules are tried in order, returning the first extracted file that matches:
    /// 1. the explicit `graphic_href` (case-sensitive substring of the file name, any extension);
    /// 2. the figure `id` (case-insensitive substring) with an image extension;
    /// 3. the figure `label` with whitespace/dots stripped (case-insensitive) with an image extension.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn find_matching_file(
        figure: &Figure,
        extracted_files: &[String],
        image_extensions: &[&str],
    ) -> Option<String> {
        // Rule 1: match by explicit graphic href. Case-sensitive and does not
        // require an image extension, mirroring the original behavior.
        if let Some(file_name) = &figure.graphic_href
            && let Some(matched) =
                Self::find_first_file(extracted_files, false, image_extensions, |filename| {
                    filename.contains(file_name.as_str())
                })
        {
            return Some(matched);
        }

        // Rule 2: match by figure id (case-insensitive) with an image extension.
        let figure_id_lower = figure.id.to_lowercase();
        if let Some(matched) =
            Self::find_first_file(extracted_files, true, image_extensions, |filename| {
                filename.to_lowercase().contains(&figure_id_lower)
            })
        {
            return Some(matched);
        }

        // Rule 3: match by label (whitespace/dots stripped) with an image extension.
        if let Some(label) = &figure.label {
            let label_clean = label.to_lowercase().replace([' ', '.'], "");
            if let Some(matched) =
                Self::find_first_file(extracted_files, true, image_extensions, |filename| {
                    filename.to_lowercase().contains(&label_clean)
                })
            {
                return Some(matched);
            }
        }

        None
    }

    /// Return the first extracted file whose file name satisfies `predicate`.
    ///
    /// When `require_image_ext` is true, the file must additionally have an
    /// extension (case-insensitive) present in `image_extensions`. The predicate
    /// receives the raw (non-lower-cased) file name so callers control casing.
    #[cfg(not(target_arch = "wasm32"))]
    fn find_first_file(
        extracted_files: &[String],
        require_image_ext: bool,
        image_extensions: &[&str],
        predicate: impl Fn(&str) -> bool,
    ) -> Option<String> {
        for file_path in extracted_files {
            let path = Path::new(file_path);
            let Some(filename) = path.file_name() else {
                continue;
            };
            if !predicate(&filename.to_string_lossy()) {
                continue;
            }
            if require_image_ext && !Self::has_image_extension(path, image_extensions) {
                continue;
            }
            return Some(file_path.clone());
        }
        None
    }

    /// Whether `path` has an extension (case-insensitive) in `image_extensions`.
    #[cfg(not(target_arch = "wasm32"))]
    fn has_image_extension(path: &Path, image_extensions: &[&str]) -> bool {
        path.extension()
            .map(|ext| image_extensions.contains(&ext.to_string_lossy().to_lowercase().as_str()))
            .unwrap_or(false)
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

    #[cfg(not(target_arch = "wasm32"))]
    fn figure(id: &str, label: Option<&str>, graphic_href: Option<&str>) -> Figure {
        Figure {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            caption: None,
            alt_text: None,
            fig_type: None,
            graphic_href: graphic_href.map(|s| s.to_string()),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "tif", "tiff"];

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_find_matching_file_by_graphic_href() {
        let files = vec![
            "PMC1/PMC1.xml".to_string(),
            "PMC1/gr1_lrg.jpg".to_string(),
            "PMC1/fig2.png".to_string(),
        ];
        // graphic_href match is a case-sensitive substring and ignores extension.
        let fig = figure("fig-1", None, Some("gr1_lrg.jpg"));
        assert_eq!(
            PmcTarClient::find_matching_file(&fig, &files, IMAGE_EXTS),
            Some("PMC1/gr1_lrg.jpg".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_find_matching_file_by_figure_id() {
        let files = vec!["PMC1/PMC1.xml".to_string(), "PMC1/GR1.PNG".to_string()];
        // id match is case-insensitive and requires an image extension.
        let fig = figure("gr1", None, None);
        assert_eq!(
            PmcTarClient::find_matching_file(&fig, &files, IMAGE_EXTS),
            Some("PMC1/GR1.PNG".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_find_matching_file_by_label() {
        let files = vec!["PMC1/PMC1.xml".to_string(), "PMC1/figure1.jpg".to_string()];
        // label match strips spaces/dots and is case-insensitive: "Figure 1." -> "figure1".
        let fig = figure("unrelated-id", Some("Figure 1."), None);
        assert_eq!(
            PmcTarClient::find_matching_file(&fig, &files, IMAGE_EXTS),
            Some("PMC1/figure1.jpg".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_find_matching_file_id_requires_image_extension() {
        // A non-image file whose name contains the id must not match rule 2.
        let files = vec!["PMC1/gr1.xml".to_string()];
        let fig = figure("gr1", None, None);
        assert_eq!(
            PmcTarClient::find_matching_file(&fig, &files, IMAGE_EXTS),
            None
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_find_matching_file_no_match() {
        let files = vec!["PMC1/other.jpg".to_string()];
        let fig = figure("gr9", Some("Figure 9"), Some("missing.png"));
        assert_eq!(
            PmcTarClient::find_matching_file(&fig, &files, IMAGE_EXTS),
            None
        );
    }
}
