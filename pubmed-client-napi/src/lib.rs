#![deny(clippy::all)]

mod client;
mod config;
mod error;
mod models;
mod query;

pub use client::PubMedClient;
pub use config::Config;
pub use models::{
    Article, Author, CitationMatch, CitationQuery, Citations, DatabaseCount, DatabaseInfo,
    EPostResult, ExtractedFigure, Figure, FullTextArticle, GlobalQueryResults, MarkdownOptions,
    OaSubsetInfo, PmcLinks, Reference, RelatedArticles, Section, SpellCheckResult, Summary,
};
pub use query::SearchQuery;
