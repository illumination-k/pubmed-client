//! Europe PMC REST API data models and JSON response parsers.
//!
//! Europe PMC (<https://europepmc.org>) is operated by EBI and aggregates
//! literature from many sources (PubMed/`MED`, PMC, preprints/`PPR`, patents,
//! agricultural literature, ...). Unlike the NCBI E-utilities, its REST API
//! returns JSON for metadata endpoints and JATS XML for full text.
//!
//! This module provides pure, network-free parsing of the JSON responses.
//! Full text is JATS and is parsed by [`crate::pmc::parser::parse_pmc_xml`].

mod citations;
mod de;
mod links;
mod models;
mod references;
mod search;

pub use citations::{EuropePmcCitationList, parse_citations_response};
pub use links::{EuropePmcDatabaseLinkList, parse_database_links_response};
pub use models::{
    EuropePmcCitation, EuropePmcDatabaseLink, EuropePmcDbCrossReferenceInfo, EuropePmcReference,
    EuropePmcResult,
};
pub use references::{EuropePmcReferenceList, parse_references_response};
pub use search::{EuropePmcSearchResponse, parse_search_response};
