//! PubMed XML parsing and data models
//!
//! This module provides parsers for PubMed XML responses and the data types
//! that represent PubMed article metadata.

pub mod models;
pub mod parser;

// Re-export public types
pub use models::{
    AbstractSection, Affiliation, ArticleSummary, Author, ChemicalConcept, CitationMatch,
    CitationMatchStatus, CitationMatches, CitationQuery, Citations, DatabaseCount, DatabaseInfo,
    EPostResult, FieldInfo, GlobalQueryResults, HistorySession, LinkInfo, MeshHeading,
    MeshQualifier, MeshTerm, PmcLinks, PubMedArticle, RelatedArticles, SearchResult,
    SpellCheckResult, SpelledQuerySegment, SupplementalConcept,
};
pub use parser::{parse_article_from_xml, parse_articles_from_xml};
