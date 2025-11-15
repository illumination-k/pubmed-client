//! PMC module for Python bindings
//!
//! This module contains PMC client and data models.

pub mod client;
pub mod models;

// Re-export public types
pub use client::PyPmcClient;
pub use models::{
    PyArticleSection, PyExtractedFigure, PyFigure, PyPmcAffiliation, PyPmcAuthor, PyPmcFullText,
    PyReference, PyTable,
};
