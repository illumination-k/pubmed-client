#![deny(
    clippy::panic,
    clippy::absolute_paths,
    clippy::print_stderr,
    clippy::print_stdout
)]

//! # PubMed Parser
//!
//! XML parsers and data models for PubMed and PMC (PubMed Central) articles.
//!
//! This crate provides pure, stateless parsing functions and data types for working
//! with PubMed and PMC XML responses. It has no network dependencies and can be used
//! independently of any HTTP client.

pub mod common;
pub mod error;
pub mod pmc;
pub mod pubmed;

// Re-export main types for convenience
pub use common::{Affiliation, Author, PmcId, PubMedId};
pub use error::{ParseError, Result};
