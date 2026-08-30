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

// ================================================================================================
// JSON Conversion
// ================================================================================================

/// Convert a `serde_json` value into the closest native Python object.
///
/// Used for the untyped remainder of Europe PMC responses (`extra`), whose
/// shape the API is free to change; pinning it to a `#[pyclass]` would break
/// the moment Europe PMC adds a field, so it is handed to Python as plain
/// dicts, lists and scalars instead.
pub fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    use pyo3::types::PyList;
    use serde_json::Value;

    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => b.into_py_any(py),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py_any(py)
            } else if let Some(u) = n.as_u64() {
                u.into_py_any(py)
            } else if let Some(f) = n.as_f64() {
                f.into_py_any(py)
            } else {
                // serde_json only reaches here for arbitrary-precision numbers,
                // which have no lossless Python scalar; keep the digits.
                n.to_string().into_py_any(py)
            }
        }
        Value::String(s) => s.into_py_any(py),
        Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            list.into_py_any(py)
        }
        Value::Object(map) => json_map_to_py(py, map),
    }
}

/// Convert a `serde_json` object map into a Python dict.
pub fn json_map_to_py(
    py: Python<'_>,
    map: &serde_json::Map<String, serde_json::Value>,
) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    use pyo3::types::PyDict;

    let dict = PyDict::new(py);
    for (key, value) in map {
        dict.set_item(key, json_to_py(py, value)?)?;
    }
    dict.into_py_any(py)
}
