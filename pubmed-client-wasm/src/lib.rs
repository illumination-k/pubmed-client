//! WebAssembly bindings for the PubMed client library
//!
//! This module provides JavaScript-compatible bindings for use in Node.js and browsers.

use pubmed_client::{config::ClientConfig, pmc::PmcFullText, pubmed::PubMedArticle, Client};
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
            match client.pubmed.search_and_fetch(&query, limit).await {
                Ok(articles) => {
                    let js_articles: Vec<JsArticle> =
                        articles.into_iter().map(JsArticle::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_articles)?)
                }
                Err(e) => Err(JsValue::from_str(&format!("Search failed: {e}"))),
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
                Err(e) => Err(JsValue::from_str(&format!("Batch fetch failed: {e}"))),
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
                Err(e) => Err(JsValue::from_str(&format!("Fetch failed: {e}"))),
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
                Err(e) => Err(JsValue::from_str(&format!("Full text fetch failed: {e}"))),
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
                Err(e) => Err(JsValue::from_str(&format!("PMC check failed: {e}"))),
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
                Err(e) => Err(JsValue::from_str(&format!(
                    "Related articles fetch failed: {e}"
                ))),
            }
        })
        .unchecked_into()
    }

    /// Convert PMC full text to markdown
    pub fn convert_to_markdown(&self, full_text_js: JsValue) -> Result<String, JsValue> {
        let js_full_text: JsFullText = serde_wasm_bindgen::from_value(full_text_js)
            .map_err(|e| JsValue::from_str(&format!("Invalid full text data: {e}")))?;

        let full_text: PmcFullText = js_full_text.into();
        let converter = pubmed_client::pmc::PmcMarkdownConverter::new();
        Ok(converter.convert(&full_text))
    }
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
    pub title: String,
    pub authors: Vec<JsAuthor>,
    pub journal: JsJournal,
    pub pub_date: String,
    pub doi: Option<String>,
    pub sections: Vec<JsSection>,
    pub references: Vec<JsReference>,
    pub article_type: Option<String>,
    pub keywords: Vec<String>,
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
    pub title: String,
    pub abbreviation: Option<String>,
    pub publisher: Option<String>,
    pub issn_print: Option<String>,
    pub issn_electronic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsSection {
    pub section_type: String,
    pub title: Option<String>,
    pub content: String,
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

impl From<PmcFullText> for JsFullText {
    fn from(full_text: PmcFullText) -> Self {
        Self {
            pmcid: full_text.pmcid,
            pmid: full_text.pmid,
            title: full_text.title,
            authors: full_text.authors.into_iter().map(JsAuthor::from).collect(),
            journal: JsJournal::from(full_text.journal),
            pub_date: full_text.pub_date,
            doi: full_text.doi,
            sections: full_text
                .sections
                .into_iter()
                .map(JsSection::from)
                .collect(),
            references: full_text
                .references
                .into_iter()
                .map(JsReference::from)
                .collect(),
            article_type: full_text.article_type,
            keywords: full_text.keywords,
        }
    }
}

impl From<JsFullText> for PmcFullText {
    fn from(js: JsFullText) -> Self {
        Self {
            pmcid: js.pmcid,
            pmid: js.pmid,
            title: js.title,
            authors: js.authors.into_iter().map(|a| a.into()).collect(),
            journal: js.journal.into(),
            pub_date: js.pub_date,
            doi: js.doi,
            sections: js.sections.into_iter().map(|s| s.into()).collect(),
            references: js.references.into_iter().map(|r| r.into()).collect(),
            article_type: js.article_type,
            keywords: js.keywords,
            funding: Vec::new(),
            conflict_of_interest: None,
            acknowledgments: None,
            data_availability: None,
            supplementary_materials: Vec::new(),
        }
    }
}

impl From<pubmed_client::pmc::Author> for JsAuthor {
    fn from(author: pubmed_client::pmc::Author) -> Self {
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

impl From<JsAuthor> for pubmed_client::pmc::Author {
    fn from(js: JsAuthor) -> Self {
        let affiliations = js
            .affiliations
            .into_iter()
            .map(|name| pubmed_client::pmc::Affiliation {
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
            is_corresponding: js.is_corresponding,
        }
    }
}

impl From<pubmed_client::pmc::JournalInfo> for JsJournal {
    fn from(journal: pubmed_client::pmc::JournalInfo) -> Self {
        Self {
            title: journal.title,
            abbreviation: journal.abbreviation,
            publisher: journal.publisher,
            issn_print: journal.issn_print,
            issn_electronic: journal.issn_electronic,
        }
    }
}

impl From<JsJournal> for pubmed_client::pmc::JournalInfo {
    fn from(js: JsJournal) -> Self {
        Self {
            title: js.title,
            abbreviation: js.abbreviation,
            issn_print: js.issn_print,
            issn_electronic: js.issn_electronic,
            publisher: js.publisher,
            volume: None,
            issue: None,
        }
    }
}

impl From<pubmed_client::pmc::ArticleSection> for JsSection {
    fn from(section: pubmed_client::pmc::ArticleSection) -> Self {
        Self {
            section_type: section.section_type,
            title: section.title,
            content: section.content,
        }
    }
}

impl From<JsSection> for pubmed_client::pmc::ArticleSection {
    fn from(js: JsSection) -> Self {
        Self {
            section_type: js.section_type,
            title: js.title,
            content: js.content,
            subsections: Vec::new(),
            id: None,
            figures: Vec::new(),
            tables: Vec::new(),
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
            journal: reference.journal,
            year: reference.year,
            pmid: reference.pmid,
            doi: reference.doi,
        }
    }
}

impl From<JsReference> for pubmed_client::pmc::Reference {
    fn from(js: JsReference) -> Self {
        // Convert simple strings back to Author structs
        let authors: Vec<pubmed_client::pmc::Author> = js
            .authors
            .into_iter()
            .map(|name| pubmed_client::pmc::Author {
                given_names: None,
                surname: None,
                initials: None,
                suffix: None,
                full_name: name,
                affiliations: Vec::new(),
                orcid: None,
                email: None,
                roles: Vec::new(),
                is_corresponding: false,
            })
            .collect();

        Self {
            id: js.id,
            title: js.title,
            authors,
            journal: js.journal,
            year: js.year,
            volume: None,
            issue: None,
            pages: None,
            pmid: js.pmid,
            doi: js.doi,
            ref_type: None,
        }
    }
}
