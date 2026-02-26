//! XML parsing utilities for PMC parser
//!
//! This module re-exports common XML parsing utilities.
//! The actual implementation has been moved to `crate::common::xml_utils`
//! for sharing between PubMed and PMC parsers.

// Re-export all common XML utilities
pub use crate::common::xml_utils::*;
