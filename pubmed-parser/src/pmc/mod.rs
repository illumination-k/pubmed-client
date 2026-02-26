//! PMC (PubMed Central) XML parsing and data models
//!
//! This module provides parsers for PMC full-text XML responses
//! and the data types that represent PMC articles.

pub mod models;
pub mod oa_api;
pub mod parser;

// Re-export public types
pub use models::{
    Affiliation, ArticleSection, Author, Figure, FundingInfo, JournalInfo, OaSubsetInfo,
    PmcFullText, Reference, Table,
};
pub use parser::parse_pmc_xml;
