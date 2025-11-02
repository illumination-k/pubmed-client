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
