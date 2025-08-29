//! PMC (PubMed Central) client for fetching full-text articles
//!
//! This module provides functionality to interact with PMC APIs to fetch
//! full-text articles, check availability, and parse structured content.

pub mod client;
pub mod markdown;
pub mod models;
pub mod parser;
pub mod tar;

// Re-export public types
pub use client::PmcClient;
pub use markdown::{HeadingStyle, MarkdownConfig, PmcMarkdownConverter, ReferenceStyle};
pub use models::{
    Affiliation, ArticleSection, Author, Figure, FundingInfo, JournalInfo, PmcFullText, Reference,
    Table,
};
pub use parser::parse_pmc_xml;
pub use tar::PmcTarClient;
