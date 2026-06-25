//! PMC (PubMed Central) XML parsing and data models
//!
//! This module provides parsers for PMC full-text XML responses
//! and the data types that represent PMC articles.

pub mod domain;
pub mod oa_api;
pub mod parser;

// Re-export domain types as the primary API
pub use domain::{
    Abstract, AbstractSection, ArticleMeta, Back, Body, Definition, Figure, Formula, Front,
    FundingInfo, JournalMeta, KeywordGroup, License, Permissions, PmcArticle, Reference,
    RelatedArticle, Section, SectionIter, SectionKind, SubjectGroup, SupplementaryMaterial, Table,
    TableCell, TableRow, TitleGroup,
};

// Re-export OA types
pub use oa_api::OaSubsetInfo;
pub use parser::parse_pmc_xml;
