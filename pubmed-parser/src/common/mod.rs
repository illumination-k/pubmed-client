//! Common data structures and utilities shared between PubMed and PMC modules

pub mod ids;
pub mod models;

// Crate-internal: these helpers only ever serve the PubMed and PMC parsers in
// this crate, and are not part of the published surface.
pub(crate) mod xml_utils;

// Re-export common types
pub use ids::{PmcId, PubMedId};
pub use models::{Affiliation, Author, HistoryDate, PublicationDate, format_author_name};
