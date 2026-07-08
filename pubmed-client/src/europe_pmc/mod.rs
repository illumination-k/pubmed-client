//! Europe PMC REST API client.
//!
//! [`EuropePmcClient`] talks to the Europe PMC RESTful Web Service
//! (<https://europepmc.org/RestfulWebService>), a complementary data source to
//! the NCBI E-utilities. It offers cross-source search (PubMed/`MED`, PMC,
//! preprints/`PPR`, patents, ...), JATS full-text retrieval, reference and
//! citation graphs, external database links, and supplementary file downloads.
//!
//! Records are addressed by a `(source, id)` pair via [`EuropePmcId`]. JSON
//! response models live in [`pubmed_parser::europe_pmc`] and are re-exported
//! from the crate root; full text reuses the JATS [`pubmed_parser::pmc::PmcArticle`].

mod citations;
mod client;
mod fulltext;
mod id;
mod links;
mod references;
mod search;
#[cfg(not(target_arch = "wasm32"))]
mod supplementary;

pub use client::EuropePmcClient;
pub use id::{EuropePmcId, EuropePmcSource};
pub use search::{EuropePmcSearchOptions, ResultType};

// Re-export the parser-side response/result models for convenience.
pub use pubmed_parser::europe_pmc::{
    EuropePmcCitation, EuropePmcCitationList, EuropePmcDatabaseLink, EuropePmcDatabaseLinkList,
    EuropePmcDbCrossReferenceInfo, EuropePmcReference, EuropePmcReferenceList, EuropePmcResult,
    EuropePmcSearchResponse,
};
