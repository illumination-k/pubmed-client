#![deny(
    clippy::panic,
    clippy::absolute_paths,
    clippy::print_stderr,
    clippy::print_stdout
)]

//! # PubMed Formatter
//!
//! Citation export and markdown conversion for PubMed and PMC articles.
//!
//! This crate provides formatting functionality that transforms parsed article
//! data into various output formats:
//!
//! - **Citation Export**: BibTeX, RIS, CSL-JSON, NBIB formats for PubMed articles
//! - **Markdown Conversion**: Configurable PMC full-text to Markdown conversion

pub mod pmc;
pub mod pubmed;

// Re-export main types for convenience
pub use pmc::markdown::{HeadingStyle, MarkdownConfig, PmcMarkdownConverter, ReferenceStyle};
pub use pubmed::export::{ExportFormat, articles_to_bibtex, articles_to_csl_json, articles_to_ris};
