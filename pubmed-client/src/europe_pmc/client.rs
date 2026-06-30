//! Core [`EuropePmcClient`] definition and shared plumbing.

use std::time::Duration;

use reqwest::Client;
use tracing::info;

use crate::cache::{PmcCache, create_cache};
use crate::config::ClientConfig;
use crate::rate_limit::RateLimiter;
use crate::request::RequestExecutor;

/// Base URL for the Europe PMC RESTful Web Service.
///
/// Europe PMC is hosted by EBI on a different host and path scheme from the
/// NCBI E-utilities, so it deliberately does **not** reuse
/// [`ClientConfig::base_url`] (which is the NCBI eutils override). Use
/// [`EuropePmcClient::with_base_url`] to point at a proxy or mock server.
pub(crate) const EUROPE_PMC_BASE_URL: &str = "https://www.ebi.ac.uk/europepmc/webservices/rest";

/// Client for the Europe PMC REST API.
///
/// Provides cross-source search, JATS full-text retrieval, reference and
/// citation graphs, external database links, and supplementary file downloads.
/// No API key is required; transport-level configuration (timeout, user agent,
/// retry, rate limiting, caching) is shared with the rest of the workspace via
/// [`ClientConfig`].
///
/// # Example
///
/// ```no_run
/// use pubmed_client::europe_pmc::{EuropePmcClient, EuropePmcId};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = EuropePmcClient::new();
///     let results = client.search("malaria vaccine", 10).await?;
///     for r in &results {
///         println!("{}/{}: {}", r.source, r.id, r.title.as_deref().unwrap_or(""));
///     }
///
///     let article = client.fetch_full_text(&EuropePmcId::pmc("PMC3258128")?).await?;
///     println!("Title: {}", article.title().unwrap_or("Untitled"));
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct EuropePmcClient {
    pub(crate) client: Client,
    pub(crate) base_url: String,
    pub(crate) rate_limiter: RateLimiter,
    pub(crate) config: ClientConfig,
    /// Cache for parsed full-text articles only (keyed by `epmc-ft:<source>:<id>`).
    pub(crate) cache: Option<PmcCache>,
}

impl EuropePmcClient {
    /// Create a new Europe PMC client with default configuration.
    pub fn new() -> Self {
        Self::with_config(ClientConfig::new())
    }

    /// Create a new Europe PMC client with custom configuration.
    ///
    /// Transport settings (timeout, user agent, retry, rate limit, cache) are
    /// taken from `config`. The NCBI-specific `base_url` field is ignored; the
    /// Europe PMC base URL is used instead.
    pub fn with_config(config: ClientConfig) -> Self {
        let rate_limiter = config.create_rate_limiter();

        // reqwest's builder only fails if the TLS backend cannot be initialized,
        // which is an unrecoverable process-level error, so this infallible
        // constructor is allowed to `expect`.
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

        let cache = config.cache_config.as_ref().map(create_cache);

        Self {
            client,
            base_url: EUROPE_PMC_BASE_URL.to_string(),
            rate_limiter,
            cache,
            config,
        }
    }

    /// Create a new Europe PMC client with a custom HTTP client and default config.
    pub fn with_client(client: Client) -> Self {
        let config = ClientConfig::new();
        let rate_limiter = config.create_rate_limiter();

        Self {
            client,
            base_url: EUROPE_PMC_BASE_URL.to_string(),
            rate_limiter,
            cache: None,
            config,
        }
    }

    /// Override the base URL (e.g. to target a proxy or a wiremock test server).
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Clear the full-text cache, if one is configured.
    pub async fn clear_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.clear().await;
            info!("Cleared Europe PMC full-text cache");
        }
    }

    /// Return the number of cached full-text entries (best-effort).
    pub fn cache_entry_count(&self) -> u64 {
        self.cache.as_ref().map_or(0, |cache| cache.entry_count())
    }

    /// Flush pending cache operations (useful in tests).
    pub async fn sync_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.sync().await;
        }
    }

    /// Build a request executor borrowing this client's HTTP client, rate limiter, and config.
    pub(crate) fn executor(&self) -> RequestExecutor<'_> {
        RequestExecutor::new(&self.client, &self.rate_limiter, &self.config)
    }
}

impl Default for EuropePmcClient {
    fn default() -> Self {
        Self::new()
    }
}
