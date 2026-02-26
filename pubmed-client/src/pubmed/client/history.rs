//! History server operations (EPost, fetch from history, streaming search)

use crate::common::PubMedId;
use crate::error::{PubMedError, Result};
use crate::pubmed::models::{EPostResult, HistorySession, PubMedArticle, SearchResult};
use crate::pubmed::parser::parse_articles_from_xml;
use crate::pubmed::query::SortOrder;
use crate::pubmed::responses::{EPostResponse, ESearchResult};
use crate::retry::with_retry;
use tracing::{debug, info, instrument, warn};

use super::PubMedClient;

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

impl PubMedClient {
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
    /// use pubmed_client::PubMedClient;
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
        self.search_with_history_and_options(query, limit, None)
            .await
    }

    /// Search for articles with history server support and sort options
    ///
    /// This method enables NCBI's history server feature, which stores search results
    /// on the server and returns WebEnv/query_key identifiers. These can be used
    /// with `fetch_from_history()` to efficiently paginate through large result sets.
    ///
    /// Also returns query translation showing how PubMed interpreted the query.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of PMIDs to return in the initial response
    /// * `sort` - Optional sort order for results
    ///
    /// # Returns
    ///
    /// Returns a `Result<SearchResult>` containing PMIDs, history session, and query translation
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::{PubMedClient, pubmed::SortOrder};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let result = client
    ///         .search_with_history_and_options("asthma", 100, Some(&SortOrder::PublicationDate))
    ///         .await?;
    ///
    ///     println!("Total results: {}", result.total_count);
    ///     if let Some(translation) = &result.query_translation {
    ///         println!("Query interpreted as: {}", translation);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self, sort), fields(query = %query, limit = limit))]
    pub async fn search_with_history_and_options(
        &self,
        query: &str,
        limit: usize,
        sort: Option<&SortOrder>,
    ) -> Result<SearchResult> {
        if query.trim().is_empty() {
            debug!("Empty query provided, returning empty results");
            return Ok(SearchResult {
                pmids: Vec::new(),
                total_count: 0,
                webenv: None,
                query_key: None,
                query_translation: None,
            });
        }

        // Use usehistory=y to enable history server
        let mut url = format!(
            "{}/esearch.fcgi?db=pubmed&term={}&retmax={}&retstart={}&retmode=json&usehistory=y",
            self.base_url,
            urlencoding::encode(query),
            limit,
            0
        );

        if let Some(sort_order) = sort {
            url.push_str(&format!("&sort={}", sort_order.as_api_param()));
        }

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
            query_translation = ?search_result.esearchresult.querytranslation,
            "Search with history completed"
        );

        Ok(SearchResult {
            pmids: search_result.esearchresult.idlist,
            total_count,
            webenv: search_result.esearchresult.webenv,
            query_key: search_result.esearchresult.query_key,
            query_translation: search_result.esearchresult.querytranslation,
        })
    }

    /// Upload a list of PMIDs to the NCBI History server using EPost
    ///
    /// This stores the UIDs on the server and returns WebEnv/query_key identifiers
    /// that can be used with `fetch_from_history()` to retrieve article metadata.
    ///
    /// This is useful when you have a pre-existing list of PMIDs (e.g., from a file,
    /// database, or external source) and want to use them with history server features
    /// like batch fetching.
    ///
    /// # Arguments
    ///
    /// * `pmids` - Slice of PubMed IDs as strings
    ///
    /// # Returns
    ///
    /// Returns a `Result<EPostResult>` containing WebEnv and query_key
    ///
    /// # Errors
    ///
    /// * `ParseError::InvalidPmid` - If any PMID is invalid
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::ApiError` - If the NCBI API returns an error
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///
    ///     // Upload PMIDs to the history server
    ///     let result = client.epost(&["31978945", "33515491", "25760099"]).await?;
    ///
    ///     println!("WebEnv: {}", result.webenv);
    ///     println!("Query Key: {}", result.query_key);
    ///
    ///     // Use the session to fetch articles
    ///     let session = result.history_session();
    ///     let articles = client.fetch_from_history(&session, 0, 100).await?;
    ///     println!("Fetched {} articles", articles.len());
    ///
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn epost(&self, pmids: &[&str]) -> Result<EPostResult> {
        self.epost_internal(pmids, None).await
    }

    /// Upload PMIDs to an existing History server session using EPost
    ///
    /// This appends UIDs to an existing WebEnv session, allowing you to combine
    /// multiple sets of IDs into a single session for subsequent operations.
    ///
    /// # Arguments
    ///
    /// * `pmids` - Slice of PubMed IDs as strings
    /// * `session` - Existing history session to append to
    ///
    /// # Returns
    ///
    /// Returns a `Result<EPostResult>` with the updated session information.
    /// The returned `webenv` will be the same as the input session, and a new
    /// `query_key` will be assigned for the uploaded IDs.
    ///
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn epost_to_session(
        &self,
        pmids: &[&str],
        session: &HistorySession,
    ) -> Result<EPostResult> {
        self.epost_internal(pmids, Some(session)).await
    }

    /// Internal implementation for EPost
    async fn epost_internal(
        &self,
        pmids: &[&str],
        session: Option<&HistorySession>,
    ) -> Result<EPostResult> {
        if pmids.is_empty() {
            return Err(PubMedError::InvalidQuery(
                "PMID list cannot be empty for EPost".to_string(),
            ));
        }

        // Validate all PMIDs upfront
        let validated: Vec<u32> = pmids
            .iter()
            .map(|pmid| {
                PubMedId::parse(pmid)
                    .map(|p| p.as_u32())
                    .map_err(PubMedError::from)
            })
            .collect::<Result<Vec<_>>>()?;

        let id_list: String = validated
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        // Build form data for POST request
        let mut params = vec![
            ("db".to_string(), "pubmed".to_string()),
            ("id".to_string(), id_list),
            ("retmode".to_string(), "json".to_string()),
        ];

        if let Some(session) = session {
            params.push(("WebEnv".to_string(), session.webenv.clone()));
        }

        // Append API parameters (api_key, email, tool)
        params.extend(self.config().build_api_params());

        let url = format!("{}/epost.fcgi", self.base_url);

        debug!(pmids_count = pmids.len(), "Making EPost API request");

        let response = with_retry(
            || async {
                self.rate_limiter().acquire().await?;
                debug!("Making POST request to: {}", url);
                let response = self
                    .http_client()
                    .post(&url)
                    .form(&params)
                    .send()
                    .await
                    .map_err(PubMedError::from)?;

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
            &self.config().retry_config,
            "NCBI EPost API request",
        )
        .await?;

        if !response.status().is_success() {
            warn!("EPost request failed with status: {}", response.status());
            return Err(PubMedError::ApiError {
                status: response.status().as_u16(),
                message: response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string(),
            });
        }

        let epost_response: EPostResponse = response.json().await?;

        // Check for API error
        if let Some(error_msg) = &epost_response.epostresult.error {
            return Err(PubMedError::ApiError {
                status: 200,
                message: format!("NCBI EPost API error: {}", error_msg),
            });
        }

        let webenv = epost_response
            .epostresult
            .webenv
            .ok_or_else(|| PubMedError::WebEnvNotAvailable)?;

        let query_key = epost_response
            .epostresult
            .query_key
            .ok_or_else(|| PubMedError::WebEnvNotAvailable)?;

        info!(
            pmids_count = pmids.len(),
            query_key = %query_key,
            "EPost completed successfully"
        );

        Ok(EPostResult { webenv, query_key })
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
    /// use pubmed_client::PubMedClient;
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

    /// Fetch all articles for a list of PMIDs using EPost and the History server
    ///
    /// This is the recommended method for fetching large numbers of articles by PMID.
    /// It uploads the PMID list to the History server via EPost (using HTTP POST to
    /// avoid URL length limits), then fetches articles in batches using pagination.
    ///
    /// For small lists (up to ~200 PMIDs), `fetch_articles()` works fine. Use this
    /// method when you have hundreds or thousands of PMIDs.
    ///
    /// # Arguments
    ///
    /// * `pmids` - Slice of PubMed IDs as strings
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<PubMedArticle>>` containing all fetched articles
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///
    ///     // Works efficiently even with thousands of PMIDs
    ///     let pmids: Vec<&str> = vec!["31978945", "33515491", "25760099"];
    ///     let articles = client.fetch_all_by_pmids(&pmids).await?;
    ///     println!("Fetched {} articles", articles.len());
    ///
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn fetch_all_by_pmids(&self, pmids: &[&str]) -> Result<Vec<PubMedArticle>> {
        if pmids.is_empty() {
            return Ok(Vec::new());
        }

        // Upload PMIDs to History server
        let epost_result = self.epost(pmids).await?;
        let session = epost_result.history_session();

        const BATCH_SIZE: usize = 200;
        let total = pmids.len();
        let mut all_articles = Vec::with_capacity(total);
        let mut offset = 0;

        while offset < total {
            let articles = self
                .fetch_from_history(&session, offset, BATCH_SIZE)
                .await?;

            if articles.is_empty() {
                break;
            }

            info!(
                offset = offset,
                fetched = articles.len(),
                total = total,
                "Fetched batch from history"
            );

            offset += articles.len();
            all_articles.extend(articles);
        }

        info!(
            total_fetched = all_articles.len(),
            requested = pmids.len(),
            "fetch_all_by_pmids completed"
        );

        Ok(all_articles)
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
    /// use pubmed_client::PubMedClient;
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
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[tokio::test]
    async fn test_epost_empty_input() {
        let client = PubMedClient::new();
        let result = client.epost(&[]).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_epost_invalid_pmid() {
        let client = PubMedClient::new();
        let result = client.epost(&["not_a_number"]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_epost_validates_all_pmids_before_request() {
        let client = PubMedClient::new();

        let start = Instant::now();
        let result = client.epost(&["31978945", "invalid", "33515491"]).await;
        assert!(result.is_err());
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_fetch_all_by_pmids_empty_input() {
        let client = PubMedClient::new();
        let result = client.fetch_all_by_pmids(&[]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_fetch_all_by_pmids_invalid_pmid() {
        let client = PubMedClient::new();
        let result = client.fetch_all_by_pmids(&["not_a_number"]).await;
        assert!(result.is_err());
    }
}
