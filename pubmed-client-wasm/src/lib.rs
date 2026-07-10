//! WebAssembly bindings for the PubMed client library
//!
//! This module provides JavaScript-compatible bindings for use in Node.js and browsers.

use wasm_bindgen::prelude::*;

mod client;
mod config;
mod error;
mod models;
mod query;

pub use client::WasmPubMedClient;
pub use config::WasmClientConfig;
pub use models::{
    JsArticle, JsAuthor, JsCitationMatch, JsCitationQuery, JsDatabaseCount, JsEPostResult,
    JsFigure, JsFullText, JsFunding, JsGlobalQueryResults, JsJournal, JsMarkdownOptions,
    JsOaSubsetInfo, JsReference, JsSection, JsSpellCheckResult, JsSummary, JsTable,
};
pub use query::WasmSearchQuery;

#[wasm_bindgen]
extern "C" {
    /// `Promise<JsArticle[]>`
    pub type JsPromiseArticles;

    /// `Promise<JsArticle>`
    pub type JsPromiseArticle;

    /// `Promise<JsFullText>`
    pub type JsPromiseFullText;

    /// `Promise<string | null>`
    pub type JsPromiseOptString;

    /// `Promise<string[]>`
    pub type JsPromiseStringArray;

    /// `Promise<JsSummary[]>`
    pub type JsPromiseSummaries;
}

// Set up panic handler and allocator for better WASM experience
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();

    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
}
