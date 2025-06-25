use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::pubmed::models::{
    Citations, DatabaseInfo, FieldInfo, LinkInfo, PmcLinks, PubMedArticle, RelatedArticles,
};
use crate::pubmed::parser::PubMedXmlParser;
use crate::pubmed::responses::{EInfoResponse, ELinkResponse, ESearchResult};
use crate::rate_limit::RateLimiter;
use reqwest::Client;
use tracing::{debug, info, instrument, warn};

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

        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(config.effective_user_agent())
            .build()
            .expect("Failed to create HTTP client");

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
        // Validate PMID format
        if pmid.trim().is_empty() || !pmid.chars().all(|c| c.is_ascii_digit()) {
            warn!("Invalid PMID format provided");
            return Err(PubMedError::InvalidPmid {
                pmid: pmid.to_string(),
            });
        }

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await?;

        // Build URL with API parameters
        let mut url = format!(
            "{}/efetch.fcgi?db=pubmed&id={}&retmode=xml&rettype=abstract",
            self.base_url, pmid
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        if !api_params.is_empty() {
            for (key, value) in api_params {
                url.push('&');
                url.push_str(&key);
                url.push('=');
                url.push_str(&urlencoding::encode(&value));
            }
        }

        debug!("Making EFetch API request");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            warn!("API request failed with status: {}", response.status());
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

        debug!("Received successful API response, parsing XML");
        let xml_text = response.text().await?;

        let result = PubMedXmlParser::parse_article_from_xml(&xml_text, pmid);
        match &result {
            Ok(article) => {
                info!(
                    title = %article.title,
                    authors_count = article.authors.len(),
                    has_abstract = article.abstract_text.is_some(),
                    "Successfully parsed article"
                );
            }
            Err(e) => {
                warn!("Failed to parse article XML: {}", e);
            }
        }

        result
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
        if query.trim().is_empty() {
            debug!("Empty query provided, returning empty results");
            return Ok(Vec::new());
        }

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await?;

        // Build URL with API parameters
        let mut url = format!(
            "{}/esearch.fcgi?db=pubmed&term={}&retmax={}&retmode=json",
            self.base_url,
            urlencoding::encode(query),
            limit
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        debug!("Making ESearch API request");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            warn!(
                "Search API request failed with status: {}",
                response.status()
            );
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

        let search_result: ESearchResult = response.json().await?;
        let pmids = search_result.esearchresult.idlist;

        info!(results_found = pmids.len(), "Search completed successfully");

        Ok(pmids)
    }

    /// Search and fetch multiple articles with metadata
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

        let mut articles = Vec::new();
        for pmid in pmids {
            match self.fetch_article(&pmid).await {
                Ok(article) => articles.push(article),
                Err(PubMedError::ArticleNotFound { .. }) => {
                    // Skip articles that can't be found
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(articles)
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
        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await?;

        // Build URL with API parameters
        let mut url = format!("{}/einfo.fcgi?retmode=json", self.base_url);

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        debug!("Making EInfo API request for database list");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            warn!(
                "EInfo API request failed with status: {}",
                response.status()
            );
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

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
                message: "Database name cannot be empty".to_string(),
            });
        }

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await?;

        // Build URL with API parameters
        let mut url = format!(
            "{}/einfo.fcgi?db={}&retmode=json",
            self.base_url,
            urlencoding::encode(database)
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        debug!("Making EInfo API request for database details");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            warn!(
                "EInfo API request failed with status: {}",
                response.status()
            );
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

        let einfo_response: EInfoResponse = response.json().await?;

        let db_info_list =
            einfo_response
                .einfo_result
                .db_info
                .ok_or_else(|| PubMedError::ApiError {
                    message: format!(
                        "Database '{}' not found or no information available",
                        database
                    ),
                })?;

        let db_info = db_info_list
            .into_iter()
            .next()
            .ok_or_else(|| PubMedError::ApiError {
                message: format!("Database '{}' information not found", database),
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
    ///     println!("Found {} citing articles", citations.citing_pmids.len());
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

        // Acquire rate limit token before making request
        self.rate_limiter.acquire().await?;

        // Build URL with API parameters
        let mut url = format!(
            "{}/elink.fcgi?dbfrom=pubmed&db={}&id={}&linkname={}&retmode=json",
            self.base_url,
            urlencoding::encode(target_db),
            urlencoding::encode(&ids),
            urlencoding::encode(link_name)
        );

        // Add API parameters (API key, email, tool)
        let api_params = self.config.build_api_params();
        for (key, value) in api_params {
            url.push('&');
            url.push_str(&key);
            url.push('=');
            url.push_str(&urlencoding::encode(&value));
        }

        debug!("Making ELink API request");
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            warn!(
                "ELink API request failed with status: {}",
                response.status()
            );
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

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
    use super::*;
    use std::time::{Duration, Instant};

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
        assert!(
            config
                .effective_user_agent()
                .starts_with("pubmed-client-rs/")
        );
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
        assert!(std::mem::size_of_val(&rate_limiter) > 0);
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
}
