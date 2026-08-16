//! The error envelope that crosses the FFI boundary.
//!
//! Failures reach Go as a JSON object rather than a bare sentence:
//!
//! ```json
//! {"kind": "article_not_found", "message": "Article not found: PMID 1", "status": null}
//! ```
//!
//! The `kind` is what lets the Go package expose sentinel errors
//! (`ErrNotFound`, `ErrRateLimited`, …) instead of asking callers to match on
//! message text. Go treats an unparseable envelope as `{kind: "unknown"}` with
//! the whole string as the message, so adding a variant here can never break an
//! older caller.

use serde::Serialize;

use pubmed_client::{ParseError, PubMedError};

/// Machine-readable classification of a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// An argument was null, not UTF-8, or not the JSON the call expected.
    InvalidArgument,
    /// The caller's cancellation token fired before the request completed.
    Cancelled,
    /// The HTTP request itself failed (DNS, connect, timeout, TLS, …).
    Request,
    /// NCBI answered with a non-success status; `status` carries the code.
    Api,
    /// The client's rate limiter or NCBI rejected the call as too frequent.
    RateLimit,
    /// The query was malformed or empty.
    InvalidQuery,
    /// More results were requested than PubMed can return.
    SearchLimitExceeded,
    /// A history-server (WebEnv) session expired or was rejected.
    HistorySession,
    /// A history-server operation ran without a WebEnv session.
    WebenvUnavailable,
    /// PubMed has no article for the requested PMID.
    ArticleNotFound,
    /// The article has no PMC full text available.
    PmcNotAvailable,
    /// The PMID was not in a valid format.
    InvalidPmid,
    /// The PMCID was not in a valid format.
    InvalidPmcid,
    /// The response was not valid XML.
    XmlParse,
    /// The response was not valid JSON.
    JsonParse,
    /// A filesystem operation failed.
    Io,
    /// A panic was caught at the boundary; always a bug in this crate.
    Panic,
    /// Anything else, including serialization failures inside the shim.
    Internal,
}

/// An error on its way out to Go.
#[derive(Debug, Serialize)]
pub struct ShimError {
    /// Machine-readable classification.
    pub kind: ErrorKind,
    /// Human-readable description, taken from the underlying error's `Display`.
    pub message: String,
    /// HTTP status, present only for [`ErrorKind::Api`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl ShimError {
    /// Build an error of `kind` with `message`.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            status: None,
        }
    }

    /// A null, non-UTF-8, or structurally invalid argument.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidArgument, message)
    }

    /// A failure inside the shim itself rather than in `pubmed-client`.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    /// The caller cancelled the call.
    pub fn cancelled() -> Self {
        Self::new(ErrorKind::Cancelled, "call was cancelled by the caller")
    }

    /// A caught panic, reported rather than allowed to unwind into Go.
    pub fn panicked(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Panic, message)
    }

    /// Render the envelope Go parses.
    ///
    /// Serialization of this type cannot realistically fail, but a fallback
    /// keeps the message rather than losing it if it ever does.
    pub fn to_envelope(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.message.clone())
    }
}

impl From<ParseError> for ShimError {
    fn from(error: ParseError) -> Self {
        let message = error.to_string();
        let kind = match error {
            ParseError::XmlError(_) => ErrorKind::XmlParse,
            ParseError::JsonError(_) => ErrorKind::JsonParse,
            ParseError::ArticleNotFound { .. } => ErrorKind::ArticleNotFound,
            ParseError::PmcNotAvailable { .. } => ErrorKind::PmcNotAvailable,
            ParseError::InvalidPmid { .. } => ErrorKind::InvalidPmid,
            ParseError::InvalidPmcid { .. } => ErrorKind::InvalidPmcid,
            ParseError::IoError { .. } => ErrorKind::Io,
        };
        Self::new(kind, message)
    }
}

impl From<PubMedError> for ShimError {
    fn from(error: PubMedError) -> Self {
        let message = error.to_string();

        // Matched exhaustively on purpose: a new variant upstream should fail
        // the workspace build here rather than silently degrade to `internal`.
        match error {
            PubMedError::ParseError(inner) => Self::from(inner),
            PubMedError::RequestError(_) => Self::new(ErrorKind::Request, message),
            PubMedError::InvalidQuery(_) => Self::new(ErrorKind::InvalidQuery, message),
            PubMedError::RateLimitExceeded => Self::new(ErrorKind::RateLimit, message),
            PubMedError::ApiError { status, .. } => Self {
                kind: ErrorKind::Api,
                message,
                status: Some(status),
            },
            PubMedError::SearchLimitExceeded { .. } => {
                Self::new(ErrorKind::SearchLimitExceeded, message)
            }
            PubMedError::HistorySessionError(_) => Self::new(ErrorKind::HistorySession, message),
            PubMedError::WebEnvNotAvailable => Self::new(ErrorKind::WebenvUnavailable, message),
        }
    }
}

/// Result type used by every boundary function body.
pub type ShimResult<T> = Result<T, ShimError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_carries_kind_and_message() {
        let envelope = ShimError::invalid_argument("nope").to_envelope();
        assert_eq!(
            envelope, r#"{"kind":"invalid_argument","message":"nope"}"#,
            "{envelope}"
        );
    }

    #[test]
    fn api_errors_carry_the_status() {
        let error = ShimError::from(PubMedError::ApiError {
            status: 429,
            message: "slow down".to_string(),
        });
        assert_eq!(error.kind, ErrorKind::Api);
        assert_eq!(error.status, Some(429));
        assert!(error.to_envelope().contains("\"status\":429"));
    }

    #[test]
    fn parse_errors_keep_their_specific_kind() {
        let error = ShimError::from(PubMedError::ParseError(ParseError::ArticleNotFound {
            pmid: "1".to_string(),
        }));
        assert_eq!(error.kind, ErrorKind::ArticleNotFound);
        assert!(error.message.contains("PMID 1"), "{}", error.message);
    }

    #[test]
    fn pmc_unavailable_is_distinguishable() {
        let error = ShimError::from(ParseError::PmcNotAvailable {
            id: "PMC1".to_string(),
        });
        assert_eq!(error.kind, ErrorKind::PmcNotAvailable);
    }
}
