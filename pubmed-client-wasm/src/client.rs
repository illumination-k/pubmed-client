use pubmed_client::{Client, config::ClientConfig, pmc::PmcArticle};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::config::WasmClientConfig;
use crate::error::to_js_err;
use crate::models::{
    JsArticle, JsCitationMatch, JsCitationQuery, JsEPostResult, JsFullText, JsGlobalQueryResults,
    JsMarkdownOptions, JsOaSubsetInfo, JsSpellCheckResult, JsSummary,
};
use crate::{
    JsPromiseArticle, JsPromiseArticles, JsPromiseFullText, JsPromiseOptString,
    JsPromiseStringArray, JsPromiseSummaries,
};

/// JavaScript-friendly wrapper for the PubMed client
#[wasm_bindgen]
pub struct WasmPubMedClient {
    pub(crate) client: Client,
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

    /// Export articles as CSL-JSON format
    ///
    /// Returns a Promise resolving to a JSON string (an array of CSL-JSON items).
    pub fn export_csl_json(&self, pmids: Vec<String>) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.fetch_articles(&pmid_refs).await {
                Ok(articles) => {
                    let value = pubmed_client::export::articles_to_csl_json(&articles);
                    Ok(JsValue::from_str(&value.to_string()))
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
    }

    /// Export articles in MEDLINE/NBIB format
    pub fn export_nbib(&self, pmids: Vec<String>) -> js_sys::Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
            match client.pubmed.fetch_articles(&pmid_refs).await {
                Ok(articles) => {
                    let nbib = articles
                        .iter()
                        .map(pubmed_client::ExportFormat::to_nbib)
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(JsValue::from_str(&nbib))
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
