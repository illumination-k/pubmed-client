use crate::error::{PubMedError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

/// Represents a PubMed article with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PubMedArticle {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// List of authors
    pub authors: Vec<String>,
    /// Journal name
    pub journal: String,
    /// Publication date
    pub pub_date: String,
    /// DOI (Digital Object Identifier)
    pub doi: Option<String>,
    /// Abstract text (if available)
    pub abstract_text: Option<String>,
    /// Article types (e.g., "Clinical Trial", "Review", etc.)
    pub article_types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ESearchResult {
    esearchresult: ESearchData,
}

#[derive(Debug, Serialize, Deserialize)]
struct ESearchData {
    idlist: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ESummaryResult {
    result: ESummaryResultData,
}

#[derive(Debug, Serialize, Deserialize)]
struct ESummaryResultData {
    uids: Vec<String>,
    #[serde(flatten)]
    articles: std::collections::HashMap<String, ESummaryData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ESummaryData {
    title: String,
    authors: Vec<AuthorData>,
    fulljournalname: String,
    pubdate: String,
    elocationid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthorData {
    name: String,
}

/// Client for interacting with PubMed API
#[derive(Clone)]
pub struct PubMedClient {
    client: Client,
    base_url: String,
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
    pub fn search(&self) -> crate::query::SearchQuery {
        crate::query::SearchQuery::new()
    }
    /// Create a new PubMed client
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// let client = PubMedClient::new();
    /// ```
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://eutils.ncbi.nlm.nih.gov/entrez/eutils".to_string(),
        }
    }

    /// Create a new PubMed client with custom HTTP client
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
        Self {
            client,
            base_url: "https://eutils.ncbi.nlm.nih.gov/entrez/eutils".to_string(),
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

        // Use EFetch to get full article details including abstract
        let fetch_url = format!(
            "{}/efetch.fcgi?db=pubmed&id={}&retmode=xml&rettype=abstract",
            self.base_url, pmid
        );

        debug!("Making EFetch API request");
        let response = self.client.get(&fetch_url).send().await?;

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

        let result = self.parse_article_from_xml(&xml_text, pmid);
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

    /// Parse article from EFetch XML response
    #[instrument(skip(self, xml), fields(pmid = %pmid, xml_size = xml.len()))]
    pub fn parse_article_from_xml(&self, xml: &str, pmid: &str) -> Result<PubMedArticle> {
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use std::io::BufReader;

        let mut reader = Reader::from_reader(BufReader::new(xml.as_bytes()));
        reader.config_mut().trim_text(true);

        let mut title = String::new();
        let mut authors = Vec::new();
        let mut journal = String::new();
        let mut pub_date = String::new();
        let doi = None;
        let mut abstract_text = None;
        let mut article_types = Vec::new();

        let mut buf = Vec::new();
        let mut in_article_title = false;
        let mut in_abstract = false;
        let mut in_abstract_text = false;
        let mut in_journal_title = false;
        let mut in_pub_date = false;
        let mut in_author_list = false;
        let mut in_author = false;
        let mut in_last_name = false;
        let mut in_fore_name = false;
        let mut in_publication_type = false;
        let mut current_author_last = String::new();
        let mut current_author_fore = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"ArticleTitle" => in_article_title = true,
                        b"Abstract" => in_abstract = true,
                        b"AbstractText" => in_abstract_text = true,
                        b"Title" if !in_article_title => in_journal_title = true,
                        b"PubDate" => in_pub_date = true,
                        b"AuthorList" => in_author_list = true,
                        b"Author" if in_author_list => {
                            in_author = true;
                            current_author_last.clear();
                            current_author_fore.clear();
                        }
                        b"LastName" if in_author => in_last_name = true,
                        b"ForeName" if in_author => in_fore_name = true,
                        b"PublicationType" => in_publication_type = true,
                        b"ELocationID" => {
                            // Check if this is a DOI
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"EIdType" && attr.value.as_ref() == b"doi"
                                {
                                    // We'll capture the DOI text in the next text event
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => match e.name().as_ref() {
                    b"ArticleTitle" => in_article_title = false,
                    b"Abstract" => in_abstract = false,
                    b"AbstractText" => in_abstract_text = false,
                    b"Title" => in_journal_title = false,
                    b"PubDate" => in_pub_date = false,
                    b"AuthorList" => in_author_list = false,
                    b"Author" => {
                        if in_author {
                            let full_name = if !current_author_fore.is_empty() {
                                format!("{} {}", current_author_fore, current_author_last)
                            } else {
                                current_author_last.clone()
                            };
                            if !full_name.trim().is_empty() {
                                authors.push(full_name);
                            }
                            in_author = false;
                        }
                    }
                    b"LastName" => in_last_name = false,
                    b"ForeName" => in_fore_name = false,
                    b"PublicationType" => in_publication_type = false,
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    let text = e
                        .unescape()
                        .map_err(|_| PubMedError::XmlParseError {
                            message: "Failed to decode XML text".to_string(),
                        })?
                        .into_owned();

                    if in_article_title {
                        title = text;
                    } else if in_abstract_text && in_abstract {
                        abstract_text = Some(text);
                    } else if in_journal_title && !in_article_title {
                        journal = text;
                    } else if in_pub_date {
                        if pub_date.is_empty() {
                            pub_date = text;
                        } else {
                            pub_date.push(' ');
                            pub_date.push_str(&text);
                        }
                    } else if in_last_name && in_author {
                        current_author_last = text;
                    } else if in_fore_name && in_author {
                        current_author_fore = text;
                    } else if in_publication_type {
                        article_types.push(text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(PubMedError::XmlParseError {
                        message: format!("XML parsing error: {}", e),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        // If no article found, return error
        if title.is_empty() {
            debug!("No article title found in XML, article not found");
            return Err(PubMedError::ArticleNotFound {
                pmid: pmid.to_string(),
            });
        }

        debug!(
            authors_parsed = authors.len(),
            has_abstract = abstract_text.is_some(),
            journal = %journal,
            "Completed XML parsing"
        );

        Ok(PubMedArticle {
            pmid: pmid.to_string(),
            title,
            authors,
            journal,
            pub_date,
            doi,
            abstract_text,
            article_types,
        })
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

        let search_url = format!(
            "{}/esearch.fcgi?db=pubmed&term={}&retmax={}&retmode=json",
            self.base_url,
            urlencoding::encode(query),
            limit
        );

        debug!("Making ESearch API request");
        let response = self.client.get(&search_url).send().await?;

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
}

impl Default for PubMedClient {
    fn default() -> Self {
        Self::new()
    }
}
