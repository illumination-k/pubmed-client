use std::{mem, path::Path, time::Duration};

use crate::common::PmcId;
use crate::config::ClientConfig;
use crate::error::{ParseError, PubMedError, Result};
use crate::pmc::common;
use crate::pmc::extracted::{ExtractedFigure, FigureBlob, FigureSelection};
use crate::pmc::parser::parse_pmc_xml;
use crate::rate_limit::RateLimiter;
use crate::request::RequestExecutor;
#[cfg(not(target_arch = "wasm32"))]
use crate::request::fetch_with_retry;
use crate::tls::install_default_crypto_provider;
use pubmed_parser::pmc::{Figure, PmcArticle, Section};
use reqwest::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::Response;
use tracing::debug;

/// File extensions a figure's graphic may use in the OA Cloud package.
///
/// Matching is case-insensitive; `pdf` and `eps` are included because some
/// journals deposit vector figures rather than rasters.
#[cfg(not(target_arch = "wasm32"))]
const FIGURE_IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
];

#[cfg(not(target_arch = "wasm32"))]
use futures_util::{StreamExt, TryStreamExt, stream};
#[cfg(not(target_arch = "wasm32"))]
use tokio::{fs as tokio_fs, task};

/// Download client for PMC Open Access articles via the PMC OA Cloud (AWS S3).
///
/// Fetches an article's full-text XML, media, and supplementary files as
/// individual per-article objects from the `pmc-oa-opendata` S3 bucket. This
/// replaces the retired PMC FTP service and its legacy `oa_package` tar.gz
/// bundles (removed by NCBI in August 2026).
#[derive(Clone)]
pub struct PmcCloudClient {
    client: Client,
    rate_limiter: RateLimiter,
    pub(crate) config: ClientConfig,
}

impl PmcCloudClient {
    /// Create a new PMC OA Cloud client with configuration
    pub fn new(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();

        // rustls has no built-in provider under `rustls-tls`; install one first.
        install_default_crypto_provider();

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

    /// Create a cloud client sharing an existing HTTP client and rate limiter.
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
    /// use pubmed_client::pmc::cloud::PmcCloudClient;
    /// use pubmed_client::ClientConfig;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = ClientConfig::new();
    ///     let client = PmcCloudClient::new(config);
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

        let base_url = self
            .config
            .effective_oa_cloud_base_url()
            .trim_end_matches('/');
        let concurrency = self.config.effective_oa_download_concurrency();

        // The article's files are independent S3 objects and — unlike eutils —
        // the OA Cloud bucket is not subject to the NCBI rate limit, so we fetch
        // them concurrently (bounded by `concurrency`). `buffered` preserves the
        // listing order so figure matching stays deterministic.
        let downloaded = stream::iter(keys)
            .map(|key| async move {
                // The object filename is the last path segment of the S3 key,
                // e.g. `PMC7906746.1/gr1_lrg.jpg` -> `gr1_lrg.jpg`.
                let Some(file_name) = key.rsplit('/').next().filter(|s| !s.is_empty()) else {
                    return Ok::<Option<String>, PubMedError>(None);
                };

                let url = format!("{}/{}", base_url, key);
                let response = self.s3_get(&url).await?;
                let bytes = response.bytes().await.map_err(PubMedError::from)?;

                let output_path = output_dir.join(file_name);
                tokio_fs::write(&output_path, &bytes)
                    .await
                    .map_err(|e| ParseError::IoError {
                        message: format!("Failed to write cloud file {}: {}", file_name, e),
                    })?;

                debug!("Downloaded cloud file: {}", output_path.display());
                Ok(Some(output_path.to_string_lossy().to_string()))
            })
            .buffered(concurrency)
            .try_filter_map(|opt| async move { Ok(opt) })
            .try_collect::<Vec<String>>()
            .await?;

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
        let response = self.s3_get(&url).await?;
        let body = response.text().await?;

        let keys = Self::parse_cloud_listing(&body)?;
        Ok(Self::select_latest_version_keys(keys))
    }

    /// Parse the `<Key>` entries from an S3 ListObjectsV2 XML response.
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_cloud_listing(xml_content: &str) -> Result<Vec<String>> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        use quick_xml::escape::resolve_predefined_entity;

        let mut reader = Reader::from_str(xml_content);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut keys = Vec::new();
        let mut in_key = false;
        // A key containing an escaped character (e.g. `&amp;`) arrives as
        // multiple Text/GeneralRef events, so accumulate until </Key>.
        let mut key = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"Key" => {
                    in_key = true;
                    key.clear();
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"Key" => {
                    in_key = false;
                    // Skip folder-marker keys (zero-byte objects ending in `/`).
                    if !key.is_empty() && !key.ends_with('/') {
                        keys.push(mem::take(&mut key));
                    }
                }
                Ok(Event::Text(ref e)) if in_key => {
                    let text = e.decode().map_err(|err| {
                        ParseError::XmlError(format!("Invalid UTF-8 in S3 Key: {}", err))
                    })?;
                    key.push_str(&text);
                }
                Ok(Event::GeneralRef(ref e)) if in_key => {
                    let char_ref = e.resolve_char_ref().map_err(|err| {
                        ParseError::XmlError(format!("Invalid reference in S3 Key: {}", err))
                    })?;
                    if let Some(ch) = char_ref {
                        key.push(ch);
                    } else {
                        let name = e.decode().map_err(|err| {
                            ParseError::XmlError(format!("Invalid UTF-8 in S3 Key: {}", err))
                        })?;
                        if let Some(text) = resolve_predefined_entity(&name) {
                            key.push_str(text);
                        }
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
    /// use pubmed_client::pmc::cloud::PmcCloudClient;
    /// use pubmed_client::ClientConfig;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = ClientConfig::new();
    ///     let client = PmcCloudClient::new(config);
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

        // Download the article's OA package once. It already contains the JATS
        // full-text XML, so we parse that rather than issuing a second,
        // rate-limited eutils fetch of the same document.
        let extracted_files = self.download_files(&normalized_pmcid, &output_dir).await?;

        let full_text = self
            .parse_article_xml(&normalized_pmcid, &extracted_files)
            .await?;

        let figures = self
            .match_figures_with_files(&full_text, &extracted_files, &output_dir)
            .await?;

        Ok(figures)
    }

    /// Fetch an article's figures as in-memory blobs, without writing to disk.
    ///
    /// Unlike [`extract_figures_with_captions`], which downloads the whole OA
    /// package into a directory, this lists the article's objects, fetches only
    /// the JATS XML plus the images its `<fig>` elements resolve to, and returns
    /// the bytes. Nothing touches the filesystem, so it is usable from a server
    /// that has no writable directory to offer.
    ///
    /// [`extract_figures_with_captions`]: Self::extract_figures_with_captions
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    ///
    /// # Errors
    ///
    /// * `ParseError::InvalidPmcid` - If the PMCID format is invalid
    /// * `PubMedError::RequestError` - If an HTTP request fails
    /// * `ParseError::PmcNotAvailable` - If the article is not in the OA Cloud
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::pmc::cloud::PmcCloudClient;
    /// use pubmed_client::ClientConfig;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcCloudClient::new(ClientConfig::new());
    ///     for blob in client.fetch_figures("PMC7906746").await? {
    ///         println!("{} ({}, {} bytes)", blob.file_name, blob.content_type, blob.data.len());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn fetch_figures(&self, pmcid: &str) -> Result<Vec<FigureBlob>> {
        self.fetch_figures_with(pmcid, &FigureSelection::new())
            .await
    }

    /// Fetch a chosen subset of an article's figures as in-memory blobs.
    ///
    /// See [`FigureSelection`] for how figures are named and capped. The
    /// selection is resolved against the article's JATS XML *before* any image
    /// is downloaded, so a `limit` bounds the bytes fetched, not just the bytes
    /// returned.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::pmc::{FigureSelection, cloud::PmcCloudClient};
    /// use pubmed_client::ClientConfig;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcCloudClient::new(ClientConfig::new());
    ///     let selection = FigureSelection::new().with_ids(["Figure 1"]);
    ///     let blobs = client.fetch_figures_with("PMC7906746", &selection).await?;
    ///     println!("{} figure(s)", blobs.len());
    ///     Ok(())
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn fetch_figures_with(
        &self,
        pmcid: &str,
        selection: &FigureSelection,
    ) -> Result<Vec<FigureBlob>> {
        let pmc_id = PmcId::parse(pmcid)?;
        let normalized_pmcid = pmc_id.as_str();

        let keys = self.list_cloud_object_keys(&normalized_pmcid).await?;
        if keys.is_empty() {
            return Err(ParseError::PmcNotAvailable {
                id: pmcid.to_string(),
            }
            .into());
        }

        let article = self.fetch_article_xml(&normalized_pmcid, &keys).await?;

        let mut figures = Vec::new();
        for section in article.sections() {
            Self::collect_figures_recursive(section, &mut figures);
        }
        if !selection.ids.is_empty() {
            figures.retain(|figure| Self::figure_matches_any_id(figure, &selection.ids));
        }

        // Resolve each figure to its object key first, so only the images an
        // article actually references are downloaded — an OA package also holds
        // the PDF and supplementary files, which a figure request never wants.
        // The limit is applied here, before any byte is fetched.
        let mut wanted: Vec<(Figure, String)> = figures
            .into_iter()
            .filter_map(|figure| {
                Self::find_matching_file(&figure, &keys, FIGURE_IMAGE_EXTENSIONS)
                    .map(|key| (figure, key))
            })
            .collect();
        if let Some(limit) = selection.limit {
            wanted.truncate(limit);
        }

        let base_url = self
            .config
            .effective_oa_cloud_base_url()
            .trim_end_matches('/');
        let concurrency = self.config.effective_oa_download_concurrency();

        // Same reasoning as `download_cloud_files`: the OA Cloud bucket is not
        // under the NCBI rate limit, and `buffered` keeps document order.
        stream::iter(wanted)
            .map(|(figure, key)| async move {
                let file_name = key.rsplit('/').next().unwrap_or(key.as_str()).to_string();
                let url = format!("{}/{}", base_url, key);
                let response = self.s3_get(&url).await?;
                let data = response.bytes().await.map_err(PubMedError::from)?.to_vec();
                let dimensions = Self::blob_dimensions(&data);

                debug!("Fetched figure blob: {} ({} bytes)", file_name, data.len());
                Ok::<FigureBlob, PubMedError>(FigureBlob {
                    content_type: FigureBlob::guess_content_type(&file_name),
                    file_name,
                    data,
                    dimensions,
                    figure,
                })
            })
            .buffered(concurrency)
            .try_collect()
            .await
    }

    /// Whether `figure` is selected by any of the caller-supplied ids.
    ///
    /// Both the figure id and its label are normalized the same way (lowercased
    /// with spaces and dots removed), so `"Figure 1."`, `"figure1"` and the raw
    /// `"fig1"` id all select the same figure.
    #[cfg(not(target_arch = "wasm32"))]
    fn figure_matches_any_id(figure: &Figure, ids: &[String]) -> bool {
        fn normalize(value: &str) -> String {
            value.to_lowercase().replace([' ', '.'], "")
        }

        let id = normalize(&figure.id);
        let label = figure.label.as_deref().map(normalize);

        ids.iter()
            .map(|wanted| normalize(wanted))
            .any(|wanted| wanted == id || label.as_deref().is_some_and(|label| label == wanted))
    }

    /// Read image dimensions from bytes already in memory.
    ///
    /// Header-only, like [`get_image_dimensions`]; formats `imagesize` does not
    /// recognize (`svg`, `eps`, `pdf`) yield `None`.
    ///
    /// [`get_image_dimensions`]: Self::get_image_dimensions
    #[cfg(not(target_arch = "wasm32"))]
    fn blob_dimensions(data: &[u8]) -> Option<(u32, u32)> {
        let size = imagesize::blob_size(data).ok()?;
        Some((
            u32::try_from(size.width).ok()?,
            u32::try_from(size.height).ok()?,
        ))
    }

    /// Fetch and parse the article's JATS XML straight from the OA Cloud.
    ///
    /// `keys` are the article's S3 object keys; the XML is downloaded on its own
    /// rather than as part of a full package download. Falls back to eutils in
    /// the unexpected case where the listing holds no XML, matching
    /// [`parse_article_xml`]'s behavior.
    ///
    /// [`parse_article_xml`]: Self::parse_article_xml
    #[cfg(not(target_arch = "wasm32"))]
    async fn fetch_article_xml(
        &self,
        normalized_pmcid: &str,
        keys: &[String],
    ) -> Result<PmcArticle> {
        if let Some(key) = Self::find_article_xml(keys, normalized_pmcid) {
            let base_url = self
                .config
                .effective_oa_cloud_base_url()
                .trim_end_matches('/');
            let xml_content = self
                .s3_get(&format!("{}/{}", base_url, key))
                .await?
                .text()
                .await?;
            return Ok(parse_pmc_xml(&xml_content, normalized_pmcid)?);
        }

        debug!(
            pmcid = %normalized_pmcid,
            "OA Cloud listing had no XML; falling back to eutils fetch"
        );
        let xml_content = common::fetch_pmc_xml(
            &self.executor(),
            self.config.effective_base_url(),
            normalized_pmcid,
        )
        .await?;
        Ok(parse_pmc_xml(&xml_content, normalized_pmcid)?)
    }

    /// Parse the article's JATS XML, preferring the copy already downloaded from
    /// the OA Cloud so no redundant eutils request is made.
    ///
    /// OA packages always include the full-text XML; the eutils fallback only
    /// triggers in the unexpected case where the downloaded files contain no
    /// `.xml`, so behavior never regresses relative to the previous eutils-only
    /// path.
    #[cfg(not(target_arch = "wasm32"))]
    async fn parse_article_xml(
        &self,
        normalized_pmcid: &str,
        extracted_files: &[String],
    ) -> Result<PmcArticle> {
        if let Some(xml_path) = Self::find_article_xml(extracted_files, normalized_pmcid) {
            let xml_content =
                tokio_fs::read_to_string(&xml_path)
                    .await
                    .map_err(|e| ParseError::IoError {
                        message: format!("Failed to read downloaded XML {}: {}", xml_path, e),
                    })?;
            return Ok(parse_pmc_xml(&xml_content, normalized_pmcid)?);
        }

        debug!(
            pmcid = %normalized_pmcid,
            "OA Cloud package had no XML; falling back to eutils fetch"
        );
        let xml_content = common::fetch_pmc_xml(
            &self.executor(),
            self.config.effective_base_url(),
            normalized_pmcid,
        )
        .await?;
        Ok(parse_pmc_xml(&xml_content, normalized_pmcid)?)
    }

    /// Find the article's JATS XML among an OA package's entries.
    ///
    /// The JATS file is named `<PMCID>.<version>.xml`, so we match on a file
    /// name that ends in `.xml` and contains the PMCID (case-insensitive).
    /// Entries may be local file paths (after a download) or S3 object keys
    /// (before one) — only the last path segment is inspected either way.
    #[cfg(not(target_arch = "wasm32"))]
    fn find_article_xml(entries: &[String], normalized_pmcid: &str) -> Option<String> {
        let pmcid_lower = normalized_pmcid.to_lowercase();
        entries
            .iter()
            .find(|path| {
                let name = Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                name.ends_with(".xml") && name.contains(&pmcid_lower)
            })
            .cloned()
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

        for figure in all_figures {
            let matching_file =
                Self::find_matching_file(&figure, extracted_files, FIGURE_IMAGE_EXTENSIONS);

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

    /// Read image dimensions from the file header.
    ///
    /// `imagesize` parses only the header, so this never decodes pixel data.
    /// Formats it does not recognize (`svg`, `eps`, `pdf` among the extensions
    /// figures are matched on) simply yield `None`.
    #[cfg(not(target_arch = "wasm32"))]
    async fn get_image_dimensions(file_path: &str) -> Option<(u32, u32)> {
        task::spawn_blocking({
            let file_path = file_path.to_string();
            move || {
                let size = imagesize::size(&file_path).ok()?;
                Some((
                    u32::try_from(size.width).ok()?,
                    u32::try_from(size.height).ok()?,
                ))
            }
        })
        .await
        .ok()
        .flatten()
    }

    fn executor(&self) -> RequestExecutor<'_> {
        RequestExecutor::new(&self.client, &self.rate_limiter, &self.config)
    }

    /// GET a PMC OA Cloud (AWS S3) URL.
    ///
    /// The `pmc-oa-opendata` bucket is AWS Open Data, not an NCBI E-utilities
    /// endpoint, so these requests are **not** rate-limited by the NCBI quota —
    /// their parallelism is bounded by [`ClientConfig::effective_oa_download_concurrency`]
    /// instead. Retry and status-aware error mapping still apply.
    #[cfg(not(target_arch = "wasm32"))]
    async fn s3_get(&self, url: &str) -> Result<Response> {
        debug!("Making PMC OA Cloud (S3) request to: {url}");
        fetch_with_retry(
            || self.client.get(url),
            &self.config.retry_config,
            None,
            "PMC OA Cloud request",
        )
        .await
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
        let _client = PmcCloudClient::new(config);
    }

    #[test]
    fn test_with_shared_creation() {
        let config = ClientConfig::new();
        let rate_limiter = config.create_rate_limiter();
        let client = Client::new();
        let _cloud_client = PmcCloudClient::with_shared(client, rate_limiter, config);
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

        let keys = PmcCloudClient::parse_cloud_listing(xml).unwrap();
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
        let keys = PmcCloudClient::parse_cloud_listing(xml).unwrap();
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
        let latest = PmcCloudClient::select_latest_version_keys(keys);
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
    fn test_find_article_xml() {
        let files = vec![
            "/tmp/PMC9991720/gr1_lrg.jpg".to_string(),
            "/tmp/PMC9991720/PMC9991720.1.xml".to_string(),
            "/tmp/PMC9991720/PMC9991720.1.json".to_string(),
        ];
        assert_eq!(
            PmcCloudClient::find_article_xml(&files, "PMC9991720"),
            Some("/tmp/PMC9991720/PMC9991720.1.xml".to_string())
        );
        // Case-insensitive on both the file name and the PMCID.
        assert_eq!(
            PmcCloudClient::find_article_xml(&["/tmp/pmc9991720.1.XML".to_string()], "PMC9991720"),
            Some("/tmp/pmc9991720.1.XML".to_string())
        );
        // No XML present -> None (triggers the eutils fallback).
        assert_eq!(
            PmcCloudClient::find_article_xml(
                &["/tmp/PMC9991720/gr1.jpg".to_string()],
                "PMC9991720"
            ),
            None
        );
        // An unrelated XML must not match.
        assert_eq!(
            PmcCloudClient::find_article_xml(&["/tmp/PMC0000001.1.xml".to_string()], "PMC9991720"),
            None
        );
        // The same matching serves S3 object keys, which `fetch_figures` uses
        // to pull the XML before anything has been downloaded.
        assert_eq!(
            PmcCloudClient::find_article_xml(
                &["PMC9991720.1/PMC9991720.1.xml".to_string()],
                "PMC9991720"
            ),
            Some("PMC9991720.1/PMC9991720.1.xml".to_string())
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_select_latest_version_keys_empty() {
        assert!(PmcCloudClient::select_latest_version_keys(vec![]).is_empty());
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
            PmcCloudClient::find_matching_file(&fig, &files, IMAGE_EXTS),
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
            PmcCloudClient::find_matching_file(&fig, &files, IMAGE_EXTS),
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
            PmcCloudClient::find_matching_file(&fig, &files, IMAGE_EXTS),
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
            PmcCloudClient::find_matching_file(&fig, &files, IMAGE_EXTS),
            None
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_find_matching_file_no_match() {
        let files = vec!["PMC1/other.jpg".to_string()];
        let fig = figure("gr9", Some("Figure 9"), Some("missing.png"));
        assert_eq!(
            PmcCloudClient::find_matching_file(&fig, &files, IMAGE_EXTS),
            None
        );
    }

    /// A 3x2 PNG: signature + an IHDR chunk declaring the dimensions.
    #[cfg(not(target_arch = "wasm32"))]
    const PNG_3X2: &[u8] = &[
        0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, // signature
        0x00, 0x00, 0x00, 0x0d, b'I', b'H', b'D', b'R', // chunk length + type
        0x00, 0x00, 0x00, 0x03, // width  = 3
        0x00, 0x00, 0x00, 0x02, // height = 2
        0x08, 0x06, 0x00, 0x00, 0x00, // bit depth, color type, compression, filter, interlace
        0x00, 0x00, 0x00, 0x00, // CRC (not validated when reading the header)
    ];

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_image_dimensions_reads_png_header() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gr1.png");
        fs::write(&path, PNG_3X2).unwrap();

        assert_eq!(
            PmcCloudClient::get_image_dimensions(&path.to_string_lossy()).await,
            Some((3, 2))
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_get_image_dimensions_none_for_unsupported_and_missing() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // SVG / EPS / PDF are in the matched extension list but carry no
        // readable raster header, so they yield `None`.
        let svg = dir.path().join("gr1.svg");
        fs::write(&svg, b"<svg width=\"10\" height=\"20\"></svg>").unwrap();
        assert_eq!(
            PmcCloudClient::get_image_dimensions(&svg.to_string_lossy()).await,
            None
        );

        let missing = dir.path().join("does-not-exist.png");
        assert_eq!(
            PmcCloudClient::get_image_dimensions(&missing.to_string_lossy()).await,
            None
        );
    }
}
