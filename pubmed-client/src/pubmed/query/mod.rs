//! Query builder for constructing PubMed search queries with filters
//!
//! This module provides a flexible query builder for constructing complex PubMed search queries
//! using E-utilities syntax. The query builder supports:
//!
//! - Field-specific searches (title, abstract, journal, etc.)
//! - Date filtering with flexible precision
//! - MeSH term filtering
//! - Author and affiliation filtering
//! - Boolean logic operations (AND, OR, NOT)
//! - Article type and language filtering
//! - Query validation and optimization
//!
//! # Examples
//!
//! Basic search:
//! ```
//! use pubmed_client_rs::pubmed::SearchQuery;
//!
//! let query = SearchQuery::new()
//!     .query("covid-19 treatment")
//!     .published_after(2020)
//!     .free_full_text()
//!     .limit(10);
//! ```
//!
//! Complex boolean search:
//! ```
//! use pubmed_client_rs::pubmed::{SearchQuery, ArticleType, Language};
//!
//! let ai_query = SearchQuery::new()
//!     .title_contains("machine learning")
//!     .or(SearchQuery::new().mesh_term("Artificial Intelligence"));
//!
//! let medical_query = SearchQuery::new()
//!     .mesh_term("Medicine")
//!     .and(SearchQuery::new().human_studies_only());
//!
//! let final_query = ai_query
//!     .and(medical_query)
//!     .article_type(ArticleType::Review)
//!     .language(Language::English)
//!     .published_between(2020, Some(2023));
//! ```

// Core modules
mod builder;
mod date;
mod filters;

// Feature modules
mod advanced;
mod boolean;
mod dates;
mod search;
mod validation;

// Re-export all public types
pub use builder::SearchQuery;
pub use date::PubDate;
pub use filters::{ArticleType, Language};
