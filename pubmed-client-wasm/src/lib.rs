//! WebAssembly bindings for the PubMed client library
//!
//! This module provides JavaScript-compatible bindings for use in Node.js and browsers.

use pubmed_client::{
    Client, config::ClientConfig, pmc::PmcArticle, pubmed::ArticleSummary, pubmed::PubMedArticle,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen]
extern "C" {
    /// `Promise<JsArticle[]>`
    pub type JsPromiseArticles;

    /// `Promise<JsArticle>`
    pub type JsPromiseArticle;

    /// `Promise<JsFullText>`
    pub type JsPromiseFullText;

    /// `Promise<string | null>`
    pub type JsPromiseOptString;

    /// `Promise<string[]>`
    pub type JsPromiseStringArray;

    /// `Promise<JsSummary[]>`
    pub type JsPromiseSummaries;
}

// Set up panic handler and allocator for better WASM experience
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();

    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
}

/// JavaScript-friendly configuration for the PubMed client
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct WasmClientConfig {
    api_key: Option<String>,
    email: Option<String>,
    tool: Option<String>,
    rate_limit: Option<f64>,
    timeout_seconds: Option<u64>,
}

#[wasm_bindgen]
impl WasmClientConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    #[wasm_bindgen(setter)]
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }

    #[wasm_bindgen(setter)]
    pub fn set_email(&mut self, email: String) {
        self.email = Some(email);
    }

    #[wasm_bindgen(setter)]
    pub fn set_tool(&mut self, tool: String) {
        self.tool = Some(tool);
    }

    #[wasm_bindgen(setter)]
    pub fn set_rate_limit(&mut self, rate_limit: f64) {
        self.rate_limit = Some(rate_limit);
    }

    #[wasm_bindgen(setter)]
    pub fn set_timeout_seconds(&mut self, timeout_seconds: u64) {
        self.timeout_seconds = Some(timeout_seconds);
    }
}

impl From<WasmClientConfig> for ClientConfig {
    fn from(wasm_config: WasmClientConfig) -> Self {
        let mut config = ClientConfig::new();

        if let Some(api_key) = wasm_config.api_key {
            config = config.with_api_key(&api_key);
        }

        if let Some(email) = wasm_config.email {
            config = config.with_email(&email);
        }

        if let Some(tool) = wasm_config.tool {
            config = config.with_tool(&tool);
        }

        if let Some(rate_limit) = wasm_config.rate_limit {
            config = config.with_rate_limit(rate_limit);
        }

        if let Some(timeout_seconds) = wasm_config.timeout_seconds {
            config = config.with_timeout_seconds(timeout_seconds);
        }

        config
    }
}

// ================================================================================================
// Error Conversion
// ================================================================================================

/// Convert `PubMedError` to a `JsValue` carrying a proper `js_sys::Error` with a
/// `type` property that consumers can switch on (`"ParseError"`, `"RateLimitExceeded"`, etc.).
///
/// The match is exhaustive — adding a new `PubMedError` variant will fail to compile
/// until an explicit mapping is added here.
fn to_js_err(err: pubmed_client::error::PubMedError) -> JsValue {
    use pubmed_client::error::PubMedError;
    let (error_type, message) = match &err {
        PubMedError::ParseError(_) => ("ParseError", err.to_string()),
        PubMedError::RequestError(_) => ("RequestError", err.to_string()),
        PubMedError::InvalidQuery(_) => ("InvalidQuery", err.to_string()),
        PubMedError::RateLimitExceeded => ("RateLimitExceeded", err.to_string()),
        PubMedError::ApiError { .. } => ("ApiError", err.to_string()),
        PubMedError::SearchLimitExceeded { .. } => ("SearchLimitExceeded", err.to_string()),
        PubMedError::HistorySessionError(_) => ("HistorySessionError", err.to_string()),
        PubMedError::WebEnvNotAvailable => ("WebEnvNotAvailable", err.to_string()),
    };
    let js_error = js_sys::Error::new(&message);
    let _ = js_sys::Reflect::set(
        &js_error,
        &JsValue::from_str("type"),
        &JsValue::from_str(error_type),
    );
    js_error.into()
}

/// JavaScript-friendly wrapper for the PubMed client
#[wasm_bindgen]
pub struct WasmPubMedClient {
    client: Client,
}

impl Default for WasmPubMedClient {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmPubMedClient {
    /// Create a new client with default configuration
    /// Uses a conservative rate limit of 1 request per second for testing
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Use a conservative rate limit for WASM environments to avoid 429 errors
        let config = ClientConfig::new()
            .with_rate_limit(1.0) // 1 request per second
            .with_tool("pubmed-client-wasm");

        Self {
            client: Client::with_config(config),
        }
    }

    /// Create a new client with custom configuration
    #[wasm_bindgen]
    pub fn with_config(config: WasmClientConfig) -> Self {
        let client_config: ClientConfig = config.into();
        Self {
            client: Client::with_config(client_config),
        }
    }

    /// Create a new client optimized for testing with very conservative rate limits
    #[wasm_bindgen]
    pub fn new_for_testing() -> Self {
        let config = ClientConfig::new()
            .with_rate_limit(0.5) // 0.5 requests per second (1 request every 2 seconds)
            .with_tool("pubmed-client-wasm-test");

        Self {
            client: Client::with_config(config),
        }
    }

    /// Search for articles and return a Promise
    pub fn search_articles(&self, query: String, limit: usize) -> JsPromiseArticles {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pubmed.search_and_fetch(&query, limit, None).await {
                Ok(articles) => {
                    let js_articles: Vec<JsArticle> =
                        articles.into_iter().map(JsArticle::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_articles)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Fetch multiple articles by PMIDs in a single batch request
    ///
    /// More efficient than fetching one by one. Automatically batches large requests.
    pub fn fetch_articles(&self, pmids: Vec<String>) -> JsPromiseArticles {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.fetch_articles(&pmid_refs).await {
                Ok(articles) => {
                    let js_articles: Vec<JsArticle> =
                        articles.into_iter().map(JsArticle::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_articles)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Fetch a single article by PMID
    pub fn fetch_article(&self, pmid: String) -> JsPromiseArticle {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pubmed.fetch_article(&pmid).await {
                Ok(article) => {
                    let js_article = JsArticle::from(article);
                    Ok(serde_wasm_bindgen::to_value(&js_article)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Fetch lightweight article summaries by PMIDs using the ESummary API
    pub fn fetch_summaries(&self, pmids: Vec<String>) -> JsPromiseSummaries {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.fetch_summaries(&pmid_refs).await {
                Ok(summaries) => {
                    let js_summaries: Vec<JsSummary> =
                        summaries.into_iter().map(JsSummary::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_summaries)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Search PubMed and fetch lightweight summaries
    pub fn search_summaries(&self, query: String, limit: usize) -> JsPromiseSummaries {
        let client = self.client.clone();
        future_to_promise(async move {
            match client
                .pubmed
                .search_and_fetch_summaries(&query, limit, None)
                .await
            {
                Ok(summaries) => {
                    let js_summaries: Vec<JsSummary> =
                        summaries.into_iter().map(JsSummary::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_summaries)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Fetch full text from PMC
    pub fn fetch_full_text(&self, pmcid: String) -> JsPromiseFullText {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pmc.fetch_full_text(&pmcid).await {
                Ok(full_text) => {
                    let js_full_text = JsFullText::from(full_text);
                    Ok(serde_wasm_bindgen::to_value(&js_full_text)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Check if PMC full text is available for a PMID
    pub fn check_pmc_availability(&self, pmid: String) -> JsPromiseOptString {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pmc.check_pmc_availability(&pmid).await {
                Ok(pmcid_opt) => Ok(serde_wasm_bindgen::to_value(&pmcid_opt)?),
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Get related articles for given PMIDs
    pub fn get_related_articles(&self, pmids: Vec<u32>) -> JsPromiseStringArray {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.get_related_articles(&pmids).await {
                Ok(related) => Ok(serde_wasm_bindgen::to_value(&related)?),
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Match citations to PMIDs using the ECitMatch API
    pub fn match_citations(&self, citations: JsValue) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            let citation_inputs: Vec<JsCitationQuery> =
                serde_wasm_bindgen::from_value(citations)
                    .map_err(|e| JsValue::from_str(&format!("Invalid citations data: {e}")))?;

            let rust_citations: Vec<pubmed_client::CitationQuery> = citation_inputs
                .iter()
                .map(|c| {
                    pubmed_client::CitationQuery::new(
                        &c.journal,
                        &c.year,
                        &c.volume,
                        &c.first_page,
                        &c.author_name,
                        &c.key,
                    )
                })
                .collect();

            match client.pubmed.match_citations(&rust_citations).await {
                Ok(results) => {
                    let js_results: Vec<JsCitationMatch> =
                        results.matches.iter().map(JsCitationMatch::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_results)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Query all NCBI databases for record counts
    pub fn global_query(&self, term: String) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pubmed.global_query(&term).await {
                Ok(results) => {
                    let js_results = JsGlobalQueryResults::from(results);
                    Ok(serde_wasm_bindgen::to_value(&js_results)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Upload a list of PMIDs to the NCBI History server using EPost
    ///
    /// Returns a Promise resolving to an object with `webenv` and `query_key` fields.
    pub fn epost(&self, pmids: Vec<String>) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.epost(&pmid_refs).await {
                Ok(result) => {
                    let js_result = JsEPostResult {
                        webenv: result.webenv,
                        query_key: result.query_key,
                    };
                    Ok(serde_wasm_bindgen::to_value(&js_result)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Fetch all articles for a list of PMIDs using EPost and the History server
    ///
    /// Uploads via EPost (HTTP POST), then fetches in paginated batches.
    /// Recommended for large PMID lists.
    pub fn fetch_all_by_pmids(&self, pmids: Vec<String>) -> JsPromiseArticles {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.fetch_all_by_pmids(&pmid_refs).await {
                Ok(articles) => {
                    let js_articles: Vec<JsArticle> =
                        articles.into_iter().map(JsArticle::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_articles)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Check spelling of a search term using the ESpell API (PubMed database)
    pub fn spell_check(&self, term: String) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pubmed.spell_check(&term).await {
                Ok(result) => {
                    let js_result = JsSpellCheckResult::from(result);
                    Ok(serde_wasm_bindgen::to_value(&js_result)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Check spelling of a search term against a specific database using the ESpell API
    pub fn spell_check_db(&self, term: String, db: String) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pubmed.spell_check_db(&term, &db).await {
                Ok(result) => {
                    let js_result = JsSpellCheckResult::from(result);
                    Ok(serde_wasm_bindgen::to_value(&js_result)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Get citing articles for given PMIDs
    pub fn get_citations(&self, pmids: Vec<u32>) -> JsPromiseStringArray {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.get_citations(&pmids).await {
                Ok(citations) => Ok(serde_wasm_bindgen::to_value(&citations)?),
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Get PMC links for given PMIDs (check full-text availability)
    pub fn get_pmc_links(&self, pmids: Vec<u32>) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.get_pmc_links(&pmids).await {
                Ok(links) => Ok(serde_wasm_bindgen::to_value(&links)?),
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// List all available NCBI databases
    pub fn get_database_list(&self) -> JsPromiseStringArray {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.get_database_list().await {
                Ok(databases) => Ok(serde_wasm_bindgen::to_value(&databases)?),
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }

    /// Get detailed information about a specific NCBI database
    pub fn get_database_info(&self, database: String) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.get_database_info(&database).await {
                Ok(info) => Ok(serde_wasm_bindgen::to_value(&info)?),
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Export articles as BibTeX format
    pub fn export_bibtex(&self, pmids: Vec<String>) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.fetch_articles(&pmid_refs).await {
                Ok(articles) => {
                    let bibtex = pubmed_client::export::articles_to_bibtex(&articles);
                    Ok(JsValue::from_str(&bibtex))
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Export articles as RIS format
    pub fn export_ris(&self, pmids: Vec<String>) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.fetch_articles(&pmid_refs).await {
                Ok(articles) => {
                    let ris = pubmed_client::export::articles_to_ris(&articles);
                    Ok(JsValue::from_str(&ris))
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Convert PMC full text to markdown
    pub fn convert_to_markdown(&self, full_text_js: JsValue) -> Result<String, JsValue> {
        let js_full_text: JsFullText = serde_wasm_bindgen::from_value(full_text_js)
            .map_err(|e| JsValue::from_str(&format!("Invalid full text data: {e}")))?;

        let full_text: PmcArticle = js_full_text.into();
        let converter = pubmed_client::pmc::PmcMarkdownConverter::new();
        Ok(converter.convert(&full_text))
    }

    /// Fetch a PMC article and convert it to markdown in a single call
    ///
    /// `options` is an optional object with boolean fields: `include_metadata`,
    /// `include_toc`, `use_yaml_frontmatter`, `include_orcid_links`,
    /// `include_figure_captions`. Unset fields fall back to converter defaults.
    pub fn fetch_pmc_as_markdown(&self, pmcid: String, options: JsValue) -> js_sys::Promise {
        let client = self.client.clone();
        // Parse options synchronously so a malformed value rejects eagerly.
        let parsed: Result<Option<JsMarkdownOptions>, _> =
            if options.is_undefined() || options.is_null() {
                Ok(None)
            } else {
                serde_wasm_bindgen::from_value(options).map(Some)
            };
        future_to_promise(async move {
            let options = parsed
                .map_err(|e| JsValue::from_str(&format!("Invalid markdown options: {e}")))?
                .unwrap_or_default();
            match client.pmc.fetch_full_text(&pmcid).await {
                Ok(full_text) => {
                    let mut converter = pubmed_client::pmc::PmcMarkdownConverter::new();
                    if let Some(v) = options.include_metadata {
                        converter = converter.with_include_metadata(v);
                    }
                    if let Some(v) = options.include_toc {
                        converter = converter.with_include_toc(v);
                    }
                    if let Some(v) = options.use_yaml_frontmatter {
                        converter = converter.with_yaml_frontmatter(v);
                    }
                    if let Some(v) = options.include_orcid_links {
                        converter = converter.with_include_orcid_links(v);
                    }
                    if let Some(v) = options.include_figure_captions {
                        converter = converter.with_include_figure_captions(v);
                    }
                    Ok(JsValue::from_str(&converter.convert(&full_text)))
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Check whether a PMC article is in the Open Access (OA) subset
    ///
    /// Resolves to a `JsOaSubsetInfo` object describing programmatic full-text
    /// availability, license, download link, and (when not available) error details.
    pub fn is_oa_subset(&self, pmcid: String) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            match client.pmc.is_oa_subset(&pmcid).await {
                Ok(info) => {
                    let js_info = JsOaSubsetInfo::from(info);
                    Ok(serde_wasm_bindgen::to_value(&js_info)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }
}

/// JavaScript-friendly markdown conversion options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsMarkdownOptions {
    #[serde(default)]
    pub include_metadata: Option<bool>,
    #[serde(default)]
    pub include_toc: Option<bool>,
    #[serde(default)]
    pub use_yaml_frontmatter: Option<bool>,
    #[serde(default)]
    pub include_orcid_links: Option<bool>,
    #[serde(default)]
    pub include_figure_captions: Option<bool>,
}

/// JavaScript-friendly OA (Open Access) subset availability information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsOaSubsetInfo {
    pub pmcid: String,
    pub is_oa_subset: bool,
    pub citation: Option<String>,
    pub license: Option<String>,
    pub retracted: bool,
    pub download_link: Option<String>,
    pub download_format: Option<String>,
    pub updated: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl From<pubmed_client::OaSubsetInfo> for JsOaSubsetInfo {
    fn from(info: pubmed_client::OaSubsetInfo) -> Self {
        Self {
            pmcid: info.pmcid,
            is_oa_subset: info.is_oa_subset,
            citation: info.citation,
            license: info.license,
            retracted: info.retracted,
            download_link: info.download_link,
            download_format: info.download_format,
            updated: info.updated,
            error_code: info.error_code,
            error_message: info.error_message,
        }
    }
}

/// JavaScript-friendly EPost result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsEPostResult {
    pub webenv: String,
    pub query_key: String,
}

/// JavaScript-friendly article representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsArticle {
    pub pmid: String,
    pub title: String,
    pub authors: Vec<String>,
    pub journal: String,
    pub pub_date: String,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub pmc_id: Option<String>,
    pub article_types: Vec<String>,
    pub keywords: Vec<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub language: Option<String>,
    pub journal_abbreviation: Option<String>,
    pub issn: Option<String>,
}

impl From<PubMedArticle> for JsArticle {
    fn from(article: PubMedArticle) -> Self {
        // Convert Author structs to simple strings
        let author_names: Vec<String> = article
            .authors
            .into_iter()
            .map(|author| author.full_name)
            .collect();

        Self {
            pmid: article.pmid,
            title: article.title,
            authors: author_names,
            journal: article.journal,
            pub_date: article.pub_date,
            abstract_text: article.abstract_text,
            doi: article.doi,
            pmc_id: article.pmc_id,
            article_types: article.article_types,
            keywords: article.keywords.unwrap_or_default(),
            volume: article.volume,
            issue: article.issue,
            pages: article.pages,
            language: article.language,
            journal_abbreviation: article.journal_abbreviation,
            issn: article.issn,
        }
    }
}

/// JavaScript-friendly lightweight article summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsSummary {
    pub pmid: String,
    pub title: String,
    pub authors: Vec<String>,
    pub journal: String,
    pub full_journal_name: String,
    pub pub_date: String,
    pub epub_date: String,
    pub doi: Option<String>,
    pub pmc_id: Option<String>,
    pub volume: String,
    pub issue: String,
    pub pages: String,
    pub languages: Vec<String>,
    pub pub_types: Vec<String>,
    pub issn: String,
    pub essn: String,
    pub sort_pub_date: String,
    pub pmc_ref_count: u64,
    pub record_status: String,
}

impl From<ArticleSummary> for JsSummary {
    fn from(s: ArticleSummary) -> Self {
        Self {
            pmid: s.pmid,
            title: s.title,
            authors: s.authors,
            journal: s.journal,
            full_journal_name: s.full_journal_name,
            pub_date: s.pub_date,
            epub_date: s.epub_date,
            doi: s.doi,
            pmc_id: s.pmc_id,
            volume: s.volume,
            issue: s.issue,
            pages: s.pages,
            languages: s.languages,
            pub_types: s.pub_types,
            issn: s.issn,
            essn: s.essn,
            sort_pub_date: s.sort_pub_date,
            pmc_ref_count: s.pmc_ref_count,
            record_status: s.record_status,
        }
    }
}

/// JavaScript-friendly full text representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsFullText {
    pub pmcid: String,
    pub pmid: Option<String>,
    pub title: Option<String>,
    pub authors: Vec<JsAuthor>,
    pub journal: JsJournal,
    pub pub_date: String,
    pub doi: Option<String>,
    pub sections: Vec<JsSection>,
    pub references: Vec<JsReference>,
    pub article_type: Option<String>,
    pub keywords: Vec<String>,
    pub funding: Vec<JsFunding>,
    pub conflict_of_interest: Option<String>,
    pub acknowledgments: Option<String>,
    pub data_availability: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsFunding {
    pub source: Option<String>,
    pub award_id: Option<String>,
    pub statement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsAuthor {
    pub given_names: Option<String>,
    pub surname: Option<String>,
    pub full_name: String,
    pub email: Option<String>,
    pub affiliations: Vec<String>,
    pub is_corresponding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsJournal {
    pub title: Option<String>,
    pub abbreviation: Option<String>,
    pub publisher: Option<String>,
    pub issn_print: Option<String>,
    pub issn_electronic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsSection {
    pub section_type: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub subsections: Vec<JsSection>,
    pub figures: Vec<JsFigure>,
    pub tables: Vec<JsTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsFigure {
    pub id: String,
    pub label: Option<String>,
    pub caption: Option<String>,
    pub graphic_href: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsTable {
    pub id: String,
    pub label: Option<String>,
    pub caption: Option<String>,
    pub footnotes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsReference {
    pub id: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub journal: Option<String>,
    pub year: Option<String>,
    pub pmid: Option<String>,
    pub doi: Option<String>,
}

impl From<PmcArticle> for JsFullText {
    fn from(article: PmcArticle) -> Self {
        let PmcArticle {
            article_type,
            front,
            body,
            back,
            data_availability,
            ..
        } = article;
        let journal_meta = front.journal_meta;
        let meta = front.article_meta;
        let sections = body.map(|b| b.sections).unwrap_or_default();
        let back = back.unwrap_or(pubmed_client::pmc::Back {
            acknowledgments: None,
            conflict_of_interest: None,
            references: Vec::new(),
            appendices: Vec::new(),
            glossary: Vec::new(),
        });
        Self {
            pmcid: meta.pmcid.to_string(),
            pmid: meta.pmid.map(|p| p.to_string()),
            title: meta.title_group.article_title,
            authors: meta.authors.into_iter().map(JsAuthor::from).collect(),
            journal: JsJournal::from(journal_meta),
            pub_date: meta
                .pub_dates
                .first()
                .map(|d| {
                    let mut s = String::new();
                    if let Some(y) = d.year {
                        s.push_str(&y.to_string());
                    }
                    if let Some(m) = d.month {
                        s.push_str(&format!("-{:02}", m));
                    }
                    if let Some(day) = d.day {
                        s.push_str(&format!("-{:02}", day));
                    }
                    s
                })
                .unwrap_or_default(),
            doi: meta.doi,
            sections: sections.into_iter().map(JsSection::from).collect(),
            references: back.references.into_iter().map(JsReference::from).collect(),
            article_type,
            keywords: meta.keywords,
            funding: meta
                .funding
                .into_iter()
                .map(|f| JsFunding {
                    source: f.source,
                    award_id: f.award_id,
                    statement: f.statement,
                })
                .collect(),
            conflict_of_interest: back.conflict_of_interest,
            acknowledgments: back.acknowledgments,
            data_availability,
        }
    }
}

impl From<JsFullText> for PmcArticle {
    fn from(js: JsFullText) -> Self {
        use pubmed_client::pmc::{ArticleMeta, Back, Body, Front, JournalMeta, TitleGroup};
        Self {
            article_type: js.article_type,
            front: Front {
                journal_meta: JournalMeta::from(js.journal),
                article_meta: ArticleMeta {
                    pmcid: pubmed_client::PmcId::parse(&js.pmcid)
                        .unwrap_or_else(|_| pubmed_client::PmcId::from_u32(1)),
                    pmid: js
                        .pmid
                        .and_then(|p| pubmed_client::PubMedId::parse(&p).ok()),
                    doi: js.doi,
                    categories: Vec::new(),
                    title_group: TitleGroup {
                        article_title: js.title,
                        subtitle: None,
                    },
                    authors: js.authors.into_iter().map(|a| a.into()).collect(),
                    pub_dates: Vec::new(),
                    volume: None,
                    issue: None,
                    fpage: None,
                    lpage: None,
                    elocation_id: None,
                    history: Vec::new(),
                    permissions: None,
                    abstracts: Vec::new(),
                    keywords: js.keywords,
                    keyword_groups: Vec::new(),
                    subject_groups: Vec::new(),
                    related_articles: Vec::new(),
                    author_notes: Vec::new(),
                    funding: js
                        .funding
                        .into_iter()
                        .map(|f| pubmed_client::pmc::FundingInfo {
                            source: f.source,
                            award_id: f.award_id,
                            statement: f.statement,
                        })
                        .collect(),
                },
            },
            body: Some(Body {
                sections: js.sections.into_iter().map(|s| s.into()).collect(),
            }),
            back: Some(Back {
                acknowledgments: js.acknowledgments,
                conflict_of_interest: js.conflict_of_interest,
                references: js.references.into_iter().map(|r| r.into()).collect(),
                appendices: Vec::new(),
                glossary: Vec::new(),
            }),
            supplementary_materials: Vec::new(),
            data_availability: js.data_availability,
        }
    }
}

impl From<pubmed_client::Author> for JsAuthor {
    fn from(author: pubmed_client::Author) -> Self {
        // Convert affiliations to simple strings
        let affiliation_names: Vec<String> = author
            .affiliations
            .into_iter()
            .filter_map(|a| a.institution)
            .collect();

        Self {
            given_names: author.given_names,
            surname: author.surname,
            full_name: author.full_name,
            email: author.email,
            affiliations: affiliation_names,
            is_corresponding: author.is_corresponding,
        }
    }
}

impl From<JsAuthor> for pubmed_client::Author {
    fn from(js: JsAuthor) -> Self {
        let affiliations = js
            .affiliations
            .into_iter()
            .map(|name| pubmed_client::Affiliation {
                id: None,
                institution: Some(name),
                department: None,
                address: None,
                country: None,
            })
            .collect();

        Self {
            given_names: js.given_names,
            surname: js.surname,
            initials: None,
            suffix: None,
            full_name: js.full_name,
            affiliations,
            orcid: None,
            email: js.email,
            roles: Vec::new(),
            collab_name: None,
            is_corresponding: js.is_corresponding,
        }
    }
}

impl From<pubmed_client::pmc::JournalMeta> for JsJournal {
    fn from(journal: pubmed_client::pmc::JournalMeta) -> Self {
        Self {
            title: journal.title,
            abbreviation: journal.abbreviation,
            publisher: journal.publisher,
            issn_print: journal.issn_print,
            issn_electronic: journal.issn_electronic,
        }
    }
}

impl From<JsJournal> for pubmed_client::pmc::JournalMeta {
    fn from(js: JsJournal) -> Self {
        Self {
            title: js.title,
            abbreviation: js.abbreviation,
            issn_print: js.issn_print,
            issn_electronic: js.issn_electronic,
            publisher: js.publisher,
        }
    }
}

impl From<pubmed_client::pmc::Section> for JsSection {
    fn from(section: pubmed_client::pmc::Section) -> Self {
        Self {
            section_type: section.section_type,
            title: section.title,
            content: section.content,
            subsections: section
                .subsections
                .into_iter()
                .map(JsSection::from)
                .collect(),
            figures: section
                .figures
                .into_iter()
                .map(|f| JsFigure {
                    id: f.id,
                    label: f.label,
                    caption: f.caption,
                    graphic_href: f.graphic_href,
                })
                .collect(),
            tables: section
                .tables
                .into_iter()
                .map(|t| JsTable {
                    id: t.id,
                    label: t.label,
                    caption: t.caption,
                    footnotes: t.footnotes,
                })
                .collect(),
        }
    }
}

impl From<JsSection> for pubmed_client::pmc::Section {
    fn from(js: JsSection) -> Self {
        Self {
            id: None,
            section_type: js.section_type,
            label: None,
            title: js.title,
            content: js.content,
            subsections: js.subsections.into_iter().map(|s| s.into()).collect(),
            figures: js
                .figures
                .into_iter()
                .map(|f| pubmed_client::pmc::Figure {
                    id: f.id,
                    label: f.label,
                    caption: f.caption,
                    alt_text: None,
                    fig_type: None,
                    graphic_href: f.graphic_href,
                })
                .collect(),
            tables: js
                .tables
                .into_iter()
                .map(|t| pubmed_client::pmc::Table {
                    id: t.id,
                    label: t.label,
                    caption: t.caption,
                    head: Vec::new(),
                    body: Vec::new(),
                    footnotes: t.footnotes,
                })
                .collect(),
            formulas: Vec::new(),
        }
    }
}

impl From<pubmed_client::pmc::Reference> for JsReference {
    fn from(reference: pubmed_client::pmc::Reference) -> Self {
        // Convert Author structs to simple strings
        let author_names: Vec<String> = reference
            .authors
            .into_iter()
            .map(|author| author.full_name)
            .collect();

        Self {
            id: reference.id,
            title: reference.title,
            authors: author_names,
            journal: reference.source,
            year: reference.year,
            pmid: reference.pmid,
            doi: reference.doi,
        }
    }
}

impl From<JsReference> for pubmed_client::pmc::Reference {
    fn from(js: JsReference) -> Self {
        // Convert simple strings back to Author structs
        let authors: Vec<pubmed_client::Author> = js
            .authors
            .into_iter()
            .map(pubmed_client::Author::from_full_name)
            .collect();

        Self {
            id: js.id,
            publication_type: None,
            title: js.title,
            authors,
            editors: Vec::new(),
            source: js.journal,
            year: js.year,
            volume: None,
            issue: None,
            pages: None,
            elocation_id: None,
            publisher_name: None,
            publisher_loc: None,
            edition: None,
            isbn: None,
            conf_name: None,
            pmid: js.pmid,
            doi: js.doi,
        }
    }
}

// ================================================================================================
// ECitMatch types for WASM
// ================================================================================================

/// JavaScript-friendly citation query input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsCitationQuery {
    pub journal: String,
    pub year: String,
    pub volume: String,
    pub first_page: String,
    pub author_name: String,
    pub key: String,
}

/// JavaScript-friendly citation match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsCitationMatch {
    pub journal: String,
    pub year: String,
    pub volume: String,
    pub first_page: String,
    pub author_name: String,
    pub key: String,
    pub pmid: Option<String>,
    pub status: String,
}

impl From<&pubmed_client::CitationMatch> for JsCitationMatch {
    fn from(m: &pubmed_client::CitationMatch) -> Self {
        let status = match m.status {
            pubmed_client::CitationMatchStatus::Found => "found",
            pubmed_client::CitationMatchStatus::NotFound => "not_found",
            pubmed_client::CitationMatchStatus::Ambiguous => "ambiguous",
        };
        Self {
            journal: m.journal.clone(),
            year: m.year.clone(),
            volume: m.volume.clone(),
            first_page: m.first_page.clone(),
            author_name: m.author_name.clone(),
            key: m.key.clone(),
            pmid: m.pmid.clone(),
            status: status.to_string(),
        }
    }
}

// ================================================================================================
// EGQuery types for WASM
// ================================================================================================

/// JavaScript-friendly database count result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsDatabaseCount {
    pub db_name: String,
    pub menu_name: String,
    pub count: u64,
    pub status: String,
}

/// JavaScript-friendly global query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsGlobalQueryResults {
    pub term: String,
    pub results: Vec<JsDatabaseCount>,
}

// ================================================================================================
// ESpell types for WASM
// ================================================================================================

/// JavaScript-friendly spell check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsSpellCheckResult {
    pub database: String,
    pub query: String,
    pub corrected_query: String,
    pub has_corrections: bool,
    pub replacements: Vec<String>,
}

impl From<pubmed_client::SpellCheckResult> for JsSpellCheckResult {
    fn from(result: pubmed_client::SpellCheckResult) -> Self {
        let has_corrections = result.has_corrections();
        let replacements = result
            .replacements()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            database: result.database,
            query: result.query,
            corrected_query: result.corrected_query,
            has_corrections,
            replacements,
        }
    }
}

// ================================================================================================
// WasmSearchQuery builder
// ================================================================================================

/// Map legacy snake_case article-type aliases to the canonical names accepted by
/// `ArticleType::from_str_insensitive`. Unknown values pass through unchanged.
fn normalize_article_type(article_type: &str) -> &str {
    match article_type {
        "clinical_trial" => "Clinical Trial",
        "meta_analysis" => "Meta-Analysis",
        "randomized_controlled_trial" => "Randomized Controlled Trial",
        "systematic_review" => "Systematic Review",
        "case_report" => "Case Reports",
        "observational_study" => "Observational Study",
        other => other,
    }
}

/// Search query builder for constructing complex PubMed queries
///
/// Provides a fluent API for building PubMed search queries with filters,
/// date ranges, article types, and boolean logic.
#[wasm_bindgen]
pub struct WasmSearchQuery {
    inner: pubmed_client::SearchQuery,
}

#[wasm_bindgen]
impl WasmSearchQuery {
    /// Create a new empty search query builder
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: pubmed_client::SearchQuery::new(),
        }
    }

    /// Set the main search query text
    pub fn query(mut self, query: &str) -> Self {
        self.inner = self.inner.query(query);
        self
    }

    /// Search in article titles only
    pub fn title(mut self, title: &str) -> Self {
        self.inner = self.inner.title_contains(title);
        self
    }

    /// Search in title and abstract
    pub fn title_abstract(mut self, text: &str) -> Self {
        self.inner = self.inner.title_or_abstract(text);
        self
    }

    /// Search by author name
    pub fn author(mut self, author: &str) -> Self {
        self.inner = self.inner.author(author);
        self
    }

    /// Search by first author name
    pub fn first_author(mut self, author: &str) -> Self {
        self.inner = self.inner.first_author(author);
        self
    }

    /// Search by journal name
    pub fn journal(mut self, journal: &str) -> Self {
        self.inner = self.inner.journal(journal);
        self
    }

    /// Search by MeSH term
    pub fn mesh_term(mut self, term: &str) -> Self {
        self.inner = self.inner.mesh_term(term);
        self
    }

    /// Search by MeSH major topic
    pub fn mesh_major_topic(mut self, term: &str) -> Self {
        self.inner = self.inner.mesh_major_topic(term);
        self
    }

    /// Filter to only free full text articles
    pub fn free_full_text_only(mut self) -> Self {
        self.inner = self.inner.free_full_text_only();
        self
    }

    /// Filter to only articles available in PMC
    pub fn pmc_only(mut self) -> Self {
        self.inner = self.inner.pmc_only();
        self
    }

    /// Filter to only articles with full text available
    pub fn full_text_only(mut self) -> Self {
        self.inner = self.inner.full_text_only();
        self
    }

    /// Filter to articles published after a given year (must be 1800–3000)
    pub fn published_after(mut self, year: u32) -> Result<Self, JsValue> {
        pubmed_client::validate_year(year).map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.published_after(year);
        Ok(self)
    }

    /// Filter to articles published in a date range (years must be 1800–3000)
    pub fn date_range(mut self, start_year: u32, end_year: Option<u32>) -> Result<Self, JsValue> {
        pubmed_client::validate_year(start_year).map_err(|e| JsValue::from_str(&e))?;
        if let Some(end) = end_year {
            pubmed_client::validate_year(end).map_err(|e| JsValue::from_str(&e))?;
            if start_year > end {
                return Err(JsValue::from_str(&format!(
                    "Start year ({}) must be <= end year ({})",
                    start_year, end
                )));
            }
        }
        self.inner = self.inner.date_range(start_year, end_year);
        Ok(self)
    }

    /// Filter by article type string (case-insensitive).
    ///
    /// Accepted values: `"Clinical Trial"`, `"Review"`, `"Systematic Review"`,
    /// `"Meta-Analysis"` / `"Meta Analysis"`, `"Case Reports"` / `"Case Report"`,
    /// `"Randomized Controlled Trial"` / `"RCT"`, `"Observational Study"`.
    /// Also accepts the legacy snake_case aliases used in previous versions.
    pub fn article_type_str(mut self, article_type: &str) -> Result<Self, JsValue> {
        let normalized = normalize_article_type(article_type);
        let at = pubmed_client::ArticleType::from_str_insensitive(normalized)
            .map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.article_type(at);
        Ok(self)
    }

    /// Filter to human-only studies
    pub fn humans_only(mut self) -> Self {
        self.inner = self.inner.human_studies_only();
        self
    }

    /// Filter by language string (case-insensitive, accepts full names and ISO 639-2 codes).
    ///
    /// Unknown values fall back to `Language::Other(s)` rather than being silently ignored.
    pub fn language_str(mut self, language: &str) -> Self {
        let lang = pubmed_client::Language::from_str_insensitive(language);
        self.inner = self.inner.language(lang);
        self
    }

    /// Set the sort order for results (case-insensitive).
    ///
    /// Accepted values: `"relevance"`, `"pub_date"` / `"publication_date"` / `"date"`,
    /// `"author"` / `"first_author"`, `"journal"` / `"journal_name"`.
    pub fn sort_str(mut self, sort: &str) -> Result<Self, JsValue> {
        let order = pubmed_client::SortOrder::from_str_insensitive(sort)
            .map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.sort(order);
        Ok(self)
    }

    /// Add multiple search terms (joined with the default AND logic)
    pub fn terms(mut self, terms: Vec<String>) -> Self {
        self.inner = self.inner.terms(&terms);
        self
    }

    /// Set the maximum number of results to retrieve
    pub fn limit(mut self, limit: usize) -> Self {
        self.inner = self.inner.limit(limit);
        self
    }

    /// Get the currently configured result limit
    pub fn get_limit(&self) -> usize {
        self.inner.get_limit()
    }

    /// Search in article abstracts only
    pub fn abstract_contains(mut self, text: &str) -> Self {
        self.inner = self.inner.abstract_contains(text);
        self
    }

    /// Filter to only articles that have an abstract
    pub fn has_abstract(mut self) -> Self {
        self.inner = self.inner.has_abstract();
        self
    }

    /// Search by journal abbreviation (e.g. "N Engl J Med")
    pub fn journal_abbreviation(mut self, abbreviation: &str) -> Self {
        self.inner = self.inner.journal_abbreviation(abbreviation);
        self
    }

    /// Search by grant number
    pub fn grant_number(mut self, grant_number: &str) -> Self {
        self.inner = self.inner.grant_number(grant_number);
        self
    }

    /// Search by ISBN
    pub fn isbn(mut self, isbn: &str) -> Self {
        self.inner = self.inner.isbn(isbn);
        self
    }

    /// Search by ISSN
    pub fn issn(mut self, issn: &str) -> Self {
        self.inner = self.inner.issn(issn);
        self
    }

    /// Search by last author name
    pub fn last_author(mut self, author: &str) -> Self {
        self.inner = self.inner.last_author(author);
        self
    }

    /// Search by author affiliation / institution
    pub fn affiliation(mut self, institution: &str) -> Self {
        self.inner = self.inner.affiliation(institution);
        self
    }

    /// Search by author ORCID identifier
    pub fn orcid(mut self, orcid_id: &str) -> Self {
        self.inner = self.inner.orcid(orcid_id);
        self
    }

    /// Search by multiple MeSH terms (OR logic)
    pub fn mesh_terms(mut self, terms: Vec<String>) -> Self {
        self.inner = self.inner.mesh_terms(&terms);
        self
    }

    /// Search by MeSH subheading
    pub fn mesh_subheading(mut self, subheading: &str) -> Self {
        self.inner = self.inner.mesh_subheading(subheading);
        self
    }

    /// Filter to animal-only studies
    pub fn animal_studies_only(mut self) -> Self {
        self.inner = self.inner.animal_studies_only();
        self
    }

    /// Filter by age group (e.g. "Adult", "Child")
    pub fn age_group(mut self, age_group: &str) -> Self {
        self.inner = self.inner.age_group(age_group);
        self
    }

    /// Filter by organism using its MeSH term
    pub fn organism_mesh(mut self, organism: &str) -> Self {
        self.inner = self.inner.organism_mesh(organism);
        self
    }

    /// Add a raw custom filter clause appended verbatim to the query
    pub fn custom_filter(mut self, filter: &str) -> Self {
        self.inner = self.inner.custom_filter(filter);
        self
    }

    /// Filter by multiple article types (OR logic, case-insensitive).
    ///
    /// Accepts the same values as `article_type_str`, including the legacy
    /// snake_case aliases. An empty array is a no-op.
    pub fn article_types_str(mut self, article_types: Vec<String>) -> Result<Self, JsValue> {
        if article_types.is_empty() {
            return Ok(self);
        }
        let mut parsed = Vec::with_capacity(article_types.len());
        for at in &article_types {
            let normalized = normalize_article_type(at);
            parsed.push(
                pubmed_client::ArticleType::from_str_insensitive(normalized)
                    .map_err(|e| JsValue::from_str(&e))?,
            );
        }
        self.inner = self.inner.article_types(&parsed);
        Ok(self)
    }

    /// Filter to a single publication year (must be 1800–3000)
    pub fn published_in_year(mut self, year: u32) -> Result<Self, JsValue> {
        pubmed_client::validate_year(year).map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.published_in_year(year);
        Ok(self)
    }

    /// Filter to a publication-year range (years must be 1800–3000)
    pub fn published_between(
        mut self,
        start_year: u32,
        end_year: Option<u32>,
    ) -> Result<Self, JsValue> {
        pubmed_client::validate_year(start_year).map_err(|e| JsValue::from_str(&e))?;
        if let Some(end) = end_year {
            pubmed_client::validate_year(end).map_err(|e| JsValue::from_str(&e))?;
            if start_year > end {
                return Err(JsValue::from_str(&format!(
                    "Start year ({}) must be <= end year ({})",
                    start_year, end
                )));
            }
        }
        self.inner = self.inner.published_between(start_year, end_year);
        Ok(self)
    }

    /// Filter to articles published before a given year (must be 1800–3000)
    pub fn published_before(mut self, year: u32) -> Result<Self, JsValue> {
        pubmed_client::validate_year(year).map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.published_before(year);
        Ok(self)
    }

    /// Combine this query with another using AND logic, returning a new query
    pub fn and(&self, other: &WasmSearchQuery) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().and(other.inner.clone()),
        }
    }

    /// Combine this query with another using OR logic, returning a new query
    pub fn or(&self, other: &WasmSearchQuery) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().or(other.inner.clone()),
        }
    }

    /// Negate this query using NOT logic, returning a new query
    pub fn negate(&self) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().negate(),
        }
    }

    /// Exclude articles matching another query, returning a new query
    pub fn exclude(&self, excluded: &WasmSearchQuery) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().exclude(excluded.inner.clone()),
        }
    }

    /// Wrap this query in parentheses for grouping, returning a new query
    pub fn group(&self) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().group(),
        }
    }

    /// Build the query string
    pub fn build(&self) -> String {
        self.inner.build()
    }

    /// Search and fetch articles using this query
    pub fn search_and_fetch(&self, client: &WasmPubMedClient, limit: usize) -> JsPromiseArticles {
        let query_string = self.inner.build();
        let sort = self.inner.get_sort().cloned();
        let rust_client = client.client.clone();
        future_to_promise(async move {
            match rust_client
                .pubmed
                .search_and_fetch(&query_string, limit, sort.as_ref())
                .await
            {
                Ok(articles) => {
                    let js_articles: Vec<JsArticle> =
                        articles.into_iter().map(JsArticle::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_articles)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }
}

impl Default for WasmSearchQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl From<pubmed_client::GlobalQueryResults> for JsGlobalQueryResults {
    fn from(results: pubmed_client::GlobalQueryResults) -> Self {
        Self {
            term: results.term,
            results: results
                .results
                .into_iter()
                .map(|r| JsDatabaseCount {
                    db_name: r.db_name,
                    menu_name: r.menu_name,
                    count: r.count,
                    status: r.status,
                })
                .collect(),
        }
    }
}
