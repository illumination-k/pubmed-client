//! Common data structures shared between PubMed and PMC modules

pub mod models;

// Re-export common types
pub use models::{format_author_name, Affiliation, Author};
