//! Client configuration for the MCP server.
//!
//! The server is usually launched by an MCP host (Claude Desktop, an IDE, a
//! container runtime) from a JSON config file, so every knob is reachable both
//! as a CLI flag and as an environment variable.
//!
//! Naming follows the rest of the workspace: parameters that NCBI itself cares
//! about (API key, contact e-mail, tool name, request rate) reuse the `NCBI_*`
//! variables already honoured by `pubmed-cli`, while knobs that only affect
//! this server's local behaviour use `PUBMED_MCP_*`.

use anyhow::{Result, bail};
use clap::Args;
use clap::builder::BoolishValueParser;
use pubmed_client::cache::CacheConfig;
use pubmed_client::config::ClientConfig;
use pubmed_client::retry::RetryConfig;
use pubmed_client::time::Duration;
use std::time::Duration as StdDuration;

/// Command-line/environment configuration for the underlying PubMed client.
///
/// [`Debug`] is implemented by hand so the API key is redacted: this struct is
/// the one place in the server that holds a secret, and a stray `{:?}` in a
/// log line would publish it to the host's stderr.
///
/// `about`/`long_about` are cleared explicitly: clap copies a flattened
/// struct's doc comment onto the *parent* command, which would replace
/// `pubmed-mcp`'s own "PubMed MCP Server" description in `--help` with the
/// paragraph above.
#[derive(Args, Clone)]
#[command(
    about = None,
    long_about = None,
    next_help_heading = "Client Configuration"
)]
pub struct ClientArgs {
    /// NCBI E-utilities API key (raises the rate limit from 3 to 10 req/s)
    #[arg(long, env = "NCBI_API_KEY")]
    pub api_key: Option<String>,

    /// Contact e-mail sent to NCBI (recommended by their usage guidelines)
    #[arg(long, env = "NCBI_EMAIL")]
    pub email: Option<String>,

    /// Tool name sent to NCBI (recommended by their usage guidelines)
    #[arg(long, env = "NCBI_TOOL", default_value = "pubmed-mcp")]
    pub tool: String,

    /// Requests per second (default: 3 without an API key, 10 with one)
    #[arg(long, env = "NCBI_RATE_LIMIT")]
    pub rate_limit: Option<f64>,

    /// HTTP request timeout in seconds
    #[arg(long, env = "PUBMED_MCP_TIMEOUT", default_value_t = 30)]
    pub timeout: u64,

    /// Maximum number of retries for transient failures
    #[arg(long, env = "PUBMED_MCP_MAX_RETRIES")]
    pub max_retries: Option<usize>,

    /// Base URL for NCBI E-utilities (for proxies or test environments)
    #[arg(long, env = "PUBMED_MCP_BASE_URL")]
    pub base_url: Option<String>,

    /// Enable the in-memory response cache
    ///
    /// Accepts an optional boolish value (`--cache`, `--cache=false`) so the
    /// `PUBMED_MCP_CACHE` variable can be set to `1`/`yes`/`on` as well as
    /// `true` — MCP host configs and container runtimes all spell it
    /// differently.
    #[arg(
        long,
        env = "PUBMED_MCP_CACHE",
        num_args = 0..=1,
        default_value_t = false,
        default_missing_value = "true",
        value_parser = BoolishValueParser::new(),
    )]
    pub cache: bool,

    /// Maximum number of cached responses (implies --cache)
    #[arg(long, env = "PUBMED_MCP_CACHE_CAPACITY")]
    pub cache_capacity: Option<u64>,

    /// Time-to-live for cached responses, in seconds (implies --cache)
    #[arg(long, env = "PUBMED_MCP_CACHE_TTL")]
    pub cache_ttl: Option<u64>,
}

impl std::fmt::Debug for ClientArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientArgs")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("email", &self.email)
            .field("tool", &self.tool)
            .field("rate_limit", &self.rate_limit)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("base_url", &self.base_url)
            .field("cache", &self.cache)
            .field("cache_capacity", &self.cache_capacity)
            .field("cache_ttl", &self.cache_ttl)
            .finish()
    }
}

impl ClientArgs {
    /// Whether response caching should be enabled.
    ///
    /// Setting a capacity or TTL implies `--cache`; a knob for a disabled
    /// cache is never what the caller meant.
    pub fn cache_enabled(&self) -> bool {
        self.cache || self.cache_capacity.is_some() || self.cache_ttl.is_some()
    }

    /// Build a [`ClientConfig`] from the parsed arguments.
    ///
    /// Returns an error rather than silently falling back to a default when a
    /// value is out of range — a server that quietly ignores `--rate-limit 0`
    /// is worse than one that refuses to start.
    pub fn build_config(&self) -> Result<ClientConfig> {
        if self.timeout == 0 {
            bail!("--timeout must be greater than 0");
        }

        let mut config = ClientConfig::new()
            .with_tool(self.tool.clone())
            .with_timeout(Duration::from_secs(self.timeout));

        if let Some(api_key) = &self.api_key {
            config = config.with_api_key(api_key.clone());
        }
        if let Some(email) = &self.email {
            config = config.with_email(email.clone());
        }
        if let Some(rate_limit) = self.rate_limit {
            if !(rate_limit.is_finite() && rate_limit > 0.0) {
                bail!("--rate-limit must be a positive number, got {rate_limit}");
            }
            config = config.with_rate_limit(rate_limit);
        }
        if let Some(max_retries) = self.max_retries {
            config = config.with_retry_config(RetryConfig::new().with_max_retries(max_retries));
        }
        if let Some(base_url) = &self.base_url {
            config = config.with_base_url(base_url.clone());
        }
        if self.cache_enabled() {
            let defaults = CacheConfig::default();
            config = config.with_cache_config(CacheConfig {
                max_capacity: self.cache_capacity.unwrap_or(defaults.max_capacity),
                time_to_live: self
                    .cache_ttl
                    .map(StdDuration::from_secs)
                    .unwrap_or(defaults.time_to_live),
                ..defaults
            });
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construct args directly rather than through `clap`: every field is
    /// env-backed, so a `parse_from` test would pick up whatever
    /// `NCBI_API_KEY`/`PUBMED_MCP_*` the developer or CI happens to export.
    fn args() -> ClientArgs {
        ClientArgs {
            api_key: None,
            email: None,
            tool: "pubmed-mcp".to_string(),
            rate_limit: None,
            timeout: 30,
            max_retries: None,
            base_url: None,
            cache: false,
            cache_capacity: None,
            cache_ttl: None,
        }
    }

    #[test]
    fn debug_output_never_contains_the_api_key() {
        let rendered = format!(
            "{:?}",
            ClientArgs {
                api_key: Some("super-secret".to_string()),
                ..args()
            }
        );

        assert!(!rendered.contains("super-secret"), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }

    #[test]
    fn defaults_match_the_client_defaults_plus_the_tool_name() {
        let config = args().build_config().unwrap();

        assert_eq!(config.tool.as_deref(), Some("pubmed-mcp"));
        assert_eq!(config.api_key, None);
        assert_eq!(config.email, None);
        assert_eq!(config.rate_limit, None);
        assert_eq!(config.base_url, None);
        assert_eq!(config.timeout.as_secs(), 30);
        assert_eq!(config.retry_config.max_retries, 3);
        assert!(config.cache_config.is_none());
    }

    #[test]
    fn ncbi_identification_and_limits_are_forwarded() {
        let config = ClientArgs {
            api_key: Some("secret".to_string()),
            email: Some("researcher@example.edu".to_string()),
            tool: "my-server".to_string(),
            rate_limit: Some(8.5),
            timeout: 120,
            max_retries: Some(5),
            base_url: Some("https://proxy.example.com/eutils".to_string()),
            ..args()
        }
        .build_config()
        .unwrap();

        assert_eq!(config.api_key.as_deref(), Some("secret"));
        assert_eq!(config.email.as_deref(), Some("researcher@example.edu"));
        assert_eq!(config.tool.as_deref(), Some("my-server"));
        assert_eq!(config.rate_limit, Some(8.5));
        assert_eq!(config.timeout.as_secs(), 120);
        assert_eq!(config.retry_config.max_retries, 5);
        assert_eq!(
            config.base_url.as_deref(),
            Some("https://proxy.example.com/eutils")
        );
    }

    #[test]
    fn cache_is_off_unless_asked_for() {
        assert!(!args().cache_enabled());
        assert!(args().build_config().unwrap().cache_config.is_none());
    }

    #[test]
    fn cache_flag_enables_the_defaults() {
        let defaults = CacheConfig::default();
        let cache = ClientArgs {
            cache: true,
            ..args()
        }
        .build_config()
        .unwrap()
        .cache_config
        .expect("cache should be enabled");

        assert_eq!(cache.max_capacity, defaults.max_capacity);
        assert_eq!(cache.time_to_live, defaults.time_to_live);
    }

    #[test]
    fn cache_capacity_and_ttl_imply_the_cache() {
        let cache = ClientArgs {
            cache_capacity: Some(50),
            cache_ttl: Some(600),
            ..args()
        }
        .build_config()
        .unwrap()
        .cache_config
        .expect("a capacity or TTL should turn the cache on");

        assert_eq!(cache.max_capacity, 50);
        assert_eq!(cache.time_to_live, StdDuration::from_secs(600));
    }

    #[test]
    fn only_the_given_cache_knob_overrides_its_default() {
        let defaults = CacheConfig::default();
        let cache = ClientArgs {
            cache_ttl: Some(600),
            ..args()
        }
        .build_config()
        .unwrap()
        .cache_config
        .unwrap();

        assert_eq!(cache.max_capacity, defaults.max_capacity);
        assert_eq!(cache.time_to_live, StdDuration::from_secs(600));
    }

    #[test]
    fn out_of_range_values_are_rejected_rather_than_ignored() {
        for rate_limit in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let err = ClientArgs {
                rate_limit: Some(rate_limit),
                ..args()
            }
            .build_config()
            .err()
            .expect("non-positive rate limits must not fall back to the default");
            assert!(err.to_string().contains("--rate-limit"), "{err}");
        }

        let err = ClientArgs {
            timeout: 0,
            ..args()
        }
        .build_config()
        .err()
        .expect("a zero timeout must not be accepted");
        assert!(err.to_string().contains("--timeout"), "{err}");
    }
}
