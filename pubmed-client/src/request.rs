//! Shared HTTP request execution for NCBI E-utilities endpoints.
//!
//! Every endpoint module used to hand-build its own URL with `format!`,
//! sprinkle `urlencoding::encode()` calls, manually append the
//! api_key / email / tool parameters, and then copy-paste the same
//! rate-limit → retry → error-mapping block. [`RequestExecutor`] centralizes
//! all of that so endpoints only declare their parameter set and response type.

use reqwest::{Client, Response, StatusCode, Url};
use tracing::{debug, warn};

use crate::config::ClientConfig;
use crate::error::{PubMedError, Result};
use crate::rate_limit::RateLimiter;
use crate::retry::with_retry;

/// Bundles the pieces every endpoint needs to issue a rate-limited, retrying
/// request against the NCBI E-utilities API.
///
/// It borrows from the owning client (`PubMedClient` / `PmcClient` /
/// `PmcTarClient`), so it is cheap to construct per call.
pub(crate) struct RequestExecutor<'a> {
    client: &'a Client,
    rate_limiter: &'a RateLimiter,
    config: &'a ClientConfig,
}

impl<'a> RequestExecutor<'a> {
    /// Create an executor borrowing the client's HTTP client, rate limiter, and config.
    pub(crate) fn new(
        client: &'a Client,
        rate_limiter: &'a RateLimiter,
        config: &'a ClientConfig,
    ) -> Self {
        Self {
            client,
            rate_limiter,
            config,
        }
    }

    /// Build an absolute URL from a base, an endpoint path, and query parameters.
    ///
    /// The endpoint-specific `params` are appended first, followed by the
    /// configured API parameters (api_key / email / tool). All keys and values
    /// are percent-encoded by [`Url::query_pairs_mut`], so callers never need to
    /// call `urlencoding::encode()` themselves.
    pub(crate) fn build_url(
        &self,
        base_url: &str,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<String> {
        let mut url = Url::parse(&format!("{}/{}", base_url.trim_end_matches('/'), endpoint))
            .map_err(|e| {
                crate::PubMedError::InvalidQuery(format!(
                    "failed to build request URL from base {base_url:?} and endpoint {endpoint:?}: {e}"
                ))
            })?;

        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in params {
                pairs.append_pair(key, value);
            }
            for (key, value) in self.config.build_api_params() {
                pairs.append_pair(&key, &value);
            }
        }

        Ok(url.to_string())
    }

    /// GET an endpoint, building the URL from `params` plus the API parameters.
    pub(crate) async fn get_endpoint(
        &self,
        base_url: &str,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<Response> {
        let url = self.build_url(base_url, endpoint, params)?;
        self.get(&url).await
    }

    /// GET a pre-built URL with rate limiting, retry, and status-aware errors.
    ///
    /// Use this for fully-formed URLs (e.g. external OA download links or the
    /// ECitMatch `bdata` payload that must not be re-encoded).
    pub(crate) async fn get(&self, url: &str) -> Result<Response> {
        debug!("Making GET request to: {url}");
        self.send(|| self.client.get(url)).await
    }

    /// POST form-encoded data to a pre-built URL.
    pub(crate) async fn post_form(&self, url: &str, form: &[(String, String)]) -> Result<Response> {
        debug!("Making POST request to: {url}");
        self.send(|| self.client.post(url).form(form)).await
    }

    /// Core request loop shared by GET and POST.
    ///
    /// Acquires a rate-limit token, sends a freshly-built request, retries on
    /// transient failures (5xx / 429) with exponential backoff, and converts any
    /// non-success status into an [`PubMedError::ApiError`] whose message
    /// includes the HTTP status code.
    async fn send<F>(&self, build: F) -> Result<Response>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let response = with_retry(
            || async {
                self.rate_limiter.acquire().await?;
                let response = build().send().await.map_err(PubMedError::from)?;

                // Server errors and rate limiting are retryable.
                let status = response.status();
                if status.is_server_error() || status.as_u16() == 429 {
                    return Err(api_error(status));
                }

                Ok(response)
            },
            &self.config.retry_config,
            "NCBI API request",
        )
        .await?;

        // Any remaining non-success status (e.g. 4xx) is a terminal error.
        let status = response.status();
        if !status.is_success() {
            warn!(status = status.as_u16(), "API request failed");
            return Err(api_error(status));
        }

        Ok(response)
    }
}

/// Build an [`PubMedError::ApiError`] that preserves the HTTP status code in the
/// message (e.g. `404 Not Found`) instead of discarding it.
fn api_error(status: StatusCode) -> PubMedError {
    PubMedError::ApiError {
        status: status.as_u16(),
        message: format!(
            "{} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown error")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn executor_config(config: &ClientConfig) -> (Client, RateLimiter) {
        (Client::new(), config.create_rate_limiter())
    }

    #[test]
    fn test_build_url_encodes_params_and_appends_api_params() {
        let config = ClientConfig::new()
            .with_api_key("KEY")
            .with_email("user@example.com")
            .with_tool("MyTool");
        let (client, rate_limiter) = executor_config(&config);
        let executor = RequestExecutor::new(&client, &rate_limiter, &config);

        let url = executor
            .build_url(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils",
                "esearch.fcgi",
                &[("db", "pubmed"), ("term", "covid 19")],
            )
            .unwrap();

        assert!(url.starts_with("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?"));
        assert!(url.contains("db=pubmed"));
        // Space is percent/plus encoded, never left raw.
        assert!(url.contains("term=covid+19") || url.contains("term=covid%20"));
        assert!(url.contains("api_key=KEY"));
        assert!(url.contains("email=user%40example.com"));
        assert!(url.contains("tool=MyTool"));
    }

    #[test]
    fn test_build_url_trims_trailing_slash_on_base() {
        let config = ClientConfig::new();
        let (client, rate_limiter) = executor_config(&config);
        let executor = RequestExecutor::new(&client, &rate_limiter, &config);

        let url = executor
            .build_url("http://example.com/", "einfo.fcgi", &[("retmode", "json")])
            .unwrap();
        assert!(url.starts_with("http://example.com/einfo.fcgi?"));
        // Tool is always present.
        assert!(url.contains("tool=pubmed-client"));
    }

    #[test]
    fn test_api_error_includes_status_code() {
        let err = api_error(StatusCode::NOT_FOUND);
        let PubMedError::ApiError { status, message } = err else {
            unreachable!("api_error always returns ApiError");
        };
        assert_eq!(status, 404);
        assert!(message.contains("404"));
        assert!(message.contains("Not Found"));
    }
}
