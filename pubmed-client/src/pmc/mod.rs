//! PMC (PubMed Central) client for fetching full-text articles
//!
//! This module provides functionality to interact with PMC APIs to fetch
//! full-text articles, check availability, and parse structured content.

pub mod client;
pub mod extracted;
pub mod tar;

// Re-export parser types from pubmed-parser
pub use pubmed_parser::pmc::oa_api;
pub use pubmed_parser::pmc::parser;

// Re-export formatter types from pubmed-formatter
pub use pubmed_formatter::pmc::markdown;

// Re-export public types
pub use client::PmcClient;
pub use extracted::ExtractedFigure;
pub use markdown::{HeadingStyle, MarkdownConfig, PmcMarkdownConverter, ReferenceStyle};
pub use oa_api::OaSubsetInfo;
pub use parser::parse_pmc_xml;
pub use pubmed_parser::common::{Affiliation, Author};
pub use pubmed_parser::pmc::{
    Figure, FundingInfo, JournalMeta, PmcArticle, Reference, Section, Table,
};
pub use tar::PmcTarClient;
