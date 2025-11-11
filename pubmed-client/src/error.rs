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

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for non-retryable errors

    #[test]
    fn test_json_error_not_retryable() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err = PubMedError::JsonError(json_err);

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid JSON response");
    }

    #[test]
    fn test_xml_error_not_retryable() {
        let err = PubMedError::XmlError("Invalid XML format".to_string());

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid XML response");
    }

    #[test]
    fn test_xml_parse_error_not_retryable() {
        let err = PubMedError::XmlParseError {
            message: "Failed to parse element".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid XML response");
    }

    #[test]
    fn test_article_not_found_not_retryable() {
        let err = PubMedError::ArticleNotFound {
            pmid: "12345".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Article does not exist");
        assert!(format!("{}", err).contains("12345"));
    }

    #[test]
    fn test_pmc_not_available_not_retryable() {
        let err = PubMedError::PmcNotAvailable {
            pmid: "67890".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Content not available");
        assert!(format!("{}", err).contains("67890"));
    }

    #[test]
    fn test_pmc_not_available_by_id_not_retryable() {
        let err = PubMedError::PmcNotAvailableById {
            pmcid: "PMC123456".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Content not available");
        assert!(format!("{}", err).contains("PMC123456"));
    }

    #[test]
    fn test_invalid_pmid_not_retryable() {
        let err = PubMedError::InvalidPmid {
            pmid: "invalid".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid input");
        assert!(format!("{}", err).contains("invalid"));
    }

    #[test]
    fn test_invalid_query_not_retryable() {
        let err = PubMedError::InvalidQuery("Empty query string".to_string());

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid query");
        assert!(format!("{}", err).contains("Empty query"));
    }

    #[test]
    fn test_io_error_not_retryable() {
        let err = PubMedError::IoError {
            message: "File not found".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "File system error");
        assert!(format!("{}", err).contains("File not found"));
    }

    #[test]
    fn test_search_limit_exceeded_not_retryable() {
        let err = PubMedError::SearchLimitExceeded {
            requested: 15000,
            maximum: 10000,
        };

        assert!(!err.is_retryable());
        assert!(format!("{}", err).contains("15000"));
        assert!(format!("{}", err).contains("10000"));
    }

    // Tests for retryable errors

    #[test]
    fn test_rate_limit_exceeded_is_retryable() {
        let err = PubMedError::RateLimitExceeded;

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Rate limit exceeded");
    }

    #[test]
    fn test_api_error_429_is_retryable() {
        let err = PubMedError::ApiError {
            status: 429,
            message: "Too Many Requests".to_string(),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Rate limit exceeded");
        assert!(format!("{}", err).contains("429"));
    }

    #[test]
    fn test_api_error_500_is_retryable() {
        let err = PubMedError::ApiError {
            status: 500,
            message: "Internal Server Error".to_string(),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Server error");
    }

    #[test]
    fn test_api_error_503_is_retryable() {
        let err = PubMedError::ApiError {
            status: 503,
            message: "Service Unavailable".to_string(),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Server error");
    }

    #[test]
    fn test_api_error_temporarily_unavailable_is_retryable() {
        let err = PubMedError::ApiError {
            status: 400,
            message: "Service temporarily unavailable".to_string(),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Temporary API error");
    }

    #[test]
    fn test_api_error_timeout_message_is_retryable() {
        let err = PubMedError::ApiError {
            status: 408,
            message: "Request timeout".to_string(),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Temporary API error");
    }

    #[test]
    fn test_api_error_connection_message_is_retryable() {
        let err = PubMedError::ApiError {
            status: 400,
            message: "Connection reset by peer".to_string(),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Temporary API error");
    }

    #[test]
    fn test_api_error_404_not_retryable() {
        let err = PubMedError::ApiError {
            status: 404,
            message: "Not Found".to_string(),
        };

        assert!(!err.is_retryable());
    }

    #[test]
    fn test_api_error_400_not_retryable() {
        let err = PubMedError::ApiError {
            status: 400,
            message: "Bad Request".to_string(),
        };

        assert!(!err.is_retryable());
    }

    // Tests for error display formatting

    #[test]
    fn test_error_display_messages() {
        let test_cases = vec![
            (
                PubMedError::XmlError("test".to_string()),
                "XML parsing failed: test",
            ),
            (
                PubMedError::XmlParseError {
                    message: "test error".to_string(),
                },
                "XML parsing error: test error",
            ),
            (
                PubMedError::InvalidQuery("bad query".to_string()),
                "Invalid query: bad query",
            ),
            (PubMedError::RateLimitExceeded, "API rate limit exceeded"),
        ];

        for (error, expected_message) in test_cases {
            assert_eq!(format!("{}", error), expected_message);
        }
    }

    #[test]
    fn test_error_display_with_fields() {
        let err = PubMedError::ArticleNotFound {
            pmid: "12345".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Article not found"));
        assert!(display.contains("12345"));

        let err = PubMedError::ApiError {
            status: 500,
            message: "Server Error".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("500"));
        assert!(display.contains("Server Error"));
    }

    #[test]
    fn test_result_type_alias() {
        // Test that Result<T> type alias works correctly
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_err() -> Result<String> {
            Err(PubMedError::RateLimitExceeded)
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }
}
