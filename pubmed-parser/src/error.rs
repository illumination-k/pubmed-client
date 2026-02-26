use std::result;

use thiserror::Error;

/// Error types for PubMed/PMC parsing operations
#[derive(Error, Debug)]
pub enum ParseError {
    /// XML parsing failed
    #[error("XML parsing failed: {0}")]
    XmlError(String),

    /// JSON parsing failed
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Article not found
    #[error("Article not found: PMID {pmid}")]
    ArticleNotFound { pmid: String },

    /// PMC full text not available
    #[error("PMC full text not available for {id}")]
    PmcNotAvailable { id: String },

    /// Invalid PMID format
    #[error("Invalid PMID format: {pmid}")]
    InvalidPmid { pmid: String },

    /// Invalid PMC ID format
    #[error("Invalid PMC ID format: {pmcid}")]
    InvalidPmcid { pmcid: String },

    /// IO error for file operations
    #[error("IO error: {message}")]
    IoError { message: String },
}

pub type Result<T> = result::Result<T, ParseError>;
