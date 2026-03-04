//! PMC (PubMed Central) XML parsing and data models
//!
//! This module provides parsers for PMC full-text XML responses
//! and the data types that represent PMC articles.

pub mod domain;
pub mod oa_api;
pub mod parser;

// Re-export parser models (backward compatibility)
pub use parser::models;
pub use parser::models::{
    Affiliation, ArticleSection, Author, ExtractedFigure, Figure, FundingInfo, HistoryDate,
    JournalInfo, PmcFullText, Reference, SupplementaryMaterial, Table,
};
// Re-export OA types
pub use oa_api::OaSubsetInfo;
pub use parser::{parse_pmc_xml, parse_pmc_xml_domain};
