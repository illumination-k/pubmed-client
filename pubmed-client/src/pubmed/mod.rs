//! PubMed client for searching and fetching article metadata
//!
//! This module provides functionality to interact with PubMed E-utilities APIs
//! for searching biomedical literature and retrieving article metadata.
//!
//! The client functionality is split across focused modules under [`client`]:
//! - `client/mod.rs` - Core client struct, constructors, search, and fetch operations
//! - `client/summary` - ESummary API for lightweight article metadata
//! - `client/history` - History server operations (EPost, fetch from history, streaming)
//! - `client/einfo` - Database information (EInfo API)
//! - `client/elink` - Cross-database linking (ELink API)
//! - `client/citmatch` - Citation matching (ECitMatch API)
//! - `client/egquery` - Global database queries (EGQuery API)
//! - `client/espell` - Spell checking (ESpell API)

pub mod client;
pub mod models;
pub mod parser;
pub mod query;
pub mod responses;

// Re-export public types
pub use client::PubMedClient;
pub use models::{
    Affiliation, ArticleSummary, Author, ChemicalConcept, CitationMatch, CitationMatchStatus,
    CitationMatches, CitationQuery, Citations, DatabaseCount, DatabaseInfo, EPostResult, FieldInfo,
    GlobalQueryResults, HistorySession, LinkInfo, MeshHeading, MeshQualifier, MeshTerm, PmcLinks,
    PubMedArticle, RelatedArticles, SearchResult, SpellCheckResult, SpelledQuerySegment,
    SupplementalConcept,
};
pub use parser::{parse_article_from_xml, parse_articles_from_xml};
pub use query::{ArticleType, Language, PubDate, SearchQuery, SortOrder};
