use napi::bindgen_prelude::*;
use napi_derive::napi;
use pubmed_client::{Client, ClientConfig, pmc::markdown::PmcMarkdownConverter};
use std::sync::Arc;

use crate::config::Config;
use crate::error::to_napi_err;
use crate::models::{
    Article, CitationMatch, CitationQuery, Citations, DatabaseInfo, EPostResult, ExtractedFigure,
    FullTextArticle, GlobalQueryResults, MarkdownOptions, OaSubsetInfo, PmcLinks, RelatedArticles,
    SpellCheckResult, Summary,
};
use crate::query::SearchQuery;

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
            .search_and_fetch(&query, limit, None)
            .await
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)?;

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
            .search_and_fetch_summaries(&query, limit, None)
            .await
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)
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
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)?;

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
            .map_err(to_napi_err)?;

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
            .search_and_fetch(&query_string, limit, None)
            .await
            .map_err(to_napi_err)?;

        Ok(articles.into_iter().map(Article::from).collect())
    }

    /// Upload a list of PMIDs to the NCBI History server using EPost
    ///
    /// Stores UIDs on the server and returns WebEnv/query_key identifiers
    /// that can be used with subsequent API calls.
    ///
    /// @param pmids - Array of PubMed IDs as strings
    /// @returns EPostResult containing webenv and query_key
    ///
    /// @example
    /// ```typescript
    /// const client = new PubMedClient();
    /// const result = await client.epost(["31978945", "33515491", "25760099"]);
    /// console.log(`WebEnv: ${result.webenv}, Query Key: ${result.queryKey}`);
    /// ```
    #[napi]
    pub async fn epost(&self, pmids: Vec<String>) -> Result<EPostResult> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let result = self
            .client
            .pubmed
            .epost(&pmid_refs)
            .await
            .map_err(to_napi_err)?;

        Ok(EPostResult::from(result))
    }

    /// Fetch all articles for a list of PMIDs using EPost and the History server
    ///
    /// Uploads the PMID list via EPost (HTTP POST), then fetches articles in
    /// paginated batches. Recommended for large PMID lists (hundreds or thousands).
    ///
    /// @param pmids - Array of PubMed IDs as strings
    /// @returns Array of article metadata
    ///
    /// @example
    /// ```typescript
    /// const client = new PubMedClient();
    /// const articles = await client.fetchAllByPmids(["31978945", "33515491", "25760099"]);
    /// articles.forEach(a => console.log(a.title));
    /// ```
    #[napi]
    pub async fn fetch_all_by_pmids(&self, pmids: Vec<String>) -> Result<Vec<Article>> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let articles = self
            .client
            .pubmed
            .fetch_all_by_pmids(&pmid_refs)
            .await
            .map_err(to_napi_err)?;

        Ok(articles.into_iter().map(Article::from).collect())
    }

    /// Get related articles for the given PMIDs using the ELink API
    ///
    /// @param pmids - Array of PubMed IDs
    /// @returns Related articles
    #[napi]
    pub async fn get_related_articles(&self, pmids: Vec<u32>) -> Result<RelatedArticles> {
        let related = self
            .client
            .get_related_articles(&pmids)
            .await
            .map_err(to_napi_err)?;

        Ok(RelatedArticles::from(related))
    }

    /// Get PMC links for the given PMIDs (check full-text availability)
    ///
    /// @param pmids - Array of PubMed IDs
    /// @returns PMC links
    #[napi]
    pub async fn get_pmc_links(&self, pmids: Vec<u32>) -> Result<PmcLinks> {
        let links = self
            .client
            .get_pmc_links(&pmids)
            .await
            .map_err(to_napi_err)?;

        Ok(PmcLinks::from(links))
    }

    /// Get citing articles for the given PMIDs using the ELink API
    ///
    /// @param pmids - Array of PubMed IDs
    /// @returns Citing articles
    #[napi]
    pub async fn get_citations(&self, pmids: Vec<u32>) -> Result<Citations> {
        let citations = self
            .client
            .get_citations(&pmids)
            .await
            .map_err(to_napi_err)?;

        Ok(Citations::from(citations))
    }

    /// List all available NCBI databases using the EInfo API
    ///
    /// @returns Array of database names
    #[napi]
    pub async fn get_database_list(&self) -> Result<Vec<String>> {
        self.client.get_database_list().await.map_err(to_napi_err)
    }

    /// Get detailed information about a specific NCBI database using the EInfo API
    ///
    /// @param database - Database name (e.g., "pubmed", "pmc")
    /// @returns Database information
    #[napi]
    pub async fn get_database_info(&self, database: String) -> Result<DatabaseInfo> {
        let info = self
            .client
            .get_database_info(&database)
            .await
            .map_err(to_napi_err)?;

        Ok(DatabaseInfo::from(info))
    }

    /// Match citations to PMIDs using the ECitMatch API
    ///
    /// @param citations - Array of citation queries
    /// @returns Array of citation match results
    #[napi]
    pub async fn match_citations(
        &self,
        citations: Vec<CitationQuery>,
    ) -> Result<Vec<CitationMatch>> {
        let rust_citations: Vec<pubmed_client::CitationQuery> = citations
            .iter()
            .map(pubmed_client::CitationQuery::from)
            .collect();
        let results = self
            .client
            .match_citations(&rust_citations)
            .await
            .map_err(to_napi_err)?;

        Ok(results.matches.iter().map(CitationMatch::from).collect())
    }

    /// Query all NCBI databases for record counts using the EGQuery API
    ///
    /// @param term - Search term
    /// @returns Per-database record counts
    #[napi]
    pub async fn global_query(&self, term: String) -> Result<GlobalQueryResults> {
        let results = self.client.global_query(&term).await.map_err(to_napi_err)?;

        Ok(GlobalQueryResults::from(results))
    }

    /// Export articles as BibTeX
    ///
    /// Fetches the given PMIDs and formats them as a BibTeX bibliography.
    ///
    /// @param pmids - Array of PubMed IDs as strings
    /// @returns BibTeX string
    #[napi]
    pub async fn export_bibtex(&self, pmids: Vec<String>) -> Result<String> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let articles = self
            .client
            .pubmed
            .fetch_articles(&pmid_refs)
            .await
            .map_err(to_napi_err)?;

        Ok(pubmed_client::export::articles_to_bibtex(&articles))
    }

    /// Export articles in RIS format
    ///
    /// @param pmids - Array of PubMed IDs as strings
    /// @returns RIS string
    #[napi]
    pub async fn export_ris(&self, pmids: Vec<String>) -> Result<String> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let articles = self
            .client
            .pubmed
            .fetch_articles(&pmid_refs)
            .await
            .map_err(to_napi_err)?;

        Ok(pubmed_client::export::articles_to_ris(&articles))
    }

    /// Export articles as CSL-JSON
    ///
    /// @param pmids - Array of PubMed IDs as strings
    /// @returns CSL-JSON string (array of items)
    #[napi]
    pub async fn export_csl_json(&self, pmids: Vec<String>) -> Result<String> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let articles = self
            .client
            .pubmed
            .fetch_articles(&pmid_refs)
            .await
            .map_err(to_napi_err)?;

        Ok(pubmed_client::export::articles_to_csl_json(&articles).to_string())
    }

    /// Export articles in MEDLINE/NBIB format
    ///
    /// @param pmids - Array of PubMed IDs as strings
    /// @returns NBIB string
    #[napi]
    pub async fn export_nbib(&self, pmids: Vec<String>) -> Result<String> {
        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        let articles = self
            .client
            .pubmed
            .fetch_articles(&pmid_refs)
            .await
            .map_err(to_napi_err)?;

        let nbib = articles
            .iter()
            .map(pubmed_client::ExportFormat::to_nbib)
            .collect::<Vec<_>>()
            .join("\n");
        Ok(nbib)
    }

    /// Download a PMC article's Open Access files to a directory
    ///
    /// Downloads each of the article's files individually from the PMC OA Cloud
    /// (AWS S3) service for the given PMC ID, returning the list of downloaded
    /// file paths.
    ///
    /// @param pmcid - PMC ID (e.g., "PMC7906746")
    /// @param outputDir - Directory to download files into
    /// @returns Array of downloaded file paths
    #[napi]
    pub async fn download_files(&self, pmcid: String, output_dir: String) -> Result<Vec<String>> {
        self.client
            .pmc
            .download_files(&pmcid, &output_dir)
            .await
            .map_err(to_napi_err)
    }

    /// Extract figures with their captions from a PMC article
    ///
    /// Downloads the Open Access package, extracts figure image files, and
    /// associates them with caption metadata from the article XML.
    ///
    /// @param pmcid - PMC ID (e.g., "PMC7906746")
    /// @param outputDir - Directory to extract figure files into
    /// @returns Array of extracted figures with file metadata
    #[napi]
    pub async fn extract_figures_with_captions(
        &self,
        pmcid: String,
        output_dir: String,
    ) -> Result<Vec<ExtractedFigure>> {
        let figures = self
            .client
            .pmc
            .extract_figures_with_captions(&pmcid, &output_dir)
            .await
            .map_err(to_napi_err)?;

        Ok(figures.iter().map(ExtractedFigure::from).collect())
    }
}
