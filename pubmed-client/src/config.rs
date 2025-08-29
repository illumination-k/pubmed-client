use crate::cache::CacheConfig;
use crate::rate_limit::RateLimiter;
use crate::retry::RetryConfig;
use crate::time::Duration;

/// Configuration options for PubMed and PMC clients
///
/// This configuration allows customization of rate limiting, API keys,
/// timeouts, and other client behavior to comply with NCBI guidelines
/// and optimize performance.
#[derive(Clone)]
pub struct ClientConfig {
    /// NCBI E-utilities API key for increased rate limits
    ///
    /// With an API key:
    /// - Rate limit increases from 3 to 10 requests per second
    /// - Better stability and reduced chance of blocking
    /// - Required for high-volume applications
    ///
    /// Get your API key at: <https://ncbiinsights.ncbi.nlm.nih.gov/2017/11/02/new-api-keys-for-the-e-utilities/>
    pub api_key: Option<String>,

    /// Rate limit in requests per second
    ///
    /// Default values:
    /// - 3.0 without API key (NCBI guideline)
    /// - 10.0 with API key (NCBI guideline)
    ///
    /// Setting this value overrides the automatic selection based on API key presence.
    pub rate_limit: Option<f64>,

    /// HTTP request timeout
    ///
    /// Default: 30 seconds
    pub timeout: Duration,

    /// Custom User-Agent string for HTTP requests
    ///
    /// Default: "pubmed-client-rs/{version}"
    pub user_agent: Option<String>,

    /// Base URL for NCBI E-utilities
    ///
    /// Default: <https://eutils.ncbi.nlm.nih.gov/entrez/eutils>
    /// This should rarely need to be changed unless using a proxy or test environment.
    pub base_url: Option<String>,

    /// Email address for identification (recommended by NCBI)
    ///
    /// NCBI recommends including an email address in requests for contact
    /// in case of problems. This is automatically added to requests.
    pub email: Option<String>,

    /// Tool name for identification (recommended by NCBI)
    ///
    /// NCBI recommends including a tool name in requests.
    /// Default: "pubmed-client-rs"
    pub tool: Option<String>,

    /// Retry configuration for handling transient failures
    ///
    /// Default: 3 retries with exponential backoff starting at 1 second
    pub retry_config: RetryConfig,

    /// Cache configuration for response caching
    ///
    /// Default: Memory-only cache with 1000 items max
    pub cache_config: Option<CacheConfig>,
}

impl ClientConfig {
    /// Create a new configuration with default settings
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new();
    /// ```
    pub fn new() -> Self {
        Self {
            api_key: None,
            rate_limit: None,
            timeout: Duration::from_secs(30),
            user_agent: None,
            base_url: None,
            email: None,
            tool: None,
            retry_config: RetryConfig::default(),
            cache_config: None,
        }
    }

    /// Set the NCBI API key
    ///
    /// # Arguments
    ///
    /// * `api_key` - Your NCBI E-utilities API key
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .with_api_key("your_api_key_here");
    /// ```
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set a custom rate limit
    ///
    /// # Arguments
    ///
    /// * `rate` - Requests per second (must be positive)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// // Custom rate limit of 5 requests per second
    /// let config = ClientConfig::new()
    ///     .with_rate_limit(5.0);
    /// ```
    pub fn with_rate_limit(mut self, rate: f64) -> Self {
        if rate > 0.0 {
            self.rate_limit = Some(rate);
        }
        self
    }

    /// Set the HTTP request timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for HTTP responses
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    /// use pubmed_client_rs::time::Duration;
    ///
    /// let config = ClientConfig::new()
    ///     .with_timeout(Duration::from_secs(60));
    /// ```
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the HTTP request timeout in seconds (convenience method)
    ///
    /// # Arguments
    ///
    /// * `timeout_seconds` - Maximum time to wait for HTTP responses in seconds
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .with_timeout_seconds(60);
    /// ```
    pub fn with_timeout_seconds(mut self, timeout_seconds: u64) -> Self {
        self.timeout = Duration::from_secs(timeout_seconds);
        self
    }

    /// Set a custom User-Agent string
    ///
    /// # Arguments
    ///
    /// * `user_agent` - Custom User-Agent for HTTP requests
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .with_user_agent("MyApp/1.0");
    /// ```
    pub fn with_user_agent<S: Into<String>>(mut self, user_agent: S) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Set a custom base URL for NCBI E-utilities
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL for E-utilities API
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .with_base_url("https://proxy.example.com/eutils");
    /// ```
    pub fn with_base_url<S: Into<String>>(mut self, base_url: S) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Set email address for NCBI identification
    ///
    /// # Arguments
    ///
    /// * `email` - Your email address for NCBI contact
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .with_email("researcher@university.edu");
    /// ```
    pub fn with_email<S: Into<String>>(mut self, email: S) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set tool name for NCBI identification
    ///
    /// # Arguments
    ///
    /// * `tool` - Your application/tool name
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .with_tool("BioinformaticsApp");
    /// ```
    pub fn with_tool<S: Into<String>>(mut self, tool: S) -> Self {
        self.tool = Some(tool.into());
        self
    }

    /// Set retry configuration for handling transient failures
    ///
    /// # Arguments
    ///
    /// * `retry_config` - Custom retry configuration
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    /// use pubmed_client_rs::retry::RetryConfig;
    /// use pubmed_client_rs::time::Duration;
    ///
    /// let retry_config = RetryConfig::new()
    ///     .with_max_retries(5)
    ///     .with_initial_delay(Duration::from_secs(2));
    ///
    /// let config = ClientConfig::new()
    ///     .with_retry_config(retry_config);
    /// ```
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Enable caching with default configuration
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .with_cache();
    /// ```
    pub fn with_cache(mut self) -> Self {
        self.cache_config = Some(CacheConfig::default());
        self
    }

    /// Set cache configuration
    ///
    /// # Arguments
    ///
    /// * `cache_config` - Custom cache configuration
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    /// use pubmed_client_rs::cache::CacheConfig;
    ///
    /// let cache_config = CacheConfig {
    ///     max_capacity: 5000,
    ///     ..Default::default()
    /// };
    ///
    /// let config = ClientConfig::new()
    ///     .with_cache_config(cache_config);
    /// ```
    pub fn with_cache_config(mut self, cache_config: CacheConfig) -> Self {
        self.cache_config = Some(cache_config);
        self
    }

    /// Disable all caching
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new()
    ///     .without_cache();
    /// ```
    pub fn without_cache(mut self) -> Self {
        self.cache_config = None;
        self
    }

    /// Get the effective rate limit based on configuration
    ///
    /// Returns the configured rate limit, or the appropriate default
    /// based on whether an API key is present.
    ///
    /// # Returns
    ///
    /// - Custom rate limit if set
    /// - 10.0 requests/second if API key is present
    /// - 3.0 requests/second if no API key
    pub fn effective_rate_limit(&self) -> f64 {
        self.rate_limit.unwrap_or_else(|| {
            if self.api_key.is_some() {
                10.0 // NCBI rate limit with API key
            } else {
                3.0 // NCBI rate limit without API key
            }
        })
    }

    /// Create a rate limiter based on this configuration
    ///
    /// # Returns
    ///
    /// A `RateLimiter` configured with the appropriate rate limit
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::config::ClientConfig;
    ///
    /// let config = ClientConfig::new().with_api_key("your_key");
    /// let rate_limiter = config.create_rate_limiter();
    /// ```
    pub fn create_rate_limiter(&self) -> RateLimiter {
        RateLimiter::new(self.effective_rate_limit())
    }

    /// Get the base URL for E-utilities
    ///
    /// Returns the configured base URL or the default NCBI E-utilities URL.
    pub fn effective_base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or("https://eutils.ncbi.nlm.nih.gov/entrez/eutils")
    }

    /// Get the User-Agent string
    ///
    /// Returns the configured User-Agent or a default based on the crate name and version.
    pub fn effective_user_agent(&self) -> String {
        self.user_agent.clone().unwrap_or_else(|| {
            let version = env!("CARGO_PKG_VERSION");
            format!("pubmed-client-rs/{version}")
        })
    }

    /// Get the tool name for NCBI identification
    ///
    /// Returns the configured tool name or the default.
    pub fn effective_tool(&self) -> &str {
        self.tool.as_deref().unwrap_or("pubmed-client-rs")
    }

    /// Build query parameters for NCBI API requests
    ///
    /// This includes API key, email, and tool parameters when configured.
    pub fn build_api_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        if let Some(ref api_key) = self.api_key {
            params.push(("api_key".to_string(), api_key.clone()));
        }

        if let Some(ref email) = self.email {
            params.push(("email".to_string(), email.clone()));
        }

        params.push(("tool".to_string(), self.effective_tool().to_string()));

        params
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::new();
        assert!(config.api_key.is_none());
        assert!(config.rate_limit.is_none());
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.effective_rate_limit(), 3.0);
    }

    #[test]
    fn test_config_with_api_key() {
        let config = ClientConfig::new().with_api_key("test_key");
        assert_eq!(config.api_key.as_ref().unwrap(), "test_key");
        assert_eq!(config.effective_rate_limit(), 10.0);
    }

    #[test]
    fn test_custom_rate_limit() {
        let config = ClientConfig::new().with_rate_limit(5.0);
        assert_eq!(config.effective_rate_limit(), 5.0);

        // Custom rate limit overrides API key default
        let config_with_key = ClientConfig::new()
            .with_api_key("test")
            .with_rate_limit(7.0);
        assert_eq!(config_with_key.effective_rate_limit(), 7.0);
    }

    #[test]
    fn test_invalid_rate_limit() {
        let config = ClientConfig::new().with_rate_limit(-1.0);
        assert!(config.rate_limit.is_none());
        assert_eq!(config.effective_rate_limit(), 3.0);
    }

    #[test]
    fn test_fluent_interface() {
        let config = ClientConfig::new()
            .with_api_key("test_key")
            .with_rate_limit(5.0)
            .with_timeout(Duration::from_secs(60))
            .with_email("test@example.com")
            .with_tool("TestApp");

        assert_eq!(config.api_key.as_ref().unwrap(), "test_key");
        assert_eq!(config.effective_rate_limit(), 5.0);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.email.as_ref().unwrap(), "test@example.com");
        assert_eq!(config.effective_tool(), "TestApp");
    }

    #[test]
    fn test_api_params() {
        let config = ClientConfig::new()
            .with_api_key("test_key")
            .with_email("test@example.com")
            .with_tool("TestApp");

        let params = config.build_api_params();
        assert_eq!(params.len(), 3);

        assert!(params.contains(&("api_key".to_string(), "test_key".to_string())));
        assert!(params.contains(&("email".to_string(), "test@example.com".to_string())));
        assert!(params.contains(&("tool".to_string(), "TestApp".to_string())));
    }

    #[test]
    fn test_effective_values() {
        let config = ClientConfig::new();

        assert_eq!(
            config.effective_base_url(),
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils"
        );
        assert!(config
            .effective_user_agent()
            .starts_with("pubmed-client-rs/"));
        assert_eq!(config.effective_tool(), "pubmed-client-rs");
    }

    #[test]
    fn test_rate_limiter_creation() {
        let config = ClientConfig::new().with_rate_limit(5.0);
        let rate_limiter = config.create_rate_limiter();
        // The rate limiter creation should succeed
        assert!(mem::size_of_val(&rate_limiter) > 0);
    }
}
