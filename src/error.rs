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

    /// Article not found
    #[error("Article not found: PMID {pmid}")]
    ArticleNotFound { pmid: String },

    /// PMC full text not available
    #[error("PMC full text not available for PMID {pmid}")]
    PmcNotAvailable { pmid: String },

    /// Invalid PMID format
    #[error("Invalid PMID format: {pmid}")]
    InvalidPmid { pmid: String },

    /// API rate limit exceeded
    #[error("API rate limit exceeded")]
    RateLimitExceeded,

    /// Generic API error
    #[error("API error: {message}")]
    ApiError { message: String },
}

pub type Result<T> = std::result::Result<T, PubMedError>;
