//! Utility functions for Python bindings
//!
//! This module provides runtime management and error conversion utilities.

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

// ================================================================================================
// Runtime Management
// ================================================================================================

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Get or create a shared Tokio runtime for blocking operations
///
/// Uses a process-wide singleton so the runtime (and its worker thread pool)
/// is created once and reused across all method calls. This avoids per-call
/// overhead and allows connection pools and rate limiters to persist.
#[allow(clippy::expect_used)]
pub fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime"))
}

// ================================================================================================
// Exception Hierarchy
// ================================================================================================

pyo3::create_exception!(
    pubmed_client,
    PubMedException,
    PyException,
    "Base exception for all PubMed client errors."
);
pyo3::create_exception!(
    pubmed_client,
    ParseException,
    PubMedException,
    "XML or JSON parsing failed."
);
pyo3::create_exception!(
    pubmed_client,
    RequestException,
    PubMedException,
    "HTTP request failed (network, timeout, DNS)."
);
pyo3::create_exception!(
    pubmed_client,
    InvalidQueryException,
    PubMedException,
    "Invalid query structure or parameters."
);
pyo3::create_exception!(
    pubmed_client,
    RateLimitException,
    PubMedException,
    "API rate limit exceeded (HTTP 429)."
);
pyo3::create_exception!(
    pubmed_client,
    ApiException,
    PubMedException,
    "API returned an error HTTP status code."
);
pyo3::create_exception!(
    pubmed_client,
    SearchLimitException,
    PubMedException,
    "Requested result count exceeds the maximum retrievable limit."
);
pyo3::create_exception!(
    pubmed_client,
    HistorySessionException,
    PubMedException,
    "History server session expired or WebEnv unavailable."
);

// ================================================================================================
// Error Conversion
// ================================================================================================

/// Convert a `PubMedError` into the appropriate typed Python exception.
///
/// The match is exhaustive (no wildcard arm) so that adding a new variant to
/// `PubMedError` produces a compile error here, forcing an explicit mapping.
pub fn to_py_err(err: ::pubmed_client::error::PubMedError) -> PyErr {
    use ::pubmed_client::error::PubMedError;
    match err {
        PubMedError::ParseError(ref e) => PyErr::new::<ParseException, _>(e.to_string()),
        PubMedError::RequestError(ref e) => {
            PyErr::new::<RequestException, _>(format!("HTTP request failed: {e}"))
        }
        PubMedError::InvalidQuery(ref msg) => {
            PyErr::new::<InvalidQueryException, _>(format!("Invalid query: {msg}"))
        }
        PubMedError::RateLimitExceeded => {
            PyErr::new::<RateLimitException, _>("API rate limit exceeded")
        }
        PubMedError::ApiError {
            status,
            ref message,
        } => PyErr::new::<ApiException, _>(format!("API error {status}: {message}")),
        PubMedError::SearchLimitExceeded { requested, maximum } => {
            PyErr::new::<SearchLimitException, _>(format!(
                "Search limit exceeded: requested {requested}, maximum is {maximum}"
            ))
        }
        PubMedError::HistorySessionError(ref msg) => PyErr::new::<HistorySessionException, _>(
            format!("History session expired or invalid: {msg}"),
        ),
        PubMedError::WebEnvNotAvailable => {
            PyErr::new::<HistorySessionException, _>("WebEnv not available in search result")
        }
    }
}
