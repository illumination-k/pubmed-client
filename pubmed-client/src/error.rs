use std::result;
use std::time::Duration;

use crate::retry::RetryableError;
use thiserror::Error;

/// Error types for PubMed client operations with enhanced contextual information
#[derive(Error, Debug)]
pub enum PubMedError {
    /// HTTP request failed
    #[error("HTTP request failed: {message}{}", format_suggestion(.suggestion))]
    RequestError {
        message: String,
        #[source]
        source: reqwest::Error,
        suggestion: Option<String>,
        retry_after: Option<Duration>,
    },

    /// JSON parsing failed
    #[error("JSON parsing failed: {message}{}", format_context(.context))]
    JsonError {
        message: String,
        #[source]
        source: serde_json::Error,
        context: Option<String>,
    },

    /// XML parsing failed
    #[error("XML parsing failed: {message}{}{}", format_context(.context), format_suggestion(.suggestion))]
    XmlError {
        message: String,
        context: Option<String>,
        suggestion: Option<String>,
    },

    /// XML parsing error with detailed message
    #[error("XML parsing error: {message}{}", format_suggestion(.suggestion))]
    XmlParseError {
        message: String,
        suggestion: Option<String>,
    },

    /// Article not found
    #[error("Article not found: PMID {pmid}. {suggestion}")]
    ArticleNotFound { pmid: String, suggestion: String },

    /// PMC full text not available
    #[error("PMC full text not available for PMID {pmid}. {suggestion}")]
    PmcNotAvailable { pmid: String, suggestion: String },

    /// PMC full text not available for PMCID
    #[error("PMC full text not available for PMCID {pmcid}. {suggestion}")]
    PmcNotAvailableById { pmcid: String, suggestion: String },

    /// Invalid PMID format
    #[error("Invalid PMID format: '{pmid}'. {suggestion}")]
    InvalidPmid { pmid: String, suggestion: String },

    /// Invalid query structure or parameters
    #[error("Invalid query: {message}{}", format_suggestion(.suggestion))]
    InvalidQuery {
        message: String,
        suggestion: Option<String>,
    },

    /// API rate limit exceeded
    #[error("API rate limit exceeded{}", format_retry_after(.retry_after))]
    RateLimitExceeded {
        retry_after: Option<Duration>,
        suggestion: String,
    },

    /// Generic API error with HTTP status code
    #[error("API error {status}: {message}{}{}{}", format_context(.context), format_suggestion(.suggestion), format_retry_after(.retry_after))]
    ApiError {
        status: u16,
        message: String,
        context: Option<String>,
        suggestion: Option<String>,
        retry_after: Option<Duration>,
    },

    /// IO error for file operations
    #[error("IO error: {message}{}", format_suggestion(.suggestion))]
    IoError {
        message: String,
        suggestion: Option<String>,
    },

    /// Search limit exceeded
    /// This error is returned when a search query requests more results than the maximum retrievable limit.
    #[error("Search limit exceeded: requested {requested}, maximum is {maximum}. {suggestion}")]
    SearchLimitExceeded {
        requested: usize,
        maximum: usize,
        suggestion: String,
    },
}

/// Helper function to format optional context in error messages
fn format_context(context: &Option<String>) -> String {
    context
        .as_ref()
        .map(|c| format!(" (context: {})", c))
        .unwrap_or_default()
}

/// Helper function to format optional suggestion in error messages
fn format_suggestion(suggestion: &Option<String>) -> String {
    suggestion
        .as_ref()
        .map(|s| format!(" Suggestion: {}", s))
        .unwrap_or_default()
}

/// Helper function to format optional retry_after duration in error messages
fn format_retry_after(retry_after: &Option<Duration>) -> String {
    retry_after
        .as_ref()
        .map(|d| format!(". Retry after {} seconds", d.as_secs()))
        .unwrap_or_default()
}

/// Convert reqwest::Error to PubMedError with context
impl From<reqwest::Error> for PubMedError {
    fn from(err: reqwest::Error) -> Self {
        let message = err.to_string();
        let suggestion = if err.is_timeout() {
            Some("The request timed out. Try again or check your network connection.".to_string())
        } else if err.is_connect() {
            Some("Failed to connect to the server. Check your network connection.".to_string())
        } else if let Some(status) = err.status() {
            if status.is_server_error() {
                Some("The server encountered an error. Try again later.".to_string())
            } else if status.as_u16() == 429 {
                Some("Rate limit exceeded. Wait a moment before retrying.".to_string())
            } else {
                None
            }
        } else {
            None
        };

        let retry_after = if err.is_timeout()
            || err
                .status()
                .map(|s| s.as_u16() == 429 || s.is_server_error())
                .unwrap_or(false)
        {
            Some(Duration::from_secs(5))
        } else {
            None
        };

        PubMedError::RequestError {
            message,
            source: err,
            suggestion,
            retry_after,
        }
    }
}

/// Convert serde_json::Error to PubMedError with context
impl From<serde_json::Error> for PubMedError {
    fn from(err: serde_json::Error) -> Self {
        PubMedError::JsonError {
            message: err.to_string(),
            source: err,
            context: Some("Failed to parse API response".to_string()),
        }
    }
}

pub type Result<T> = result::Result<T, PubMedError>;

impl RetryableError for PubMedError {
    fn is_retryable(&self) -> bool {
        match self {
            // Network errors are typically transient
            PubMedError::RequestError { source, .. } => {
                // Check if it's a network-related error
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if source.is_timeout() || source.is_connect() {
                        return true;
                    }
                }

                #[cfg(target_arch = "wasm32")]
                {
                    if source.is_timeout() {
                        return true;
                    }
                }

                // Check for server errors (5xx)
                if let Some(status) = source.status() {
                    return status.is_server_error() || status.as_u16() == 429;
                }

                // DNS and other network errors
                !source.is_builder() && !source.is_redirect() && !source.is_decode()
            }

            // Rate limiting should be retried after delay
            PubMedError::RateLimitExceeded { .. } => true,

            // API errors might be retryable if they indicate server issues
            PubMedError::ApiError {
                status, message, ..
            } => {
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
            PubMedError::JsonError { .. }
            | PubMedError::XmlError { .. }
            | PubMedError::XmlParseError { .. }
            | PubMedError::ArticleNotFound { .. }
            | PubMedError::PmcNotAvailable { .. }
            | PubMedError::PmcNotAvailableById { .. }
            | PubMedError::InvalidPmid { .. }
            | PubMedError::InvalidQuery { .. }
            | PubMedError::IoError { .. }
            | PubMedError::SearchLimitExceeded { .. } => false,
        }
    }

    fn retry_reason(&self) -> &str {
        if self.is_retryable() {
            match self {
                PubMedError::RequestError { source, .. } if source.is_timeout() => {
                    "Request timeout"
                }
                #[cfg(not(target_arch = "wasm32"))]
                PubMedError::RequestError { source, .. } if source.is_connect() => {
                    "Connection error"
                }
                PubMedError::RequestError { .. } => "Network error",
                PubMedError::RateLimitExceeded { .. } => "Rate limit exceeded",
                PubMedError::ApiError { status, .. } => match status {
                    429 => "Rate limit exceeded",
                    500..=599 => "Server error",
                    _ => "Temporary API error",
                },
                _ => "Transient error",
            }
        } else {
            match self {
                PubMedError::JsonError { .. } => "Invalid JSON response",
                PubMedError::XmlError { .. } | PubMedError::XmlParseError { .. } => {
                    "Invalid XML response"
                }
                PubMedError::ArticleNotFound { .. } => "Article does not exist",
                PubMedError::PmcNotAvailable { .. } | PubMedError::PmcNotAvailableById { .. } => {
                    "Content not available"
                }
                PubMedError::InvalidPmid { .. } => "Invalid input",
                PubMedError::InvalidQuery { .. } => "Invalid query",
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
        let err = PubMedError::from(json_err);

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid JSON response");
    }

    #[test]
    fn test_xml_error_not_retryable() {
        let err = PubMedError::XmlError {
            message: "Invalid XML format".to_string(),
            context: None,
            suggestion: Some("Check if the XML is well-formed".to_string()),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid XML response");
    }

    #[test]
    fn test_xml_parse_error_not_retryable() {
        let err = PubMedError::XmlParseError {
            message: "Failed to parse element".to_string(),
            suggestion: Some("Verify XML structure".to_string()),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid XML response");
    }

    #[test]
    fn test_article_not_found_not_retryable() {
        let err = PubMedError::ArticleNotFound {
            pmid: "12345".to_string(),
            suggestion: "Verify the PMID exists in PubMed".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Article does not exist");
        assert!(format!("{}", err).contains("12345"));
        assert!(format!("{}", err).contains("Verify"));
    }

    #[test]
    fn test_pmc_not_available_not_retryable() {
        let err = PubMedError::PmcNotAvailable {
            pmid: "67890".to_string(),
            suggestion: "This article may not have full-text available in PMC".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Content not available");
        assert!(format!("{}", err).contains("67890"));
    }

    #[test]
    fn test_pmc_not_available_by_id_not_retryable() {
        let err = PubMedError::PmcNotAvailableById {
            pmcid: "PMC123456".to_string(),
            suggestion: "Check if the PMCID is correct".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Content not available");
        assert!(format!("{}", err).contains("PMC123456"));
    }

    #[test]
    fn test_invalid_pmid_not_retryable() {
        let err = PubMedError::InvalidPmid {
            pmid: "invalid".to_string(),
            suggestion: "PMID must be a numeric identifier".to_string(),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid input");
        assert!(format!("{}", err).contains("invalid"));
    }

    #[test]
    fn test_invalid_query_not_retryable() {
        let err = PubMedError::InvalidQuery {
            message: "Empty query string".to_string(),
            suggestion: Some("Provide a non-empty search query".to_string()),
        };

        assert!(!err.is_retryable());
        assert_eq!(err.retry_reason(), "Invalid query");
        assert!(format!("{}", err).contains("Empty query"));
    }

    #[test]
    fn test_io_error_not_retryable() {
        let err = PubMedError::IoError {
            message: "File not found".to_string(),
            suggestion: Some("Check if the file path is correct".to_string()),
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
            suggestion: "Reduce the number of requested results or use pagination".to_string(),
        };

        assert!(!err.is_retryable());
        assert!(format!("{}", err).contains("15000"));
        assert!(format!("{}", err).contains("10000"));
    }

    // Tests for retryable errors

    #[test]
    fn test_rate_limit_exceeded_is_retryable() {
        let err = PubMedError::RateLimitExceeded {
            retry_after: Some(Duration::from_secs(60)),
            suggestion: "Wait before making more requests".to_string(),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Rate limit exceeded");
        assert!(format!("{}", err).contains("60 seconds"));
    }

    #[test]
    fn test_api_error_429_is_retryable() {
        let err = PubMedError::ApiError {
            status: 429,
            message: "Too Many Requests".to_string(),
            context: None,
            suggestion: Some("Slow down your request rate".to_string()),
            retry_after: Some(Duration::from_secs(30)),
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
            context: None,
            suggestion: Some("Try again later".to_string()),
            retry_after: Some(Duration::from_secs(5)),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Server error");
    }

    #[test]
    fn test_api_error_503_is_retryable() {
        let err = PubMedError::ApiError {
            status: 503,
            message: "Service Unavailable".to_string(),
            context: None,
            suggestion: Some("The server is temporarily unavailable".to_string()),
            retry_after: Some(Duration::from_secs(10)),
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Server error");
    }

    #[test]
    fn test_api_error_temporarily_unavailable_is_retryable() {
        let err = PubMedError::ApiError {
            status: 400,
            message: "Service temporarily unavailable".to_string(),
            context: None,
            suggestion: None,
            retry_after: None,
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Temporary API error");
    }

    #[test]
    fn test_api_error_timeout_message_is_retryable() {
        let err = PubMedError::ApiError {
            status: 408,
            message: "Request timeout".to_string(),
            context: None,
            suggestion: None,
            retry_after: None,
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Temporary API error");
    }

    #[test]
    fn test_api_error_connection_message_is_retryable() {
        let err = PubMedError::ApiError {
            status: 400,
            message: "Connection reset by peer".to_string(),
            context: None,
            suggestion: None,
            retry_after: None,
        };

        assert!(err.is_retryable());
        assert_eq!(err.retry_reason(), "Temporary API error");
    }

    #[test]
    fn test_api_error_404_not_retryable() {
        let err = PubMedError::ApiError {
            status: 404,
            message: "Not Found".to_string(),
            context: None,
            suggestion: None,
            retry_after: None,
        };

        assert!(!err.is_retryable());
    }

    #[test]
    fn test_api_error_400_not_retryable() {
        let err = PubMedError::ApiError {
            status: 400,
            message: "Bad Request".to_string(),
            context: None,
            suggestion: None,
            retry_after: None,
        };

        assert!(!err.is_retryable());
    }

    // Tests for error display formatting

    #[test]
    fn test_error_display_messages() {
        let test_cases = vec![
            (
                PubMedError::XmlError {
                    message: "test".to_string(),
                    context: None,
                    suggestion: None,
                },
                "XML parsing failed: test",
            ),
            (
                PubMedError::XmlParseError {
                    message: "test error".to_string(),
                    suggestion: None,
                },
                "XML parsing error: test error",
            ),
            (
                PubMedError::InvalidQuery {
                    message: "bad query".to_string(),
                    suggestion: None,
                },
                "Invalid query: bad query",
            ),
            (
                PubMedError::RateLimitExceeded {
                    retry_after: None,
                    suggestion: "Wait before retrying".to_string(),
                },
                "API rate limit exceeded",
            ),
        ];

        for (error, expected_message) in test_cases {
            assert_eq!(format!("{}", error), expected_message);
        }
    }

    #[test]
    fn test_error_display_with_fields() {
        let err = PubMedError::ArticleNotFound {
            pmid: "12345".to_string(),
            suggestion: "Check if the PMID exists".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("Article not found"));
        assert!(display.contains("12345"));
        assert!(display.contains("Check if the PMID exists"));

        let err = PubMedError::ApiError {
            status: 500,
            message: "Server Error".to_string(),
            context: Some("Fetching article".to_string()),
            suggestion: Some("Try again".to_string()),
            retry_after: Some(Duration::from_secs(10)),
        };
        let display = format!("{}", err);
        assert!(display.contains("500"));
        assert!(display.contains("Server Error"));
        assert!(display.contains("Fetching article"));
        assert!(display.contains("Try again"));
        assert!(display.contains("10 seconds"));
    }

    #[test]
    fn test_result_type_alias() {
        // Test that Result<T> type alias works correctly
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_err() -> Result<String> {
            Err(PubMedError::RateLimitExceeded {
                retry_after: None,
                suggestion: "Wait".to_string(),
            })
        }

        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_with_context_display() {
        let err = PubMedError::XmlError {
            message: "Missing element".to_string(),
            context: Some("Parsing article metadata".to_string()),
            suggestion: Some("Verify XML structure".to_string()),
        };
        let display = format!("{}", err);
        assert!(display.contains("Missing element"));
        assert!(display.contains("context: Parsing article metadata"));
        assert!(display.contains("Suggestion: Verify XML structure"));
    }

    #[test]
    fn test_from_reqwest_error() {
        // Test that reqwest errors are converted properly
        // This is a simplified test since we can't easily create real reqwest errors
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err = PubMedError::from(json_err);

        match err {
            PubMedError::JsonError { context, .. } => {
                assert_eq!(context, Some("Failed to parse API response".to_string()));
            }
            _ => panic!("Expected JsonError"),
        }
    }
}
