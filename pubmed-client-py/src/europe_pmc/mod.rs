//! Europe PMC module for Python bindings
//!
//! This module contains the Europe PMC client, its data models, and the
//! `(source, id)` addressing helpers shared by both.

pub mod client;
pub mod models;

use pyo3::PyResult;
use pyo3::exceptions::PyValueError;

use pubmed_client::{EuropePmcId, EuropePmcSource};

// Re-export public types
pub use client::PyEuropePmcClient;
pub use models::{
    PyEuropePmcCitation, PyEuropePmcDatabaseLink, PyEuropePmcDbCrossReferenceInfo,
    PyEuropePmcReference, PyEuropePmcResult, PyEuropePmcSearchResponse,
};

/// Resolve the `(source, id)` pair a Python call addresses.
///
/// Europe PMC identifies every record by a source database plus an id, but
/// requiring callers to spell both out for the common cases would be noise, so
/// three forms are accepted:
///
/// * a fully-qualified `"SOURCE/ID"` string (e.g. `"PPR/PPR123456"`), which
///   takes precedence over any `source` argument;
/// * an explicit `source` plus a bare id;
/// * a bare id alone — a `PMC`-prefixed id implies the `PMC` source, anything
///   else is treated as a PubMed (`MED`) record.
pub(crate) fn resolve_id(id: &str, source: Option<&str>) -> PyResult<EuropePmcId> {
    let id = id.trim();
    if id.is_empty() {
        return Err(PyValueError::new_err("id must not be empty"));
    }

    if id.contains('/') {
        return id
            .parse::<EuropePmcId>()
            .map_err(|e| PyValueError::new_err(format!("invalid Europe PMC id {id:?}: {e}")));
    }

    let source = match source {
        Some(source) if !source.trim().is_empty() => EuropePmcSource::from(source),
        _ if id.to_ascii_uppercase().starts_with("PMC") => EuropePmcSource::Pmc,
        _ => EuropePmcSource::Med,
    };

    if source == EuropePmcSource::Pmc {
        return EuropePmcId::pmc(id)
            .map_err(|e| PyValueError::new_err(format!("invalid PMC id {id:?}: {e}")));
    }

    Ok(EuropePmcId::new(source, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_pmc_id_defaults_to_pmc_source() {
        let id = resolve_id("PMC3258128", None).unwrap();
        assert_eq!(id.to_string(), "PMC/PMC3258128");
    }

    #[test]
    fn bare_numeric_id_defaults_to_med_source() {
        let id = resolve_id("33515491", None).unwrap();
        assert_eq!(id.to_string(), "MED/33515491");
    }

    #[test]
    fn explicit_pmc_source_normalizes_a_bare_number() {
        assert_eq!(
            resolve_id("3258128", Some("pmc")).unwrap().to_string(),
            "PMC/PMC3258128"
        );
    }

    #[test]
    fn qualified_id_wins_over_source_argument() {
        let id = resolve_id("PPR/PPR123456", Some("MED")).unwrap();
        assert_eq!(id.source, EuropePmcSource::Ppr);
    }

    #[test]
    fn invalid_ids_are_rejected() {
        assert!(resolve_id("   ", None).is_err());
        assert!(resolve_id("MED/", None).is_err());
    }
}
