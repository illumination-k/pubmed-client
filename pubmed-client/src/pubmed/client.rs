use std::time::Duration;

use crate::common::PubMedId;
use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pubmed::models::{
    Citations, DatabaseInfo, FieldInfo, HistorySession, LinkInfo, PmcLinks, PubMedArticle,
    RelatedArticles, SearchResult,
};
use crate::pubmed::parser::parse_articles_from_xml;
use crate::pubmed::responses::{EInfoResponse, ELinkResponse, ESearchResult};
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;
use reqwest::{Client, Response};
use tracing::{debug, info, instrument, warn};

/// State machine for streaming search results
#[cfg(not(target_arch = "wasm32"))]
enum SearchAllState {
    /// Initial state before search
    Initial { query: String, batch_size: usize },
    /// Fetching articles from history server
    Fetching {
        session: HistorySession,
        total: usize,
        batch_size: usize,
        current_offset: usize,
        pending_articles: Vec<PubMedArticle>,
        article_index: usize,
    },
    /// All articles have been fetched
    Done,
}

/// Client for interacting with PubMed API
#[derive(Clone)]
pub struct PubMedClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
}

impl PubMedClient {
    /// Create a search query builder for this client
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client
    ///         .search()
    ///         .query("covid-19 treatment")
    ///         .open_access_only()
    ///         .published_after(2020)
    ///         .limit(10)
    ///         .search_and_fetch(&client)
    ///         .await?;
    ///
    ///     println!("Found {} articles", articles.len());
    ///     Ok(())
    /// }
    /// ```
    pub fn search(&self) -> super::query::SearchQuery {
        super::query::SearchQuery::new()
    }

    /// Create a new PubMed client with default configuration
    ///
    /// Uses default NCBI rate limiting (3 requests/second) and no API key.
    /// For production use, consider using `with_config()` to set an API key.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// let client = PubMedClient::new();
    /// ```
    pub fn new() -> Self {
        let config = ClientConfig::new();
        Self::with_config(config)
    }

    /// Create a new PubMed client with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration including rate limits, API key, etc.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::{PubMedClient, ClientConfig};
    ///
    /// let config = ClientConfig::new()
    ///     .with_api_key("your_api_key_here")
    ///     .with_email("researcher@university.edu");
    ///
    /// let client = PubMedClient::with_config(config);
    /// ```
    pub fn with_config(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();
        let base_url = config.effective_base_url().to_string();

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
            base_url,
            rate_limiter,
            config,
        }
    }

    /// Create a new PubMed client with custom HTTP client and default configuration
    ///
    /// # Arguments
    ///
    /// * `client` - Custom reqwest client with specific configuration
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::PubMedClient;
    /// use reqwest::Client;
    /// use std::time::Duration;
    ///
    /// let http_client = Client::builder()
    ///     .timeout(Duration::from_secs(30))
    ///     .build()
    ///     .unwrap();
    ///
    /// let client = PubMedClient::with_client(http_client);
    /// ```
    pub fn with_client(client: Client) -> Self {
        let config = ClientConfig::new();
        let rate_limiter = config.create_rate_limiter();
        let base_url = config.effective_base_url().to_string();

        Self {
            client,
            base_url,
            rate_limiter,
            config,
        }
    }

    /// Fetch article metadata by PMID with full details including abstract
    ///
    /// # Arguments
    ///
    /// * `pmid` - PubMed ID as a string
    ///
    /// # Returns
    ///
    /// Returns a `Result<PubMedArticle>` containing the article metadata with abstract
    ///
    /// # Errors
    ///
    /// * `PubMedError::ArticleNotFound` - If the article is not found
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let article = client.fetch_article("31978945").await?;
    ///     println!("Title: {}", article.title);
    ///     if let Some(abstract_text) = &article.abstract_text {
    ///         println!("Abstract: {}", abstract_text);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmid = %pmid))]
    pub async fn fetch_article(&self, pmid: &str) -> Result<PubMedArticle> {
        let mut articles = self.fetch_articles(&[pmid]).await?;

        if articles.len() == 1 {
            Ok(articles.remove(0))
        } else {
            // Try to find by PMID in case batch returned extra/different articles
            let idx = articles.iter().position(|a| a.pmid == pmid);
            match idx {
                Some(i) => Ok(articles.remove(i)),
                None => Err(PubMedError::ArticleNotFound {
                    pmid: pmid.to_string(),
                }),
            }
        }
    }

    /// Search for articles using a query string
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing PMIDs of matching articles
    ///
    /// # Errors
    ///
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let pmids = client.search_articles("covid-19 treatment", 10).await?;
    ///     println!("Found {} articles", pmids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(query = %query, limit = limit))]
    pub async fn search_articles(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        // PubMed limits: retstart cannot exceed 9998, and retmax is capped at 9999
        // This means we can only retrieve the first 9,999 results (indices 0-9998)
        const MAX_RETRIEVABLE: usize = 9999;

        if limit > MAX_RETRIEVABLE {
            return Err(PubMedError::SearchLimitExceeded {
                requested: limit,
                maximum: MAX_RETRIEVABLE,
            });
        }

        if query.trim().is_empty() {
            debug!("Empty query provided, returning empty results");
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/esearch.fcgi?db=pubmed&term={}&retmax={}&retstart={}&retmode=json",
            self.base_url,
            urlencoding::encode(query),
            limit,
            0
        );

        debug!("Making initial ESearch API request");
        let response = self.make_request(&url).await?;

        let search_result: ESearchResult = response.json().await?;

        // Check for API error response (NCBI sometimes returns 200 OK with ERROR field)
        if let Some(error_msg) = &search_result.esearchresult.error {
            return Err(PubMedError::ApiError {
                status: 200,
                message: format!("NCBI ESearch API error: {}", error_msg),
            });
        }

        let total_count: usize = search_result
            .esearchresult
            .count
            .as_ref()
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);

        if total_count >= limit {
            warn!(
                "Total results ({}) exceed or equal requested limit ({}). Only the first {} results can be retrieved.",
                total_count, limit, MAX_RETRIEVABLE
            );
        }

        Ok(search_result.esearchresult.idlist)
    }

    /// Fetch multiple articles by PMIDs in a single batch request
    ///
    /// This method sends a single EFetch request with multiple PMIDs (comma-separated),
    /// which is significantly more efficient than fetching articles one by one.
    /// For large numbers of PMIDs, the request is automatically split into batches.
    ///
    /// # Arguments
    ///
    /// * `pmids` - Slice of PubMed IDs as strings
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<PubMedArticle>>` containing articles with metadata.
    /// Articles that fail to parse are skipped (logged via tracing).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client.fetch_articles(&["31978945", "33515491", "25760099"]).await?;
    ///     for article in &articles {
    ///         println!("{}: {}", article.pmid, article.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn fetch_articles(&self, pmids: &[&str]) -> Result<Vec<PubMedArticle>> {
        if pmids.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all PMIDs upfront
        let validated: Vec<u32> = pmids
            .iter()
            .map(|pmid| PubMedId::parse(pmid).map(|p| p.as_u32()))
            .collect::<Result<Vec<_>>>()?;

        // NCBI recommends batches of up to 200 IDs per request
        const BATCH_SIZE: usize = 200;

        let mut all_articles = Vec::with_capacity(pmids.len());

        for chunk in validated.chunks(BATCH_SIZE) {
            let id_list: String = chunk
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let url = format!(
                "{}/efetch.fcgi?db=pubmed&id={}&retmode=xml&rettype=abstract",
                self.base_url, id_list
            );

            debug!(batch_size = chunk.len(), "Making batch EFetch API request");
            let response = self.make_request(&url).await?;
            let xml_text = response.text().await?;

            if xml_text.trim().is_empty() {
                continue;
            }

            let articles = parse_articles_from_xml(&xml_text)?;
            info!(
                requested = chunk.len(),
                parsed = articles.len(),
                "Batch fetch completed"
            );
            all_articles.extend(articles);
        }

        Ok(all_articles)
    }

    /// Search and fetch multiple articles with metadata
    ///
    /// Uses batch fetching internally for efficient retrieval.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of articles to fetch
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<PubMedArticle>>` containing articles with metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = client.search_and_fetch("covid-19", 5).await?;
    ///     for article in articles {
    ///         println!("{}: {}", article.pmid, article.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_and_fetch(&self, query: &str, limit: usize) -> Result<Vec<PubMedArticle>> {
        let pmids = self.search_articles(query, limit).await?;

        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        self.fetch_articles(&pmid_refs).await
    }

    /// Search for articles with history server support
    ///
    /// This method enables NCBI's history server feature, which stores search results
    /// on the server and returns WebEnv/query_key identifiers. These can be used
    /// with `fetch_from_history()` to efficiently paginate through large result sets.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of PMIDs to return in the initial response
    ///
    /// # Returns
    ///
    /// Returns a `Result<SearchResult>` containing PMIDs and history session information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let result = client.search_with_history("covid-19", 100).await?;
    ///
    ///     println!("Total results: {}", result.total_count);
    ///     println!("First batch: {} PMIDs", result.pmids.len());
    ///
    ///     // Use history session to fetch more results
    ///     if let Some(session) = result.history_session() {
    ///         let next_batch = client.fetch_from_history(&session, 100, 100).await?;
    ///         println!("Next batch: {} articles", next_batch.len());
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(query = %query, limit = limit))]
    pub async fn search_with_history(&self, query: &str, limit: usize) -> Result<SearchResult> {
        if query.trim().is_empty() {
            debug!("Empty query provided, returning empty results");
            return Ok(SearchResult {
                pmids: Vec::new(),
                total_count: 0,
                webenv: None,
                query_key: None,
            });
        }

        // Use usehistory=y to enable history server
        let url = format!(
            "{}/esearch.fcgi?db=pubmed&term={}&retmax={}&retstart={}&retmode=json&usehistory=y",
            self.base_url,
            urlencoding::encode(query),
            limit,
            0
        );

        debug!("Making ESearch API request with history");
        let response = self.make_request(&url).await?;

        let search_result: ESearchResult = response.json().await?;

        // Check for API error response
        if let Some(error_msg) = &search_result.esearchresult.error {
            return Err(PubMedError::ApiError {
                status: 200,
                message: format!("NCBI ESearch API error: {}", error_msg),
            });
        }

        let total_count: usize = search_result
            .esearchresult
            .count
            .as_ref()
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);

        info!(
            total_count = total_count,
            returned_count = search_result.esearchresult.idlist.len(),
            has_webenv = search_result.esearchresult.webenv.is_some(),
            "Search with history completed"
        );

        Ok(SearchResult {
            pmids: search_result.esearchresult.idlist,
            total_count,
            webenv: search_result.esearchresult.webenv,
            query_key: search_result.esearchresult.query_key,
        })
    }

    /// Fetch articles from history server using WebEnv session
    ///
    /// This method retrieves articles from a previously executed search using
    /// the history server. It's useful for paginating through large result sets
    /// without re-running the search query.
    ///
    /// # Arguments
    ///
    /// * `session` - History session containing WebEnv and query_key
    /// * `start` - Starting index (0-based) for pagination
    /// * `max` - Maximum number of articles to fetch
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<PubMedArticle>>` containing the fetched articles
    ///
    /// # Note
    ///
    /// WebEnv sessions typically expire after 1 hour of inactivity.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///
    ///     // First, search with history
    ///     let result = client.search_with_history("cancer treatment", 100).await?;
    ///
    ///     if let Some(session) = result.history_session() {
    ///         // Fetch articles 100-199
    ///         let batch2 = client.fetch_from_history(&session, 100, 100).await?;
    ///         println!("Fetched {} articles", batch2.len());
    ///
    ///         // Fetch articles 200-299
    ///         let batch3 = client.fetch_from_history(&session, 200, 100).await?;
    ///         println!("Fetched {} more articles", batch3.len());
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(start = start, max = max))]
    pub async fn fetch_from_history(
        &self,
        session: &HistorySession,
        start: usize,
        max: usize,
    ) -> Result<Vec<PubMedArticle>> {
        // Use WebEnv and query_key to fetch from history server
        let url = format!(
            "{}/efetch.fcgi?db=pubmed&query_key={}&WebEnv={}&retstart={}&retmax={}&retmode=xml&rettype=abstract",
            self.base_url,
            urlencoding::encode(&session.query_key),
            urlencoding::encode(&session.webenv),
            start,
            max
        );

        debug!("Making EFetch API request from history");
        let response = self.make_request(&url).await?;

        let xml_text = response.text().await?;

        // Check for empty response or error
        if xml_text.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Check for NCBI error response
        if xml_text.contains("<ERROR>") {
            let error_msg = xml_text
                .split("<ERROR>")
                .nth(1)
                .and_then(|s| s.split("</ERROR>").next())
                .unwrap_or("Unknown error");

            return Err(PubMedError::HistorySessionError(error_msg.to_string()));
        }

        // Parse multiple articles from XML using serde-based parser
        let articles = parse_articles_from_xml(&xml_text)?;

        info!(
            fetched_count = articles.len(),
            start = start,
            "Fetched articles from history"
        );

        Ok(articles)
    }

    /// Search and stream all matching articles using history server
    ///
    /// This method performs a search and returns a stream that automatically
    /// paginates through all results using the NCBI history server. It's ideal
    /// for processing large result sets without loading all articles into memory.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `batch_size` - Number of articles to fetch per batch (recommended: 100-500)
    ///
    /// # Returns
    ///
    /// Returns a `Stream` that yields `Result<PubMedArticle>` for each article
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    /// use futures_util::StreamExt;
    /// use std::pin::pin;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///
    ///     let stream = client.search_all("cancer biomarker", 100);
    ///     let mut stream = pin!(stream);
    ///     let mut count = 0;
    ///
    ///     while let Some(result) = stream.next().await {
    ///         match result {
    ///             Ok(article) => {
    ///                 count += 1;
    ///                 println!("{}: {}", article.pmid, article.title);
    ///             }
    ///             Err(e) => eprintln!("Error: {}", e),
    ///         }
    ///
    ///         // Stop after 1000 articles
    ///         if count >= 1000 {
    ///             break;
    ///         }
    ///     }
    ///
    ///     println!("Processed {} articles", count);
    ///     Ok(())
    /// }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn search_all(
        &self,
        query: &str,
        batch_size: usize,
    ) -> impl futures_util::Stream<Item = Result<PubMedArticle>> + '_ {
        use futures_util::stream;

        let query = query.to_string();
        let batch_size = batch_size.max(1); // Ensure at least 1

        stream::unfold(
            SearchAllState::Initial { query, batch_size },
            move |state| async move {
                match state {
                    SearchAllState::Initial { query, batch_size } => {
                        // Perform initial search with history
                        match self.search_with_history(&query, batch_size).await {
                            Ok(result) => {
                                let session = result.history_session();
                                let total = result.total_count;

                                if result.pmids.is_empty() {
                                    return None;
                                }

                                // Fetch first batch of articles
                                match session {
                                    Some(session) => {
                                        match self.fetch_from_history(&session, 0, batch_size).await
                                        {
                                            Ok(articles) => {
                                                let next_state = SearchAllState::Fetching {
                                                    session,
                                                    total,
                                                    batch_size,
                                                    current_offset: batch_size,
                                                    pending_articles: articles,
                                                    article_index: 0,
                                                };
                                                self.next_article_from_state(next_state)
                                            }
                                            Err(e) => Some((Err(e), SearchAllState::Done)),
                                        }
                                    }
                                    None => {
                                        // No history session, can't stream
                                        Some((
                                            Err(PubMedError::WebEnvNotAvailable),
                                            SearchAllState::Done,
                                        ))
                                    }
                                }
                            }
                            Err(e) => Some((Err(e), SearchAllState::Done)),
                        }
                    }
                    SearchAllState::Fetching {
                        session,
                        total,
                        batch_size,
                        current_offset,
                        pending_articles,
                        article_index,
                    } => {
                        if article_index < pending_articles.len() {
                            // Return next article from current batch
                            let article = pending_articles[article_index].clone();
                            Some((
                                Ok(article),
                                SearchAllState::Fetching {
                                    session,
                                    total,
                                    batch_size,
                                    current_offset,
                                    pending_articles,
                                    article_index: article_index + 1,
                                },
                            ))
                        } else if current_offset < total {
                            // Fetch next batch
                            match self
                                .fetch_from_history(&session, current_offset, batch_size)
                                .await
                            {
                                Ok(articles) => {
                                    if articles.is_empty() {
                                        return None;
                                    }
                                    let next_state = SearchAllState::Fetching {
                                        session,
                                        total,
                                        batch_size,
                                        current_offset: current_offset + batch_size,
                                        pending_articles: articles,
                                        article_index: 0,
                                    };
                                    self.next_article_from_state(next_state)
                                }
                                Err(e) => Some((Err(e), SearchAllState::Done)),
                            }
                        } else {
                            // All done
                            None
                        }
                    }
                    SearchAllState::Done => None,
                }
            },
        )
    }

    /// Helper to get next article from state
    #[cfg(not(target_arch = "wasm32"))]
    fn next_article_from_state(
        &self,
        state: SearchAllState,
    ) -> Option<(Result<PubMedArticle>, SearchAllState)> {
        match state {
            SearchAllState::Fetching {
                ref pending_articles,
                article_index,
                ..
            } if article_index < pending_articles.len() => {
                let article = pending_articles[article_index].clone();
                let SearchAllState::Fetching {
                    session,
                    total,
                    batch_size,
                    current_offset,
                    pending_articles,
                    article_index,
                } = state
                else {
                    unreachable!()
                };
                Some((
                    Ok(article),
                    SearchAllState::Fetching {
                        session,
                        total,
                        batch_size,
                        current_offset,
                        pending_articles,
                        article_index: article_index + 1,
                    },
                ))
            }
            _ => None,
        }
    }

    /// Get list of all available NCBI databases
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing names of all available databases
    ///
    /// # Errors
    ///
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let databases = client.get_database_list().await?;
    ///     println!("Available databases: {:?}", databases);
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn get_database_list(&self) -> Result<Vec<String>> {
        // Build URL - API parameters will be added by make_request
        let url = format!("{}/einfo.fcgi?retmode=json", self.base_url);

        debug!("Making EInfo API request for database list");
        let response = self.make_request(&url).await?;

        let einfo_response: EInfoResponse = response.json().await?;

        let db_list = einfo_response.einfo_result.db_list.unwrap_or_default();

        info!(
            databases_found = db_list.len(),
            "Database list retrieved successfully"
        );

        Ok(db_list)
    }

    /// Get detailed information about a specific database
    ///
    /// # Arguments
    ///
    /// * `database` - Name of the database (e.g., "pubmed", "pmc", "books")
    ///
    /// # Returns
    ///
    /// Returns a `Result<DatabaseInfo>` containing detailed database information
    ///
    /// # Errors
    ///
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    /// * `PubMedError::ApiError` - If the database doesn't exist
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let db_info = client.get_database_info("pubmed").await?;
    ///     println!("Database: {}", db_info.name);
    ///     println!("Description: {}", db_info.description);
    ///     println!("Fields: {}", db_info.fields.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(database = %database))]
    pub async fn get_database_info(&self, database: &str) -> Result<DatabaseInfo> {
        if database.trim().is_empty() {
            return Err(PubMedError::ApiError {
                status: 400,
                message: "Database name cannot be empty".to_string(),
            });
        }

        // Build URL - API parameters will be added by make_request
        let url = format!(
            "{}/einfo.fcgi?db={}&retmode=json",
            self.base_url,
            urlencoding::encode(database)
        );

        debug!("Making EInfo API request for database details");
        let response = self.make_request(&url).await?;

        let einfo_response: EInfoResponse = response.json().await?;

        let db_info_list =
            einfo_response
                .einfo_result
                .db_info
                .ok_or_else(|| PubMedError::ApiError {
                    status: 404,
                    message: format!("Database '{database}' not found or no information available"),
                })?;

        let db_info = db_info_list
            .into_iter()
            .next()
            .ok_or_else(|| PubMedError::ApiError {
                status: 404,
                message: format!("Database '{database}' information not found"),
            })?;

        // Convert internal response to public model
        let fields = db_info
            .field_list
            .unwrap_or_default()
            .into_iter()
            .map(|field| FieldInfo {
                name: field.name,
                full_name: field.full_name,
                description: field.description,
                term_count: field.term_count.and_then(|s| s.parse().ok()),
                is_date: field.is_date.as_deref() == Some("Y"),
                is_numerical: field.is_numerical.as_deref() == Some("Y"),
                single_token: field.single_token.as_deref() == Some("Y"),
                hierarchy: field.hierarchy.as_deref() == Some("Y"),
                is_hidden: field.is_hidden.as_deref() == Some("Y"),
            })
            .collect();

        let links = db_info
            .link_list
            .unwrap_or_default()
            .into_iter()
            .map(|link| LinkInfo {
                name: link.name,
                menu: link.menu,
                description: link.description,
                target_db: link.db_to,
            })
            .collect();

        let database_info = DatabaseInfo {
            name: db_info.db_name,
            menu_name: db_info.menu_name,
            description: db_info.description,
            build: db_info.db_build,
            count: db_info.count.and_then(|s| s.parse().ok()),
            last_update: db_info.last_update,
            fields,
            links,
        };

        info!(
            fields_count = database_info.fields.len(),
            links_count = database_info.links.len(),
            "Database information retrieved successfully"
        );

        Ok(database_info)
    }

    /// Get related articles for given PMIDs
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to find related articles for
    ///
    /// # Returns
    ///
    /// Returns a `Result<RelatedArticles>` containing related article information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let related = client.get_related_articles(&[31978945]).await?;
    ///     println!("Found {} related articles", related.related_pmids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn get_related_articles(&self, pmids: &[u32]) -> Result<RelatedArticles> {
        if pmids.is_empty() {
            return Ok(RelatedArticles {
                source_pmids: Vec::new(),
                related_pmids: Vec::new(),
                link_type: "pubmed_pubmed".to_string(),
            });
        }

        let elink_response = self.elink_request(pmids, "pubmed", "pubmed_pubmed").await?;

        let mut all_related_pmids = Vec::new();

        for linkset in elink_response.linksets {
            if let Some(linkset_dbs) = linkset.linkset_dbs {
                for linkset_db in linkset_dbs {
                    if linkset_db.link_name == "pubmed_pubmed" {
                        for link_id in linkset_db.links {
                            if let Ok(pmid) = link_id.parse::<u32>() {
                                all_related_pmids.push(pmid);
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates and original PMIDs
        all_related_pmids.sort_unstable();
        all_related_pmids.dedup();
        all_related_pmids.retain(|&pmid| !pmids.contains(&pmid));

        info!(
            source_count = pmids.len(),
            related_count = all_related_pmids.len(),
            "Related articles retrieved successfully"
        );

        Ok(RelatedArticles {
            source_pmids: pmids.to_vec(),
            related_pmids: all_related_pmids,
            link_type: "pubmed_pubmed".to_string(),
        })
    }

    /// Get PMC links for given PMIDs (full-text availability)
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to check for PMC availability
    ///
    /// # Returns
    ///
    /// Returns a `Result<PmcLinks>` containing PMC IDs with full text available
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let pmc_links = client.get_pmc_links(&[31978945]).await?;
    ///     println!("Found {} PMC articles", pmc_links.pmc_ids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn get_pmc_links(&self, pmids: &[u32]) -> Result<PmcLinks> {
        if pmids.is_empty() {
            return Ok(PmcLinks {
                source_pmids: Vec::new(),
                pmc_ids: Vec::new(),
            });
        }

        let elink_response = self.elink_request(pmids, "pmc", "pubmed_pmc").await?;

        let mut pmc_ids = Vec::new();

        for linkset in elink_response.linksets {
            if let Some(linkset_dbs) = linkset.linkset_dbs {
                for linkset_db in linkset_dbs {
                    if linkset_db.link_name == "pubmed_pmc" && linkset_db.db_to == "pmc" {
                        pmc_ids.extend(linkset_db.links);
                    }
                }
            }
        }

        // Remove duplicates
        pmc_ids.sort();
        pmc_ids.dedup();

        info!(
            source_count = pmids.len(),
            pmc_count = pmc_ids.len(),
            "PMC links retrieved successfully"
        );

        Ok(PmcLinks {
            source_pmids: pmids.to_vec(),
            pmc_ids,
        })
    }

    /// Get citing articles for given PMIDs
    ///
    /// This method retrieves articles that cite the specified PMIDs from the PubMed database.
    /// The citation count returned represents only citations within the PubMed database
    /// (peer-reviewed journal articles indexed in PubMed).
    ///
    /// # Important Note on Citation Counts
    ///
    /// The citation count from this method may be **lower** than counts from other sources like
    /// Google Scholar, Web of Science, or scite.ai because:
    ///
    /// - **PubMed citations** (this method): Only includes peer-reviewed articles in PubMed
    /// - **Google Scholar/scite.ai**: Includes preprints, books, conference proceedings, and other sources
    ///
    /// For example, PMID 31978945 shows:
    /// - PubMed (this API): ~14,000 citations (PubMed database only)
    /// - scite.ai: ~23,000 citations (broader sources)
    ///
    /// This is expected behavior - this method provides accurate PubMed-specific citation data.
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to find citing articles for
    ///
    /// # Returns
    ///
    /// Returns a `Result<Citations>` containing citing article information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let citations = client.get_citations(&[31978945]).await?;
    ///     println!("Found {} citing articles in PubMed", citations.citing_pmids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn get_citations(&self, pmids: &[u32]) -> Result<Citations> {
        if pmids.is_empty() {
            return Ok(Citations {
                source_pmids: Vec::new(),
                citing_pmids: Vec::new(),
                link_type: "pubmed_pubmed_citedin".to_string(),
            });
        }

        let elink_response = self
            .elink_request(pmids, "pubmed", "pubmed_pubmed_citedin")
            .await?;

        let mut citing_pmids = Vec::new();

        for linkset in elink_response.linksets {
            if let Some(linkset_dbs) = linkset.linkset_dbs {
                for linkset_db in linkset_dbs {
                    if linkset_db.link_name == "pubmed_pubmed_citedin" {
                        for link_id in linkset_db.links {
                            if let Ok(pmid) = link_id.parse::<u32>() {
                                citing_pmids.push(pmid);
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates
        citing_pmids.sort_unstable();
        citing_pmids.dedup();

        info!(
            source_count = pmids.len(),
            citing_count = citing_pmids.len(),
            "Citations retrieved successfully"
        );

        Ok(Citations {
            source_pmids: pmids.to_vec(),
            citing_pmids,
            link_type: "pubmed_pubmed_citedin".to_string(),
        })
    }

    /// Internal helper method for making HTTP requests with retry logic
    /// Automatically appends API parameters (api_key, email, tool) to the URL
    async fn make_request(&self, url: &str) -> Result<Response> {
        // Build final URL with API parameters
        let mut final_url = url.to_string();
        let api_params = self.config.build_api_params();

        if !api_params.is_empty() {
            // Check if URL already has query parameters
            let separator = if url.contains('?') { '&' } else { '?' };
            final_url.push(separator);

            // Append API parameters
            let param_strings: Vec<String> = api_params
                .into_iter()
                .map(|(key, value)| format!("{}={}", key, urlencoding::encode(&value)))
                .collect();
            final_url.push_str(&param_strings.join("&"));
        }

        let response = with_retry(
            || async {
                self.rate_limiter.acquire().await?;
                debug!("Making API request to: {}", final_url);
                let response = self
                    .client
                    .get(&final_url)
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
        .await?;

        // Check for any non-success status (client errors, etc.)
        if !response.status().is_success() {
            warn!("API request failed with status: {}", response.status());
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
    }

    /// Internal helper method for ELink API requests
    async fn elink_request(
        &self,
        pmids: &[u32],
        target_db: &str,
        link_name: &str,
    ) -> Result<ELinkResponse> {
        // Convert PMIDs to strings and join with commas
        let id_list: Vec<String> = pmids.iter().map(|id| id.to_string()).collect();
        let ids = id_list.join(",");

        // Build URL - API parameters will be added by make_request
        let url = format!(
            "{}/elink.fcgi?dbfrom=pubmed&db={}&id={}&linkname={}&retmode=json",
            self.base_url,
            urlencoding::encode(target_db),
            urlencoding::encode(&ids),
            urlencoding::encode(link_name)
        );

        debug!("Making ELink API request");
        let response = self.make_request(&url).await?;

        let elink_response: ELinkResponse = response.json().await?;
        Ok(elink_response)
    }
}

impl Default for PubMedClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        mem,
        time::{Duration, Instant},
    };

    use super::*;

    #[test]
    fn test_client_config_rate_limiting() {
        // Test default configuration (no API key)
        let config = ClientConfig::new();
        assert_eq!(config.effective_rate_limit(), 3.0);

        // Test with API key
        let config_with_key = ClientConfig::new().with_api_key("test_key");
        assert_eq!(config_with_key.effective_rate_limit(), 10.0);

        // Test custom rate limit
        let config_custom = ClientConfig::new().with_rate_limit(5.0);
        assert_eq!(config_custom.effective_rate_limit(), 5.0);

        // Test custom rate limit overrides API key default
        let config_override = ClientConfig::new()
            .with_api_key("test_key")
            .with_rate_limit(7.0);
        assert_eq!(config_override.effective_rate_limit(), 7.0);
    }

    #[test]
    fn test_client_api_params() {
        let config = ClientConfig::new()
            .with_api_key("test_key_123")
            .with_email("test@example.com")
            .with_tool("TestTool");

        let params = config.build_api_params();

        // Should have 3 parameters
        assert_eq!(params.len(), 3);

        // Check each parameter
        assert!(params.contains(&("api_key".to_string(), "test_key_123".to_string())));
        assert!(params.contains(&("email".to_string(), "test@example.com".to_string())));
        assert!(params.contains(&("tool".to_string(), "TestTool".to_string())));
    }

    #[test]
    fn test_config_effective_values() {
        let config = ClientConfig::new()
            .with_email("test@example.com")
            .with_tool("TestApp");

        assert_eq!(
            config.effective_base_url(),
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils"
        );
        assert!(config
            .effective_user_agent()
            .starts_with("pubmed-client-rs/"));
        assert_eq!(config.effective_tool(), "TestApp");
    }

    #[test]
    fn test_rate_limiter_creation_from_config() {
        let config = ClientConfig::new()
            .with_api_key("test_key")
            .with_rate_limit(8.0);

        let rate_limiter = config.create_rate_limiter();

        // Rate limiter should be created successfully
        // We can't easily test the exact rate without async context,
        // but we can verify it was created
        assert!(mem::size_of_val(&rate_limiter) > 0);
    }

    #[tokio::test]
    async fn test_invalid_pmid_rate_limiting() {
        let config = ClientConfig::new().with_rate_limit(5.0);
        let client = PubMedClient::with_config(config);

        // Invalid PMID should fail before rate limiting (validation happens first)
        let start = Instant::now();
        let result = client.fetch_article("invalid_pmid").await;
        assert!(result.is_err());

        let elapsed = start.elapsed();
        // Should fail quickly without consuming rate limit token
        assert!(elapsed < Duration::from_millis(100));
    }

    #[test]
    fn test_empty_database_name_validation() {
        use tokio_test;

        let config = ClientConfig::new();
        let client = PubMedClient::with_config(config);

        let result = tokio_test::block_on(client.get_database_info(""));
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }

    #[test]
    fn test_whitespace_database_name_validation() {
        use tokio_test;

        let config = ClientConfig::new();
        let client = PubMedClient::with_config(config);

        let result = tokio_test::block_on(client.get_database_info("   "));
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_fetch_articles_empty_input() {
        let client = PubMedClient::new();

        let result = client.fetch_articles(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_fetch_articles_invalid_pmid() {
        let client = PubMedClient::new();

        let result = client.fetch_articles(&["not_a_number"]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_articles_validates_all_pmids_before_request() {
        let client = PubMedClient::new();

        // Mix of valid and invalid - should fail on validation before any network request
        let start = Instant::now();
        let result = client
            .fetch_articles(&["31978945", "invalid", "33515491"])
            .await;
        assert!(result.is_err());

        // Should fail quickly (validation only, no network)
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(100));
    }
}
