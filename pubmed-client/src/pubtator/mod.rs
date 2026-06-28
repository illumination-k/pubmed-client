//! Client for NCBI's [PubTator3] biomedical text-mining API.
//!
//! PubTator3 annotates PubMed abstracts and PMC full-text articles with
//! normalized bio-entities (genes, diseases, chemicals, species, variants) and
//! the relations between them. [`PubTatorClient`] wraps the two most useful
//! endpoints:
//!
//! - **Publications export** — fetch BioC-JSON annotations for a set of PMIDs or
//!   PMCIDs ([`PubTatorClient::export_annotations`],
//!   [`PubTatorClient::export_full_text_annotations`],
//!   [`PubTatorClient::export_pmc_annotations`]).
//! - **Entity autocomplete** — resolve free text to normalized entity
//!   identifiers ([`PubTatorClient::find_entity`]).
//!
//! Requests share the same rate limiting, retry, and user-agent handling as the
//! PubMed/PMC clients via the crate's request executor.
//!
//! [PubTator3]: https://www.ncbi.nlm.nih.gov/research/pubtator3/

use std::time::Duration;

use reqwest::Client;
use tracing::{debug, info, instrument};

use crate::config::ClientConfig;
use crate::error::Result;
use crate::rate_limit::RateLimiter;
use crate::request::RequestExecutor;

use pubmed_parser::pubtator::{parse_biocjson, parse_entity_matches};

pub use pubmed_parser::pubtator::{
    BioCAnnotation, BioCDocument, BioCLocation, BioCNode, BioCPassage, BioCRelation, EntityMatch,
    EntityType, PubTatorResponse, RelationRole,
};

/// Default base URL for the PubTator3 REST API.
pub const DEFAULT_PUBTATOR_BASE_URL: &str = "https://www.ncbi.nlm.nih.gov/research/pubtator3-api";

/// Client for the PubTator3 text-mining API.
#[derive(Clone)]
pub struct PubTatorClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    config: ClientConfig,
}

impl PubTatorClient {
    /// Create a new PubTator client with default configuration.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::PubTatorClient;
    ///
    /// let client = PubTatorClient::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(ClientConfig::new())
    }

    /// Create a new PubTator client with custom configuration.
    ///
    /// Reuses the supplied configuration's rate limit, API key, email, and tool
    /// name. The PubTator3 API lives at a fixed base URL (see
    /// [`DEFAULT_PUBTATOR_BASE_URL`]) independent of the E-utilities base URL;
    /// override it with [`PubTatorClient::with_base_url`] if needed (e.g. for a
    /// test mock).
    pub fn with_config(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();

        // reqwest's client builder only fails if the TLS backend cannot be
        // initialized — an unrecoverable process-level environment error — so
        // this infallible public constructor is allowed to `expect` here.
        #[allow(clippy::expect_used)]
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
            base_url: DEFAULT_PUBTATOR_BASE_URL.to_string(),
            rate_limiter,
            config,
        }
    }

    /// Create a new PubTator client borrowing an existing HTTP client.
    pub fn with_client(client: Client) -> Self {
        let config = ClientConfig::new();
        let rate_limiter = config.create_rate_limiter();
        Self {
            client,
            base_url: DEFAULT_PUBTATOR_BASE_URL.to_string(),
            rate_limiter,
            config,
        }
    }

    /// Override the PubTator3 API base URL (primarily for testing).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn executor(&self) -> RequestExecutor<'_> {
        RequestExecutor::new(&self.client, &self.rate_limiter, &self.config)
    }

    /// Fetch abstract-level annotations for the given PMIDs.
    ///
    /// Returns one [`BioCDocument`] per PMID that PubTator3 recognizes, each
    /// covering the title and abstract passages. PMIDs that PubTator3 does not
    /// know are silently omitted from the response.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubTatorClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubTatorClient::new();
    ///     let response = client.export_annotations(&["29355051"]).await?;
    ///     for doc in &response.documents {
    ///         for ann in doc.annotations() {
    ///             println!("{} [{}]", ann.text, ann.entity_type().as_str());
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self, pmids), fields(count = pmids.len()))]
    pub async fn export_annotations(&self, pmids: &[&str]) -> Result<PubTatorResponse> {
        self.export_publications(pmids, false).await
    }

    /// Fetch full-text annotations for the given PMIDs.
    ///
    /// Behaves like [`PubTatorClient::export_annotations`] but requests the full
    /// body of each article (title, abstract, and all body sections). Full text
    /// is only available for articles in the PMC Open Access subset; for others
    /// PubTator3 falls back to abstract-level annotations.
    #[instrument(skip(self, pmids), fields(count = pmids.len()))]
    pub async fn export_full_text_annotations(&self, pmids: &[&str]) -> Result<PubTatorResponse> {
        self.export_publications(pmids, true).await
    }

    /// Fetch full-text annotations addressed by PMCID rather than PMID.
    ///
    /// Uses the PubTator3 `pmc_export` endpoint. PMCIDs may be supplied with or
    /// without the `PMC` prefix.
    #[instrument(skip(self, pmcids), fields(count = pmcids.len()))]
    pub async fn export_pmc_annotations(&self, pmcids: &[&str]) -> Result<PubTatorResponse> {
        if pmcids.is_empty() {
            return Ok(PubTatorResponse::default());
        }
        let joined = pmcids.join(",");
        debug!(pmcids = %joined, "Requesting PubTator3 PMC export");
        let response = self
            .executor()
            .get_endpoint(
                &self.base_url,
                "publications/pmc_export/biocjson",
                &[("pmcids", joined.as_str())],
            )
            .await?;
        let body = response.text().await?;
        let parsed = parse_biocjson(&body)?;
        info!(
            documents = parsed.documents.len(),
            "PubTator3 PMC export complete"
        );
        Ok(parsed)
    }

    /// Resolve free text to normalized PubTator3 entities (autocomplete).
    ///
    /// Returns ranked [`EntityMatch`] suggestions. Useful for turning a
    /// user-supplied name (e.g. `"covid-19"`) into a stable entity identifier
    /// (e.g. `@DISEASE_COVID_19`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubTatorClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubTatorClient::new();
    ///     let matches = client.find_entity("BRCA1").await?;
    ///     if let Some(top) = matches.first() {
    ///         println!("{} -> {}", top.name, top.id);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(query = %query))]
    pub async fn find_entity(&self, query: &str) -> Result<Vec<EntityMatch>> {
        self.find_entity_inner(query, None, None).await
    }

    /// Like [`PubTatorClient::find_entity`], but restricted to one entity
    /// concept (e.g. `"gene"`, `"disease"`, `"chemical"`) and capped at `limit`
    /// results.
    #[instrument(skip(self), fields(query = %query, concept = %concept))]
    pub async fn find_entity_with_concept(
        &self,
        query: &str,
        concept: &str,
        limit: usize,
    ) -> Result<Vec<EntityMatch>> {
        self.find_entity_inner(query, Some(concept), Some(limit))
            .await
    }

    async fn export_publications(&self, pmids: &[&str], full: bool) -> Result<PubTatorResponse> {
        if pmids.is_empty() {
            return Ok(PubTatorResponse::default());
        }
        let joined = pmids.join(",");
        let mut params: Vec<(&str, &str)> = vec![("pmids", joined.as_str())];
        if full {
            params.push(("full", "true"));
        }
        debug!(pmids = %joined, full, "Requesting PubTator3 publications export");
        let response = self
            .executor()
            .get_endpoint(&self.base_url, "publications/export/biocjson", &params)
            .await?;
        let body = response.text().await?;
        let parsed = parse_biocjson(&body)?;
        info!(
            documents = parsed.documents.len(),
            full, "PubTator3 export complete"
        );
        Ok(parsed)
    }

    async fn find_entity_inner(
        &self,
        query: &str,
        concept: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<EntityMatch>> {
        let limit_str;
        let mut params: Vec<(&str, &str)> = vec![("query", query)];
        if let Some(concept) = concept {
            params.push(("concept", concept));
        }
        if let Some(limit) = limit {
            limit_str = limit.to_string();
            params.push(("limit", limit_str.as_str()));
        }
        let response = self
            .executor()
            .get_endpoint(&self.base_url, "entity/autocomplete/", &params)
            .await?;
        let body = response.text().await?;
        let matches = parse_entity_matches(&body)?;
        info!(matches = matches.len(), "PubTator3 entity lookup complete");
        Ok(matches)
    }
}

impl Default for PubTatorClient {
    fn default() -> Self {
        Self::new()
    }
}
