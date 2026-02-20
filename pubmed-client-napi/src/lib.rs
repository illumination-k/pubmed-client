#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use pubmed_client::{
    pmc::{markdown::PmcMarkdownConverter, PmcFullText},
    pubmed::{
        ArticleSummary, ArticleType, Language, PubMedArticle, SearchQuery as RustSearchQuery,
        SortOrder,
    },
    Client, ClientConfig,
};
use std::sync::Arc;

/// Configuration options for the PubMed client
#[napi(object)]
#[derive(Default)]
pub struct Config {
    /// NCBI API key for higher rate limits
    pub api_key: Option<String>,
    /// Email address for NCBI identification
    pub email: Option<String>,
    /// Tool name for NCBI identification
    pub tool: Option<String>,
    /// Request timeout in seconds
    pub timeout_seconds: Option<u32>,
}

/// Author information
#[napi(object)]
pub struct Author {
    /// Full name of the author
    pub full_name: String,
    /// ORCID identifier if available
    pub orcid: Option<String>,
    /// Primary affiliation
    pub affiliation: Option<String>,
}

/// PubMed article metadata
#[napi(object)]
pub struct Article {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// List of authors
    pub authors: Vec<Author>,
    /// Journal name
    pub journal: String,
    /// Publication date
    pub pub_date: String,
    /// DOI if available
    pub doi: Option<String>,
    /// PMC ID if available
    pub pmc_id: Option<String>,
    /// Abstract text
    pub abstract_text: Option<String>,
    /// Article types (e.g., "Research Article", "Review")
    pub article_types: Vec<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// Journal volume (e.g., "88")
    pub volume: Option<String>,
    /// Journal issue number (e.g., "3")
    pub issue: Option<String>,
    /// Page range (e.g., "123-130")
    pub pages: Option<String>,
    /// Article language (e.g., "eng")
    pub language: Option<String>,
    /// ISO journal abbreviation (e.g., "J Biol Chem")
    pub journal_abbreviation: Option<String>,
    /// ISSN
    pub issn: Option<String>,
}

impl From<PubMedArticle> for Article {
    fn from(article: PubMedArticle) -> Self {
        Article {
            pmid: article.pmid,
            title: article.title,
            authors: article
                .authors
                .into_iter()
                .map(|a| {
                    let affiliation = a.primary_affiliation().map(|aff| {
                        aff.institution
                            .clone()
                            .unwrap_or_else(|| aff.address.clone().unwrap_or_default())
                    });
                    Author {
                        full_name: a.full_name,
                        orcid: a.orcid,
                        affiliation,
                    }
                })
                .collect(),
            journal: article.journal,
            pub_date: article.pub_date,
            doi: article.doi,
            pmc_id: article.pmc_id,
            abstract_text: article.abstract_text,
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

/// Lightweight article summary from ESummary API
///
/// Contains basic metadata without abstracts, MeSH terms, or chemical lists.
/// Use fetchSummaries() for faster bulk metadata retrieval.
#[napi(object)]
pub struct Summary {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// Author names
    pub authors: Vec<String>,
    /// Journal name
    pub journal: String,
    /// Full journal name
    pub full_journal_name: String,
    /// Publication date
    pub pub_date: String,
    /// Electronic publication date
    pub epub_date: String,
    /// DOI if available
    pub doi: Option<String>,
    /// PMC ID if available
    pub pmc_id: Option<String>,
    /// Journal volume
    pub volume: String,
    /// Journal issue
    pub issue: String,
    /// Page range
    pub pages: String,
    /// Languages
    pub languages: Vec<String>,
    /// Publication types
    pub pub_types: Vec<String>,
    /// ISSN
    pub issn: String,
    /// Electronic ISSN
    pub essn: String,
    /// Sorted publication date
    pub sort_pub_date: String,
    /// PMC reference count
    pub pmc_ref_count: u32,
    /// Record status
    pub record_status: String,
}

impl From<ArticleSummary> for Summary {
    fn from(s: ArticleSummary) -> Self {
        Summary {
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
            pmc_ref_count: s.pmc_ref_count as u32,
            record_status: s.record_status,
        }
    }
}

/// Reference information from PMC articles
#[napi(object)]
pub struct Reference {
    /// Reference ID
    pub id: String,
    /// Reference title
    pub title: Option<String>,
    /// Authors as a single string
    pub authors: String,
    /// Journal name
    pub journal: Option<String>,
    /// Publication year
    pub year: Option<String>,
    /// PubMed ID if available
    pub pmid: Option<String>,
    /// DOI if available
    pub doi: Option<String>,
}

/// Section of a PMC article
#[napi(object)]
pub struct Section {
    /// Section type (e.g., "introduction", "methods")
    pub section_type: String,
    /// Section title
    pub title: Option<String>,
    /// Section content
    pub content: String,
}

/// Information about OA (Open Access) subset availability for a PMC article
///
/// The OA subset contains articles with programmatic access to full-text XML.
/// Not all PMC articles are in the OA subset - some publishers restrict programmatic access
/// even though the article may be viewable on the PMC website.
#[napi(object)]
pub struct OaSubsetInfo {
    /// PMC ID (e.g., "PMC7906746")
    pub pmcid: String,
    /// Whether the article is in the OA subset
    pub is_oa_subset: bool,
    /// Citation string (if available)
    pub citation: Option<String>,
    /// License type (if available)
    pub license: Option<String>,
    /// Whether the article is retracted
    pub retracted: bool,
    /// Download link for tar.gz package (if available)
    pub download_link: Option<String>,
    /// Format of the download (e.g., "tgz", "pdf")
    pub download_format: Option<String>,
    /// Last updated timestamp for the download
    pub updated: Option<String>,
    /// Error code if not in OA subset
    pub error_code: Option<String>,
    /// Error message if not in OA subset
    pub error_message: Option<String>,
}

impl From<pubmed_client::OaSubsetInfo> for OaSubsetInfo {
    fn from(info: pubmed_client::OaSubsetInfo) -> Self {
        OaSubsetInfo {
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

/// Spelling suggestion result from the ESpell API
#[napi(object)]
pub struct SpellCheckResult {
    /// The database that was queried
    pub database: String,
    /// The original query string as submitted
    pub query: String,
    /// The full corrected/suggested query as a plain string
    pub corrected_query: String,
    /// Whether any spelling corrections were made
    pub has_corrections: bool,
    /// The corrected terms (only the replaced parts)
    pub replacements: Vec<String>,
}

impl From<pubmed_client::SpellCheckResult> for SpellCheckResult {
    fn from(result: pubmed_client::SpellCheckResult) -> Self {
        let has_corrections = result.has_corrections();
        let replacements = result
            .replacements()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        SpellCheckResult {
            database: result.database,
            query: result.query,
            corrected_query: result.corrected_query,
            has_corrections,
            replacements,
        }
    }
}

/// Full-text article from PMC
#[napi(object)]
pub struct FullTextArticle {
    /// PMC ID
    pub pmcid: String,
    /// PubMed ID if available
    pub pmid: Option<String>,
    /// Article title
    pub title: String,
    /// List of authors
    pub authors: Vec<Author>,
    /// Journal name
    pub journal: String,
    /// Publication date
    pub pub_date: String,
    /// DOI if available
    pub doi: Option<String>,
    /// Article sections
    pub sections: Vec<Section>,
    /// References
    pub references: Vec<Reference>,
    /// Keywords
    pub keywords: Vec<String>,
}

impl From<PmcFullText> for FullTextArticle {
    fn from(article: PmcFullText) -> Self {
        FullTextArticle {
            pmcid: article.pmcid,
            pmid: article.pmid,
            title: article.title,
            authors: article
                .authors
                .into_iter()
                .map(|a| {
                    let affiliation = a.primary_affiliation().map(|aff| {
                        aff.institution
                            .clone()
                            .unwrap_or_else(|| aff.address.clone().unwrap_or_default())
                    });
                    Author {
                        full_name: a.full_name,
                        orcid: a.orcid,
                        affiliation,
                    }
                })
                .collect(),
            journal: article.journal.title,
            pub_date: article.pub_date,
            doi: article.doi,
            sections: article
                .sections
                .into_iter()
                .map(|s| Section {
                    section_type: s.section_type,
                    title: s.title,
                    content: s.content,
                })
                .collect(),
            references: article
                .references
                .into_iter()
                .map(|r| Reference {
                    id: r.id,
                    title: r.title,
                    authors: r
                        .authors
                        .iter()
                        .map(|a| a.full_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    journal: r.journal,
                    year: r.year,
                    pmid: r.pmid,
                    doi: r.doi,
                })
                .collect(),
            keywords: article.keywords,
        }
    }
}

/// Markdown conversion options
#[napi(object)]
#[derive(Default)]
pub struct MarkdownOptions {
    /// Include metadata header
    pub include_metadata: Option<bool>,
    /// Include table of contents
    pub include_toc: Option<bool>,
    /// Use YAML frontmatter for metadata
    pub use_yaml_frontmatter: Option<bool>,
    /// Include ORCID links for authors
    pub include_orcid_links: Option<bool>,
    /// Include figure captions
    pub include_figure_captions: Option<bool>,
}

/// PubMed/PMC API client
#[napi]
pub struct PubMedClient {
    client: Arc<Client>,
}

impl Default for PubMedClient {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl PubMedClient {
    /// Create a new PubMed client with default configuration
    #[napi(constructor)]
    pub fn new() -> Self {
        PubMedClient {
            client: Arc::new(Client::new()),
        }
    }

    /// Create a new PubMed client with custom configuration
    #[napi(factory)]
    pub fn with_config(config: Config) -> Self {
        let mut client_config = ClientConfig::new();

        if let Some(api_key) = config.api_key {
            client_config = client_config.with_api_key(api_key);
        }
        if let Some(email) = config.email {
            client_config = client_config.with_email(email);
        }
        if let Some(tool) = config.tool {
            client_config = client_config.with_tool(tool);
        }
        if let Some(timeout) = config.timeout_seconds {
            client_config = client_config.with_timeout_seconds(timeout as u64);
        }

        PubMedClient {
            client: Arc::new(Client::with_config(client_config)),
        }
    }

    /// Search PubMed and fetch article metadata
    ///
    /// @param query - Search query string (PubMed syntax supported)
    /// @param limit - Maximum number of results to return
    /// @returns Array of article metadata
    #[napi]
    pub async fn search(&self, query: String, limit: Option<u32>) -> Result<Vec<Article>> {
        let limit = limit.unwrap_or(10) as usize;
        let articles = self
            .client
            .pubmed
            .search_and_fetch(&query, limit)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(articles.into_iter().map(Article::from).collect())
    }

    /// Fetch multiple articles by PMIDs in a single batch request
    ///
    /// This is significantly more efficient than fetching articles one by one.
    /// For large numbers of PMIDs, requests are automatically batched (200 per request).
    ///
    /// @param pmids - Array of PubMed IDs
    /// @returns Array of article metadata
    #[napi]
    pub async fn fetch_articles(&self, pmids: Vec<String>) -> Result<Vec<Article>> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let articles = self
            .client
            .pubmed
            .fetch_articles(&pmid_refs)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(articles.into_iter().map(Article::from).collect())
    }

    /// Fetch a single article by PMID
    ///
    /// @param pmid - PubMed ID
    /// @returns Article metadata
    #[napi]
    pub async fn fetch_article(&self, pmid: String) -> Result<Article> {
        let article = self
            .client
            .pubmed
            .fetch_article(&pmid)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(Article::from(article))
    }

    /// Fetch lightweight article summaries by PMIDs using the ESummary API
    ///
    /// Returns basic metadata (title, authors, journal, dates, DOI) without
    /// abstracts, MeSH terms, or chemical lists. Faster than fetchArticles().
    ///
    /// @param pmids - Array of PubMed IDs
    /// @returns Array of article summaries
    #[napi]
    pub async fn fetch_summaries(&self, pmids: Vec<String>) -> Result<Vec<Summary>> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let summaries = self
            .client
            .pubmed
            .fetch_summaries(&pmid_refs)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(summaries.into_iter().map(Summary::from).collect())
    }

    /// Search PubMed and fetch lightweight summaries
    ///
    /// Combines search and ESummary fetch. Faster than search() when you
    /// only need basic metadata.
    ///
    /// @param query - Search query string
    /// @param limit - Maximum number of results
    /// @returns Array of article summaries
    #[napi]
    pub async fn search_summaries(
        &self,
        query: String,
        limit: Option<u32>,
    ) -> Result<Vec<Summary>> {
        let limit = limit.unwrap_or(10) as usize;
        let summaries = self
            .client
            .pubmed
            .search_and_fetch_summaries(&query, limit)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(summaries.into_iter().map(Summary::from).collect())
    }

    /// Fetch full-text article from PMC
    ///
    /// @param pmcid - PMC ID (e.g., "PMC7906746")
    /// @returns Full-text article data
    #[napi]
    pub async fn fetch_pmc_article(&self, pmcid: String) -> Result<FullTextArticle> {
        let article = self
            .client
            .pmc
            .fetch_full_text(&pmcid)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(FullTextArticle::from(article))
    }

    /// Fetch PMC article and convert to Markdown
    ///
    /// @param pmcid - PMC ID (e.g., "PMC7906746")
    /// @param options - Markdown conversion options
    /// @returns Markdown string
    #[napi]
    pub async fn fetch_pmc_as_markdown(
        &self,
        pmcid: String,
        options: Option<MarkdownOptions>,
    ) -> Result<String> {
        let article = self
            .client
            .pmc
            .fetch_full_text(&pmcid)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let options = options.unwrap_or_default();
        let mut converter = PmcMarkdownConverter::new();

        if let Some(include_metadata) = options.include_metadata {
            converter = converter.with_include_metadata(include_metadata);
        }
        if let Some(include_toc) = options.include_toc {
            converter = converter.with_include_toc(include_toc);
        }
        if let Some(use_yaml) = options.use_yaml_frontmatter {
            converter = converter.with_yaml_frontmatter(use_yaml);
        }
        if let Some(include_orcid) = options.include_orcid_links {
            converter = converter.with_include_orcid_links(include_orcid);
        }
        if let Some(include_figures) = options.include_figure_captions {
            converter = converter.with_include_figure_captions(include_figures);
        }

        Ok(converter.convert(&article))
    }

    /// Check if a PubMed article has full-text available in PMC
    ///
    /// @param pmid - PubMed ID
    /// @returns PMC ID if available, null otherwise
    #[napi]
    pub async fn check_pmc_availability(&self, pmid: String) -> Result<Option<String>> {
        self.client
            .pmc
            .check_pmc_availability(&pmid)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Check if a PMC article is in the OA (Open Access) subset
    ///
    /// The OA subset contains articles with programmatic access to full-text XML.
    /// Some publishers restrict programmatic access even though the article may be
    /// viewable on the PMC website.
    ///
    /// @param pmcid - PMC ID (e.g., "PMC7906746")
    /// @returns OaSubsetInfo containing detailed information about OA availability
    ///
    /// @example
    /// ```typescript
    /// const client = new PubMedClient();
    /// const info = await client.isOaSubset("PMC7906746");
    ///
    /// if (info.isOaSubset) {
    ///     console.log("Article is in OA subset");
    ///     console.log("Download:", info.downloadLink);
    /// } else {
    ///     console.log("Not in OA subset:", info.errorCode);
    /// }
    /// ```
    #[napi]
    pub async fn is_oa_subset(&self, pmcid: String) -> Result<OaSubsetInfo> {
        let info = self
            .client
            .pmc
            .is_oa_subset(&pmcid)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(OaSubsetInfo::from(info))
    }

    /// Check spelling of a search term using the ESpell API
    ///
    /// Provides spelling suggestions for terms within a single text query.
    /// Uses the PubMed database by default.
    ///
    /// @param term - The search term to spell-check
    /// @returns Spelling suggestions with corrected query
    #[napi]
    pub async fn spell_check(&self, term: String) -> Result<SpellCheckResult> {
        let result = self
            .client
            .pubmed
            .spell_check(&term)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(SpellCheckResult::from(result))
    }

    /// Check spelling of a search term against a specific database using the ESpell API
    ///
    /// Spelling suggestions are database-specific, so use the same database you plan to search.
    ///
    /// @param term - The search term to spell-check
    /// @param db - The NCBI database to check against (e.g., "pubmed", "pmc")
    /// @returns Spelling suggestions with corrected query
    #[napi]
    pub async fn spell_check_db(&self, term: String, db: String) -> Result<SpellCheckResult> {
        let result = self
            .client
            .pubmed
            .spell_check_db(&term, &db)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(SpellCheckResult::from(result))
    }

    /// Execute a search query and return articles
    ///
    /// @param query - SearchQuery instance
    /// @returns Array of article metadata
    #[napi]
    pub async fn execute_query(&self, query: &SearchQuery) -> Result<Vec<Article>> {
        let query_string = query.inner.build();
        let limit = query.inner.get_limit();
        let articles = self
            .client
            .pubmed
            .search_and_fetch(&query_string, limit)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(articles.into_iter().map(Article::from).collect())
    }
}

// ================================================================================================
// Helper Functions
// ================================================================================================

/// Validate year is within reasonable range for biomedical publications
fn validate_year(year: u32) -> Result<()> {
    if !(1800..=3000).contains(&year) {
        return Err(Error::from_reason(format!(
            "Year must be between 1800 and 3000, got: {}",
            year
        )));
    }
    Ok(())
}

/// Convert string to ArticleType enum with case-insensitive matching
fn str_to_article_type(s: &str) -> Result<ArticleType> {
    let normalized = s.trim().to_lowercase();

    match normalized.as_str() {
        "clinical trial" => Ok(ArticleType::ClinicalTrial),
        "review" => Ok(ArticleType::Review),
        "systematic review" => Ok(ArticleType::SystematicReview),
        "meta-analysis" | "meta analysis" => Ok(ArticleType::MetaAnalysis),
        "case reports" | "case report" => Ok(ArticleType::CaseReport),
        "randomized controlled trial" | "rct" => Ok(ArticleType::RandomizedControlledTrial),
        "observational study" => Ok(ArticleType::ObservationalStudy),
        _ => Err(Error::from_reason(format!(
            "Invalid article type: '{}'. Supported types: Clinical Trial, Review, Systematic Review, Meta-Analysis, Case Reports, Randomized Controlled Trial, Observational Study",
            s
        ))),
    }
}

/// Convert string to SortOrder enum with case-insensitive matching
fn str_to_sort_order(s: &str) -> Result<SortOrder> {
    let normalized = s.trim().to_lowercase();

    match normalized.as_str() {
        "relevance" => Ok(SortOrder::Relevance),
        "pub_date" | "publication_date" | "date" => Ok(SortOrder::PublicationDate),
        "author" | "first_author" => Ok(SortOrder::FirstAuthor),
        "journal" | "journal_name" => Ok(SortOrder::JournalName),
        _ => Err(Error::from_reason(format!(
            "Invalid sort order: '{}'. Supported values: relevance, pub_date, author, journal",
            s
        ))),
    }
}

/// Convert string to Language enum with case-insensitive matching
fn str_to_language(s: &str) -> Language {
    let normalized = s.trim().to_lowercase();

    match normalized.as_str() {
        "english" => Language::English,
        "japanese" => Language::Japanese,
        "german" => Language::German,
        "french" => Language::French,
        "spanish" => Language::Spanish,
        "italian" => Language::Italian,
        "chinese" => Language::Chinese,
        "russian" => Language::Russian,
        "portuguese" => Language::Portuguese,
        "arabic" => Language::Arabic,
        "dutch" => Language::Dutch,
        "korean" => Language::Korean,
        "polish" => Language::Polish,
        "swedish" => Language::Swedish,
        "danish" => Language::Danish,
        "norwegian" => Language::Norwegian,
        "finnish" => Language::Finnish,
        "turkish" => Language::Turkish,
        "hebrew" => Language::Hebrew,
        "czech" => Language::Czech,
        "hungarian" => Language::Hungarian,
        "greek" => Language::Greek,
        _ => Language::Other(s.trim().to_string()),
    }
}

// ================================================================================================
// SearchQuery Builder
// ================================================================================================

/// Builder for constructing PubMed search queries programmatically
///
/// Provides a fluent API for building complex PubMed search queries with support for:
/// - Basic search terms
/// - Date filtering
/// - Article type and language filtering
/// - Open access filtering
/// - Boolean logic operations (AND, OR, NOT)
/// - MeSH terms and author filtering
///
/// @example
/// ```typescript
/// const query = new SearchQuery()
///   .query("covid-19")
///   .publishedInYear(2024)
///   .articleType("Clinical Trial")
///   .freeFullTextOnly();
///
/// const articles = await client.executeQuery(query);
/// ```
#[napi]
pub struct SearchQuery {
    inner: RustSearchQuery,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl SearchQuery {
    /// Create a new empty search query builder
    #[napi(constructor)]
    pub fn new() -> Self {
        SearchQuery {
            inner: RustSearchQuery::new(),
        }
    }

    // ============================================================================================
    // Basic Methods
    // ============================================================================================

    /// Add a search term to the query
    ///
    /// Terms are accumulated and will be space-separated in the final query.
    ///
    /// @param term - Search term string
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("covid-19")
    ///   .query("treatment");
    /// query.build(); // "covid-19 treatment"
    /// ```
    #[napi]
    pub fn query(&mut self, term: String) -> &Self {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().query(trimmed);
        }
        self
    }

    /// Add multiple search terms at once
    ///
    /// Each term is processed like query(). Empty strings are filtered out.
    ///
    /// @param terms - Array of search term strings
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .terms(["covid-19", "vaccine", "efficacy"]);
    /// query.build(); // "covid-19 vaccine efficacy"
    /// ```
    #[napi]
    pub fn terms(&mut self, terms: Vec<String>) -> &Self {
        for term in terms {
            let trimmed = term.trim();
            if !trimmed.is_empty() {
                self.inner = self.inner.clone().query(trimmed);
            }
        }
        self
    }

    /// Set the maximum number of results to return
    ///
    /// @param limit - Maximum number of results (clamped to 1-10,000 range)
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .setLimit(50);
    /// ```
    #[napi]
    pub fn set_limit(&mut self, limit: u32) -> &Self {
        // Clamp the limit to valid range instead of throwing
        let clamped_limit = limit.clamp(1, 10000) as usize;
        self.inner = self.inner.clone().limit(clamped_limit);
        self
    }

    /// Build the final PubMed query string
    ///
    /// @returns Query string for PubMed E-utilities API
    /// @throws Error if no search terms have been added
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("covid-19")
    ///   .query("treatment");
    /// query.build(); // "covid-19 treatment"
    /// ```
    #[napi]
    pub fn build(&self) -> Result<String> {
        let query_string = self.inner.build();
        if query_string.trim().is_empty() {
            return Err(Error::from_reason(
                "Cannot build query: no search terms provided",
            ));
        }
        Ok(query_string)
    }

    /// Get the limit for this query
    ///
    /// @returns Maximum number of results (default: 20)
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery().query("cancer").limit(100);
    /// query.getLimit(); // 100
    /// ```
    #[napi(getter)]
    pub fn get_limit(&self) -> u32 {
        self.inner.get_limit() as u32
    }

    // ============================================================================================
    // Date Filtering Methods
    // ============================================================================================

    /// Filter to articles published in a specific year
    ///
    /// @param year - Year to filter by (must be between 1800 and 3000)
    /// @returns Self for method chaining
    /// @throws Error if year is outside valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("covid-19")
    ///   .publishedInYear(2024);
    /// ```
    #[napi]
    pub fn published_in_year(&mut self, year: u32) -> Result<&Self> {
        validate_year(year)?;
        self.inner = self.inner.clone().published_in_year(year);
        Ok(self)
    }

    /// Filter by publication date range
    ///
    /// @param startYear - Start year (inclusive)
    /// @param endYear - End year (inclusive, optional)
    /// @returns Self for method chaining
    /// @throws Error if years are outside valid range
    ///
    /// @example
    /// ```typescript
    /// // Filter to 2020-2024
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .publishedBetween(2020, 2024);
    ///
    /// // Filter from 2020 onwards
    /// const query2 = new SearchQuery()
    ///   .query("treatment")
    ///   .publishedBetween(2020);
    /// ```
    #[napi]
    pub fn published_between(&mut self, start_year: u32, end_year: Option<u32>) -> Result<&Self> {
        validate_year(start_year)?;

        if let Some(end) = end_year {
            validate_year(end)?;
            if start_year > end {
                return Err(Error::from_reason(format!(
                    "Start year ({}) must be <= end year ({})",
                    start_year, end
                )));
            }
        }

        self.inner = self.inner.clone().published_between(start_year, end_year);
        Ok(self)
    }

    /// Filter to articles published after a specific year
    ///
    /// @param year - Year after which articles were published
    /// @returns Self for method chaining
    /// @throws Error if year is outside valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("crispr")
    ///   .publishedAfter(2020);
    /// ```
    #[napi]
    pub fn published_after(&mut self, year: u32) -> Result<&Self> {
        validate_year(year)?;
        self.inner = self.inner.clone().published_after(year);
        Ok(self)
    }

    /// Filter to articles published before a specific year
    ///
    /// @param year - Year before which articles were published
    /// @returns Self for method chaining
    /// @throws Error if year is outside valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genome")
    ///   .publishedBefore(2020);
    /// ```
    #[napi]
    pub fn published_before(&mut self, year: u32) -> Result<&Self> {
        validate_year(year)?;
        self.inner = self.inner.clone().published_before(year);
        Ok(self)
    }

    // ============================================================================================
    // Article Type and Language Filtering Methods
    // ============================================================================================

    /// Filter by a single article type
    ///
    /// @param typeName - Article type (case-insensitive)
    ///   Supported types: "Clinical Trial", "Review", "Systematic Review",
    ///   "Meta-Analysis", "Case Reports", "Randomized Controlled Trial" (or "RCT"),
    ///   "Observational Study"
    /// @returns Self for method chaining
    /// @throws Error if article type is not recognized
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .articleType("Clinical Trial");
    /// ```
    #[napi]
    pub fn article_type(&mut self, type_name: String) -> Result<&Self> {
        let article_type = str_to_article_type(&type_name)?;
        self.inner = self.inner.clone().article_type(article_type);
        Ok(self)
    }

    /// Filter by multiple article types (OR logic)
    ///
    /// @param types - Array of article type names (case-insensitive)
    /// @returns Self for method chaining
    /// @throws Error if any article type is not recognized
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("treatment")
    ///   .articleTypes(["RCT", "Meta-Analysis"]);
    /// ```
    #[napi]
    pub fn article_types(&mut self, types: Vec<String>) -> Result<&Self> {
        if types.is_empty() {
            return Ok(self);
        }

        let article_types: std::result::Result<Vec<ArticleType>, Error> =
            types.iter().map(|s| str_to_article_type(s)).collect();

        let article_types = article_types?;
        self.inner = self.inner.clone().article_types(&article_types);
        Ok(self)
    }

    /// Filter by language
    ///
    /// @param lang - Language name (case-insensitive)
    ///   Supported: "English", "Japanese", "German", "French", "Spanish", etc.
    ///   Unknown languages are passed through as custom values.
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .language("English");
    /// ```
    #[napi]
    pub fn language(&mut self, lang: String) -> &Self {
        let language = str_to_language(&lang);
        self.inner = self.inner.clone().language(language);
        self
    }

    // ============================================================================================
    // Open Access Filtering Methods
    // ============================================================================================

    /// Filter to articles with free full text (open access)
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .freeFullTextOnly();
    /// ```
    #[napi]
    pub fn free_full_text_only(&mut self) -> &Self {
        self.inner = self.inner.clone().free_full_text_only();
        self
    }

    /// Filter to articles with full text links (including subscription-based)
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("diabetes")
    ///   .fullTextOnly();
    /// ```
    #[napi]
    pub fn full_text_only(&mut self) -> &Self {
        self.inner = self.inner.clone().full_text_only();
        self
    }

    /// Filter to articles with PMC full text
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genomics")
    ///   .pmcOnly();
    /// ```
    #[napi]
    pub fn pmc_only(&mut self) -> &Self {
        self.inner = self.inner.clone().pmc_only();
        self
    }

    /// Filter to articles that have abstracts
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genetics")
    ///   .hasAbstract();
    /// ```
    #[napi]
    pub fn has_abstract(&mut self) -> &Self {
        self.inner = self.inner.clone().has_abstract();
        self
    }

    // ============================================================================================
    // Field-Specific Search Methods
    // ============================================================================================

    /// Search in article titles only
    ///
    /// @param text - Title text to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .titleContains("machine learning");
    /// ```
    #[napi]
    pub fn title_contains(&mut self, text: String) -> &Self {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().title_contains(trimmed);
        }
        self
    }

    /// Search in article abstracts only
    ///
    /// @param text - Abstract text to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .abstractContains("neural networks");
    /// ```
    #[napi]
    pub fn abstract_contains(&mut self, text: String) -> &Self {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().abstract_contains(trimmed);
        }
        self
    }

    /// Search in both title and abstract
    ///
    /// @param text - Text to search for in title or abstract
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .titleOrAbstract("CRISPR gene editing");
    /// ```
    #[napi]
    pub fn title_or_abstract(&mut self, text: String) -> &Self {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().title_or_abstract(trimmed);
        }
        self
    }

    /// Filter by journal name
    ///
    /// @param name - Journal name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .journal("Nature");
    /// ```
    #[napi]
    pub fn journal(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().journal(trimmed);
        }
        self
    }

    /// Filter by grant number
    ///
    /// @param grantNumber - Grant number to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .grantNumber("R01AI123456");
    /// ```
    #[napi]
    pub fn grant_number(&mut self, grant_number: String) -> &Self {
        let trimmed = grant_number.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().grant_number(trimmed);
        }
        self
    }

    // ============================================================================================
    // Advanced Search Methods (MeSH, Author, etc.)
    // ============================================================================================

    /// Filter by MeSH term
    ///
    /// @param term - MeSH term to filter by
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshTerm("Neoplasms");
    /// ```
    #[napi]
    pub fn mesh_term(&mut self, term: String) -> &Self {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().mesh_term(trimmed);
        }
        self
    }

    /// Filter by MeSH major topic
    ///
    /// @param term - MeSH term to filter by as a major topic
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshMajorTopic("Diabetes Mellitus, Type 2");
    /// ```
    #[napi]
    pub fn mesh_major_topic(&mut self, term: String) -> &Self {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().mesh_major_topic(trimmed);
        }
        self
    }

    /// Filter by multiple MeSH terms
    ///
    /// @param terms - Array of MeSH terms to filter by
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshTerms(["Neoplasms", "Antineoplastic Agents"]);
    /// ```
    #[napi]
    pub fn mesh_terms(&mut self, terms: Vec<String>) -> &Self {
        let valid_terms: Vec<&str> = terms
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !valid_terms.is_empty() {
            self.inner = self.inner.clone().mesh_terms(&valid_terms);
        }
        self
    }

    /// Filter by MeSH subheading
    ///
    /// @param subheading - MeSH subheading to filter by
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshTerm("Diabetes Mellitus")
    ///   .meshSubheading("drug therapy");
    /// ```
    #[napi]
    pub fn mesh_subheading(&mut self, subheading: String) -> &Self {
        let trimmed = subheading.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().mesh_subheading(trimmed);
        }
        self
    }

    /// Filter by author name
    ///
    /// @param name - Author name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("machine learning")
    ///   .author("Williams K");
    /// ```
    #[napi]
    pub fn author(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().author(trimmed);
        }
        self
    }

    /// Filter by first author
    ///
    /// @param name - First author name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .firstAuthor("Smith J");
    /// ```
    #[napi]
    pub fn first_author(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().first_author(trimmed);
        }
        self
    }

    /// Filter by last author
    ///
    /// @param name - Last author name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genomics")
    ///   .lastAuthor("Johnson M");
    /// ```
    #[napi]
    pub fn last_author(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().last_author(trimmed);
        }
        self
    }

    /// Filter by institution/affiliation
    ///
    /// @param institution - Institution name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cardiology")
    ///   .affiliation("Harvard Medical School");
    /// ```
    #[napi]
    pub fn affiliation(&mut self, institution: String) -> &Self {
        let trimmed = institution.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().affiliation(trimmed);
        }
        self
    }

    /// Filter by ORCID identifier
    ///
    /// @param orcidId - ORCID identifier to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .orcid("0000-0001-2345-6789");
    /// ```
    #[napi]
    pub fn orcid(&mut self, orcid_id: String) -> &Self {
        let trimmed = orcid_id.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().orcid(trimmed);
        }
        self
    }

    /// Filter to human studies only
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("drug treatment")
    ///   .humanStudiesOnly();
    /// ```
    #[napi]
    pub fn human_studies_only(&mut self) -> &Self {
        self.inner = self.inner.clone().human_studies_only();
        self
    }

    /// Filter to animal studies only
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("preclinical research")
    ///   .animalStudiesOnly();
    /// ```
    #[napi]
    pub fn animal_studies_only(&mut self) -> &Self {
        self.inner = self.inner.clone().animal_studies_only();
        self
    }

    /// Filter by age group
    ///
    /// @param ageGroup - Age group to filter by (e.g., "Child", "Adult", "Aged")
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("pediatric medicine")
    ///   .ageGroup("Child");
    /// ```
    #[napi]
    pub fn age_group(&mut self, age_group: String) -> &Self {
        let trimmed = age_group.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().age_group(trimmed);
        }
        self
    }

    /// Add a custom filter
    ///
    /// @param filter - Custom filter string in PubMed syntax
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("research")
    ///   .customFilter("humans[mh]");
    /// ```
    #[napi]
    pub fn custom_filter(&mut self, filter: String) -> &Self {
        let trimmed = filter.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().custom_filter(trimmed);
        }
        self
    }

    // ============================================================================================
    // Boolean Logic Methods
    // ============================================================================================

    /// Combine this query with another using AND logic
    ///
    /// @param other - Another SearchQuery to combine with
    /// @returns New SearchQuery with combined logic
    ///
    /// @example
    /// ```typescript
    /// const q1 = new SearchQuery().query("covid-19");
    /// const q2 = new SearchQuery().query("vaccine");
    /// const combined = q1.and(q2);
    /// combined.build(); // "(covid-19) AND (vaccine)"
    /// ```
    #[napi]
    pub fn and(&self, other: &SearchQuery) -> SearchQuery {
        let combined = self.inner.clone().and(other.inner.clone());
        SearchQuery { inner: combined }
    }

    /// Combine this query with another using OR logic
    ///
    /// @param other - Another SearchQuery to combine with
    /// @returns New SearchQuery with combined logic
    ///
    /// @example
    /// ```typescript
    /// const q1 = new SearchQuery().query("diabetes");
    /// const q2 = new SearchQuery().query("hypertension");
    /// const combined = q1.or(q2);
    /// combined.build(); // "(diabetes) OR (hypertension)"
    /// ```
    #[napi]
    pub fn or(&self, other: &SearchQuery) -> SearchQuery {
        let combined = self.inner.clone().or(other.inner.clone());
        SearchQuery { inner: combined }
    }

    /// Negate this query using NOT logic
    ///
    /// @returns New SearchQuery with NOT logic
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery().query("cancer").negate();
    /// query.build(); // "NOT (cancer)"
    /// ```
    #[napi]
    pub fn negate(&self) -> SearchQuery {
        let negated = self.inner.clone().negate();
        SearchQuery { inner: negated }
    }

    /// Exclude articles matching the given query
    ///
    /// @param excluded - SearchQuery representing articles to exclude
    /// @returns New SearchQuery with exclusion logic
    ///
    /// @example
    /// ```typescript
    /// const base = new SearchQuery().query("cancer treatment");
    /// const exclude = new SearchQuery().query("animal studies");
    /// const filtered = base.exclude(exclude);
    /// filtered.build(); // "(cancer treatment) NOT (animal studies)"
    /// ```
    #[napi]
    pub fn exclude(&self, excluded: &SearchQuery) -> SearchQuery {
        let filtered = self.inner.clone().exclude(excluded.inner.clone());
        SearchQuery { inner: filtered }
    }

    /// Add parentheses around the current query for grouping
    ///
    /// @returns New SearchQuery wrapped in parentheses
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .or(new SearchQuery().query("tumor"))
    ///   .group();
    /// query.build(); // "((cancer) OR (tumor))"
    /// ```
    #[napi]
    pub fn group(&self) -> SearchQuery {
        let grouped = self.inner.clone().group();
        SearchQuery { inner: grouped }
    }

    // ============================================================================================
    // Sort Methods
    // ============================================================================================

    /// Set the sort order for search results
    ///
    /// @param sortOrder - Sort order (case-insensitive)
    ///   Supported: "relevance", "pub_date", "author", "journal"
    /// @returns Self for method chaining
    /// @throws Error if sort order is not recognized
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .sort("pub_date");
    /// ```
    #[napi]
    pub fn sort(&mut self, sort_order: String) -> Result<&Self> {
        let sort = str_to_sort_order(&sort_order)?;
        self.inner = self.inner.clone().sort(sort);
        Ok(self)
    }
}
