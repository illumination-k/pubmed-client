//! XML preprocessing utilities for PubMed parser
//!
//! This module re-exports common XML preprocessing utilities.
//! The actual implementation has been moved to `crate::common::xml_utils`
//! for sharing between PubMed and PMC parsers.

// Re-export from common module for backward compatibility
pub(crate) use crate::common::xml_utils::strip_inline_html_tags;
