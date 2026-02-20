//! PubMed module for Python bindings
//!
//! This module contains PubMed client and data models.

pub mod client;
pub mod models;

// Re-export public types
pub use client::PyPubMedClient;
pub use models::{
    PyAffiliation, PyArticleSummary, PyAuthor, PyCitationMatch, PyCitationMatches, PyCitationQuery,
    PyCitations, PyDatabaseCount, PyDatabaseInfo, PyGlobalQueryResults, PyPmcLinks,
    PyPubMedArticle, PyRelatedArticles,
};
