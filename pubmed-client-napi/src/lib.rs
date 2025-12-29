#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use pubmed_client::{
    pmc::{markdown::PmcMarkdownConverter, PmcFullText},
    pubmed::PubMedArticle,
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
}
