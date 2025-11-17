//! Common data structures and utilities shared between PubMed and PMC modules

pub mod ids;
pub mod models;
pub mod xml_utils;

// Re-export common types
pub use ids::{PmcId, PubMedId};
pub use models::{format_author_name, Affiliation, Author};
