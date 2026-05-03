//! Common data structures and utilities shared between PubMed and PMC modules

pub mod domain;
pub mod ids;
pub mod xml_utils;

// Re-export common types
pub use domain::{Affiliation, Author, HistoryDate, PublicationDate, format_author_name};
pub use ids::{PmcId, PubMedId};
