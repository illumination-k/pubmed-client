//! Europe PMC module for Python bindings
//!
//! This module contains the Europe PMC client, its data models, and the
//! `(source, id)` addressing helpers shared by both.

pub mod client;
pub mod models;

use pyo3::PyResult;
use pyo3::exceptions::PyValueError;

use pubmed_client::EuropePmcId;

// Re-export public types
pub use client::PyEuropePmcClient;
pub use models::{
    PyEuropePmcCitation, PyEuropePmcDatabaseLink, PyEuropePmcDbCrossReferenceInfo,
    PyEuropePmcReference, PyEuropePmcResult, PyEuropePmcSearchResponse,
};

/// Resolve the `(source, id)` pair a Python call addresses.
///
/// The three accepted forms are defined by [`EuropePmcId::resolve`]; this only
/// maps the core error into a Python `ValueError`.
pub(crate) fn resolve_id(id: &str, source: Option<&str>) -> PyResult<EuropePmcId> {
    EuropePmcId::resolve(id, source).map_err(|e| PyValueError::new_err(e.to_string()))
}
