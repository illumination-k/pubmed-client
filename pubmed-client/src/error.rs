use std::result;

use crate::retry::RetryableError;
use thiserror::Error;

/// Error types for PubMed client operations
#[derive(Error, Debug)]
pub enum PubMedError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    /// JSON parsing failed
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    /// XML parsing failed
    #[error("XML parsing failed: {0}")]
    XmlError(String),

    /// XML parsing error with detailed message
    #[error("XML parsing error: {message}")]
    XmlParseError { message: String },

    /// Article not found
    #[error("Article not found: PMID {pmid}")]
    ArticleNotFound { pmid: String },

    /// PMC full text not available
    #[error("PMC full text not available for PMID {pmid}")]
    PmcNotAvailable { pmid: String },

    /// PMC full text not available for PMCID
    #[error("PMC full text not available for PMCID {pmcid}")]
    PmcNotAvailableById { pmcid: String },

    /// Invalid PMID format
    #[error("Invalid PMID format: {pmid}")]
    InvalidPmid { pmid: String },

    /// Invalid query structure or parameters
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// API rate limit exceeded
    #[error("API rate limit exceeded")]
    RateLimitExceeded,

    /// Generic API error with HTTP status code
    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    /// IO error for file operations
    #[error("IO error: {message}")]
    IoError { message: String },

    /// Search limit exceeded
    /// This error is returned when a search query requests more results than the maximum retrievable limit.
    #[error("Search limit exceeded: requested {requested}, maximum is {maximum}")]
    SearchLimitExceeded { requested: usize, maximum: usize },
}

pub type Result<T> = result::Result<T, PubMedError>;

impl RetryableError for PubMedError {
    fn is_retryable(&self) -> bool {
        match self {
            // Network errors are typically transient
            PubMedError::RequestError(err) => {
                // Check if it's a network-related error
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if err.is_timeout() || err.is_connect() {
                        return true;
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if err.is_timeout() {
                        return true;
                    }
                }

                // Check for server errors (5xx)
                if let Some(status) = err.status() {
                    return status.is_server_error() || status.as_u16() == 429;
                }

                // DNS and other network errors
                !err.is_builder() && !err.is_redirect() && !err.is_decode()
            }

            // Rate limiting should be retried after delay
            PubMedError::RateLimitExceeded => true,

            // API errors might be retryable if they indicate server issues
            PubMedError::ApiError { status, message } => {
                // Server errors (5xx) and rate limiting (429) are retryable
                (*status >= 500 && *status < 600) || *status == 429 || {
                    // Also check message for specific error conditions
                    let lower_msg = message.to_lowercase();
                    lower_msg.contains("temporarily unavailable")
                        || lower_msg.contains("timeout")
                        || lower_msg.contains("connection")
                }
            }

            // All other errors are not retryable
            PubMedError::JsonError(_)
            | PubMedError::XmlError(_)
            | PubMedError::XmlParseError { .. }
            | PubMedError::ArticleNotFound { .. }
            | PubMedError::PmcNotAvailable { .. }
            | PubMedError::PmcNotAvailableById { .. }
            | PubMedError::InvalidPmid { .. }
            | PubMedError::InvalidQuery(_)
            | PubMedError::IoError { .. }
            | PubMedError::SearchLimitExceeded { .. } => false,
        }
    }

    fn retry_reason(&self) -> &str {
        if self.is_retryable() {
            match self {
                PubMedError::RequestError(err) if err.is_timeout() => "Request timeout",
                #[cfg(not(target_arch = "wasm32"))]
                PubMedError::RequestError(err) if err.is_connect() => "Connection error",
                PubMedError::RequestError(_) => "Network error",
                PubMedError::RateLimitExceeded => "Rate limit exceeded",
                PubMedError::ApiError { status, .. } => match status {
                    429 => "Rate limit exceeded",
                    500..=599 => "Server error",
                    _ => "Temporary API error",
                },
                _ => "Transient error",
            }
        } else {
            match self {
                PubMedError::JsonError(_) => "Invalid JSON response",
                PubMedError::XmlError(_) | PubMedError::XmlParseError { .. } => {
                    "Invalid XML response"
                }
                PubMedError::ArticleNotFound { .. } => "Article does not exist",
                PubMedError::PmcNotAvailable { .. } | PubMedError::PmcNotAvailableById { .. } => {
                    "Content not available"
                }
                PubMedError::InvalidPmid { .. } => "Invalid input",
                PubMedError::InvalidQuery(_) => "Invalid query",
                PubMedError::IoError { .. } => "File system error",
                _ => "Non-transient error",
            }
        }
    }
}
