//! C-ABI shim exposing the PubMed / PMC client to Go via cgo.
//!
//! The ergonomic API lives in the Go package one directory up; this crate only
//! translates between it and `pubmed-client`.
//!
//! # Boundary conventions
//!
//! Rather than mirroring every Rust type in C, values cross the boundary as
//! JSON: each call returns a freshly allocated NUL-terminated C string that the
//! caller owns and must release with [`pubmed_string_free`]. A null return
//! signals failure, in which case `out_err` receives an owned error envelope
//! (also freed with [`pubmed_string_free`]) — see [`error`] for its shape. Go
//! re-parses the JSON into the typed structs in `models.go`, which keeps the
//! FFI surface a few dozen functions while the data model stays fully typed on
//! both sides.
//!
//! Like the Python and R bindings, calls are synchronous from the caller's
//! point of view: a process-wide Tokio runtime drives the async
//! `pubmed-client` API and blocks until completion. Unlike them, a call can be
//! interrupted: every function takes a nullable cancellation token that Go
//! wires to a `context.Context` (see [`cancel`]).
//!
//! Panics are caught at every boundary function and reported as errors — an
//! `extern "C"` function that unwinds would abort the whole Go process.
//!
//! # Module map
//!
//! | Module     | Contents                                                     |
//! | ---------- | ------------------------------------------------------------ |
//! | [`error`]  | The JSON error envelope and its `kind` taxonomy               |
//! | [`ffi`]    | Argument borrowing, result ownership, panic containment       |
//! | [`cancel`] | The Tokio runtime and the cancellation token                  |
//! | [`client`] | Handle lifecycle and configuration decoding                   |
//! | [`dto`]    | Projections of Rust models onto the JSON shapes Go decodes    |
//! | [`pubmed`] | E-utilities calls (ESearch, EFetch, ESummary, ELink, …)        |
//! | [`pmc`]    | PMC full text, XML, Markdown, and Open Access downloads       |
//! | [`europe_pmc`] | Europe PMC search, full text, and citation graphs         |
//! | [`query`]  | Replay of the Go query builder onto `SearchQuery`             |
//! | [`export`] | Citation export (BibTeX, RIS, CSL-JSON, NBIB)                 |

pub mod cancel;
pub mod client;
pub mod dto;
pub mod error;
pub mod europe_pmc;
pub mod export;
pub mod ffi;
pub mod pmc;
pub mod pubmed;
pub mod query;

// The exported symbols are what Go links against; re-exporting them here keeps
// the crate usable as a normal Rust library and gives the docs one entry point.
pub use cancel::{PubmedCancel, pubmed_cancel_free, pubmed_cancel_new, pubmed_cancel_trigger};
pub use client::{PubmedClient, pubmed_client_free, pubmed_client_new, pubmed_client_version};
pub use error::{ErrorKind, ShimError};
pub use europe_pmc::{
    europe_pmc_download_supplementary_files, europe_pmc_fetch_full_text, europe_pmc_fetch_xml,
    europe_pmc_get_citations, europe_pmc_get_database_links, europe_pmc_get_references,
    europe_pmc_search, europe_pmc_search_page,
};
pub use export::pubmed_export_articles;
pub use ffi::pubmed_string_free;
pub use pmc::{
    pmc_check_availability, pmc_clear_cache, pmc_download_files, pmc_extract_figures,
    pmc_fetch_full_text, pmc_fetch_markdown, pmc_fetch_xml, pmc_is_oa_subset,
};
pub use pubmed::{
    pubmed_fetch_all_by_pmids, pubmed_fetch_article, pubmed_fetch_articles, pubmed_fetch_summaries,
    pubmed_get_citations, pubmed_get_database_info, pubmed_get_database_list, pubmed_get_pmc_links,
    pubmed_get_related_articles, pubmed_global_query, pubmed_match_citations,
    pubmed_search_and_fetch, pubmed_search_and_fetch_summaries, pubmed_search_articles,
    pubmed_search_with_full_text, pubmed_spell_check,
};
pub use query::pubmed_query_build;
