//! PubMed client for searching and fetching article metadata
//!
//! This module provides functionality to interact with PubMed E-utilities APIs
//! for searching biomedical literature and retrieving article metadata.

pub mod client;
pub mod models;
pub mod parser;
pub mod query;
pub mod responses;

// Re-export public types
pub use client::PubMedClient;
pub use models::{
    Affiliation, Author, ChemicalConcept, Citations, DatabaseInfo, FieldInfo, LinkInfo,
    MeshHeading, MeshQualifier, MeshTerm, PmcLinks, PubMedArticle, RelatedArticles,
    SupplementalConcept,
};
pub use parser::parse_article_from_xml;
pub use query::{ArticleType, Language, PubDate, SearchQuery};
