//! Common data structures and utilities shared between PubMed and PMC modules

pub mod models;
pub mod xml_utils;

// Re-export common types
pub use models::{format_author_name, Affiliation, Author};
