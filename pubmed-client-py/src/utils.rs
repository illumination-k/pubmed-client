//! Utility functions for Python bindings
//!
//! This module provides runtime management and error conversion utilities.

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use tokio::runtime::Runtime;

// ================================================================================================
// Runtime Management
// ================================================================================================

/// Get or create a Tokio runtime for blocking operations
///
/// `Runtime::new` only fails when the OS cannot create the underlying event
/// loop / worker threads — an unrecoverable environment error — so this helper
/// is allowed to `expect` here.
#[allow(clippy::expect_used)]
pub fn get_runtime() -> Runtime {
    Runtime::new().expect("Failed to create Tokio runtime")
}

// ================================================================================================
// Error Handling
// ================================================================================================

/// Convert Rust errors to Python exceptions
pub fn to_py_err(err: ::pubmed_client::error::PubMedError) -> PyErr {
    PyErr::new::<PyException, _>(format!("{}", err))
}
