//! PubMed client for searching and fetching article metadata
//!
//! This module provides functionality to interact with PubMed E-utilities APIs
//! for searching biomedical literature and retrieving article metadata.

pub mod client;
pub mod models;
pub mod parser;
pub mod responses;

// Re-export public types
pub use client::PubMedClient;
pub use models::{DatabaseInfo, FieldInfo, LinkInfo, PubMedArticle};
pub use parser::PubMedXmlParser;
