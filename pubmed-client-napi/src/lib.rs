#![deny(clippy::all)]

mod client;
mod config;
mod error;
mod europe_pmc;
mod models;
mod query;

pub use client::PubMedClient;
pub use config::Config;
pub use europe_pmc::{
    EuropePmcCitationEntry, EuropePmcDatabaseLinkEntry, EuropePmcDbCrossReference,
    EuropePmcReferenceEntry, EuropePmcSearchPage, EuropePmcSearchResult,
};
pub use models::{
    Article, Author, CitationMatch, CitationQuery, Citations, DatabaseCount, DatabaseInfo,
    EPostResult, ExtractedFigure, Figure, FullTextArticle, GlobalQueryResults, MarkdownOptions,
    OaSubsetInfo, PmcLinks, Reference, RelatedArticles, Section, SpellCheckResult, Summary,
};
pub use query::{DateInput, SearchQuery};
