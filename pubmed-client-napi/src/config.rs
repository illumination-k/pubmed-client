use napi_derive::napi;

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
    /// Requests per second allowed by the rate limiter
    ///
    /// Defaults to the NCBI limits (3/s without an API key, 10/s with one).
    /// Must be a positive number.
    pub rate_limit: Option<f64>,
    /// Enable the in-memory response cache with its default settings
    pub cache: Option<bool>,
}
