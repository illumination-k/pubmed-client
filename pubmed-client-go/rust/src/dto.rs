//! Projections of `pubmed-client` types onto the JSON shapes Go decodes.
//!
//! Most `pubmed-client` types are already `Serialize` and cross the boundary
//! untouched. The types here exist for the three cases where they are not:
//! the JATS domain tree (too deeply nested to mirror in Go), tuple-shaped
//! results, and types that derive neither `Serialize` nor `Deserialize`.

use serde::{Deserialize, Serialize};

use pubmed_client::{
    Author, EuropePmcCitation, EuropePmcDatabaseLink, EuropePmcDbCrossReferenceInfo,
    EuropePmcReference, EuropePmcResult, EuropePmcSearchResponse, Figure, JournalMeta, PmcArticle,
    PubMedArticle, Reference, Section,
};

/// Flattened projection of a [`PmcArticle`] for the Go bindings.
///
/// The JATS domain model is deeply nested (front / body / back); this borrows
/// the fields the Go `PMCArticle` struct exposes through the article's accessor
/// methods, so Go never has to mirror the full DTD tree.
#[derive(Serialize)]
pub struct PmcArticleDto<'a> {
    pmcid: String,
    pmid: Option<String>,
    title: Option<&'a str>,
    doi: Option<&'a str>,
    journal: &'a JournalMeta,
    volume: Option<&'a str>,
    issue: Option<&'a str>,
    abstract_text: Option<&'a str>,
    keywords: &'a [String],
    authors: &'a [Author],
    sections: &'a [Section],
    references: &'a [Reference],
    figures: Vec<&'a Figure>,
    figure_count: usize,
    table_count: usize,
}

impl<'a> From<&'a PmcArticle> for PmcArticleDto<'a> {
    fn from(article: &'a PmcArticle) -> Self {
        Self {
            pmcid: article.pmcid().to_string(),
            pmid: article.pmid().map(|pmid| pmid.to_string()),
            title: article.title(),
            doi: article.doi(),
            journal: article.journal(),
            volume: article.volume(),
            issue: article.issue(),
            abstract_text: article.abstract_text(),
            keywords: article.keywords(),
            authors: article.authors(),
            sections: article.sections(),
            references: article.references(),
            figures: article.all_figures(),
            figure_count: article.figure_count(),
            table_count: article.table_count(),
        }
    }
}

/// One `(article, full text)` pair from `Client::search_with_full_text`.
///
/// The Rust API returns a tuple, which would serialize as a two-element array;
/// naming the halves keeps the Go struct readable.
#[derive(Serialize)]
pub struct SearchFullTextResultDto<'a> {
    article: &'a PubMedArticle,
    full_text: Option<PmcArticleDto<'a>>,
}

impl<'a> SearchFullTextResultDto<'a> {
    /// Project one search-with-full-text pair.
    pub fn new(article: &'a PubMedArticle, full_text: Option<&'a PmcArticle>) -> Self {
        Self {
            article,
            full_text: full_text.map(PmcArticleDto::from),
        }
    }
}

/// A citation to look up through ECitMatch.
///
/// `CitationQuery` derives neither `Serialize` nor `Deserialize`, so requests
/// are decoded into this mirror and converted.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CitationQueryDto {
    journal: String,
    year: String,
    volume: String,
    first_page: String,
    author_name: String,
    key: String,
}

impl From<CitationQueryDto> for pubmed_client::CitationQuery {
    fn from(dto: CitationQueryDto) -> Self {
        pubmed_client::CitationQuery::new(
            &dto.journal,
            &dto.year,
            &dto.volume,
            &dto.first_page,
            &dto.author_name,
            &dto.key,
        )
    }
}

// ================================================================================================
// Europe PMC
// ================================================================================================
//
// The Europe PMC models flatten their unmodelled fields into the record itself
// (`#[serde(flatten)] extra`), which would leave Go with no way to reach them:
// a Go struct silently drops keys it has no field for. These projections nest
// that remainder under `extra` instead, so Go can decode it as a map while the
// modelled fields stay typed.

/// Projection of a [`EuropePmcResult`] for the Go bindings.
#[derive(Serialize)]
pub struct EuropePmcResultDto<'a> {
    id: &'a str,
    source: &'a str,
    /// Fully-qualified Europe PMC address ("SOURCE/ID").
    europe_pmc_id: String,
    pmid: Option<&'a str>,
    pmcid: Option<&'a str>,
    doi: Option<&'a str>,
    title: Option<&'a str>,
    author_string: Option<&'a str>,
    journal_title: Option<&'a str>,
    pub_year: Option<&'a str>,
    is_open_access: Option<&'a str>,
    extra: &'a serde_json::Map<String, serde_json::Value>,
}

impl<'a> From<&'a EuropePmcResult> for EuropePmcResultDto<'a> {
    fn from(result: &'a EuropePmcResult) -> Self {
        Self {
            id: &result.id,
            source: &result.source,
            europe_pmc_id: format!("{}/{}", result.source, result.id),
            pmid: result.pmid.as_deref(),
            pmcid: result.pmcid.as_deref(),
            doi: result.doi.as_deref(),
            title: result.title.as_deref(),
            author_string: result.author_string.as_deref(),
            journal_title: result.journal_title.as_deref(),
            pub_year: result.pub_year.as_deref(),
            is_open_access: result.is_open_access.as_deref(),
            extra: &result.extra,
        }
    }
}

/// Projection of one page of Europe PMC search results.
#[derive(Serialize)]
pub struct EuropePmcSearchPageDto<'a> {
    hit_count: u64,
    next_cursor_mark: Option<&'a str>,
    results: Vec<EuropePmcResultDto<'a>>,
}

impl<'a> From<&'a EuropePmcSearchResponse> for EuropePmcSearchPageDto<'a> {
    fn from(response: &'a EuropePmcSearchResponse) -> Self {
        Self {
            hit_count: response.hit_count,
            next_cursor_mark: response.next_cursor_mark.as_deref(),
            results: response.results.iter().map(Into::into).collect(),
        }
    }
}

/// Projection of a [`EuropePmcReference`] for the Go bindings.
#[derive(Serialize)]
pub struct EuropePmcReferenceDto<'a> {
    source: Option<&'a str>,
    id: Option<&'a str>,
    citation_type: Option<&'a str>,
    title: Option<&'a str>,
    author_string: Option<&'a str>,
    journal_abbreviation: Option<&'a str>,
    pub_year: Option<&'a str>,
    volume: Option<&'a str>,
    issue: Option<&'a str>,
    page_info: Option<&'a str>,
    pmid: Option<&'a str>,
    doi: Option<&'a str>,
    extra: &'a serde_json::Map<String, serde_json::Value>,
}

impl<'a> From<&'a EuropePmcReference> for EuropePmcReferenceDto<'a> {
    fn from(reference: &'a EuropePmcReference) -> Self {
        Self {
            source: reference.source.as_deref(),
            id: reference.id.as_deref(),
            citation_type: reference.citation_type.as_deref(),
            title: reference.title.as_deref(),
            author_string: reference.author_string.as_deref(),
            journal_abbreviation: reference.journal_abbreviation.as_deref(),
            pub_year: reference.pub_year.as_deref(),
            volume: reference.volume.as_deref(),
            issue: reference.issue.as_deref(),
            page_info: reference.page_info.as_deref(),
            pmid: reference.pmid.as_deref(),
            doi: reference.doi.as_deref(),
            extra: &reference.extra,
        }
    }
}

/// Projection of a [`EuropePmcCitation`] for the Go bindings.
#[derive(Serialize)]
pub struct EuropePmcCitationDto<'a> {
    id: Option<&'a str>,
    source: Option<&'a str>,
    citation_type: Option<&'a str>,
    title: Option<&'a str>,
    author_string: Option<&'a str>,
    journal_abbreviation: Option<&'a str>,
    pub_year: Option<&'a str>,
    volume: Option<&'a str>,
    issue: Option<&'a str>,
    page_info: Option<&'a str>,
    cited_by_count: Option<&'a str>,
    extra: &'a serde_json::Map<String, serde_json::Value>,
}

impl<'a> From<&'a EuropePmcCitation> for EuropePmcCitationDto<'a> {
    fn from(citation: &'a EuropePmcCitation) -> Self {
        Self {
            id: citation.id.as_deref(),
            source: citation.source.as_deref(),
            citation_type: citation.citation_type.as_deref(),
            title: citation.title.as_deref(),
            author_string: citation.author_string.as_deref(),
            journal_abbreviation: citation.journal_abbreviation.as_deref(),
            pub_year: citation.pub_year.as_deref(),
            volume: citation.volume.as_deref(),
            issue: citation.issue.as_deref(),
            page_info: citation.page_info.as_deref(),
            cited_by_count: citation.cited_by_count.as_deref(),
            extra: &citation.extra,
        }
    }
}

/// Projection of a [`EuropePmcDatabaseLink`] for the Go bindings.
///
/// This one carries no `extra`; the link models have no flattened remainder.
#[derive(Serialize)]
pub struct EuropePmcDatabaseLinkDto<'a> {
    db_name: Option<&'a str>,
    db_count: Option<u32>,
    info: &'a [EuropePmcDbCrossReferenceInfo],
}

impl<'a> From<&'a EuropePmcDatabaseLink> for EuropePmcDatabaseLinkDto<'a> {
    fn from(link: &'a EuropePmcDatabaseLink) -> Self {
        Self {
            db_name: link.db_name.as_deref(),
            db_count: link.db_count,
            info: &link.info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_queries_round_trip_through_json() {
        let dto: CitationQueryDto = serde_json::from_str(
            r#"{"journal":"science","year":"1991","volume":"88","first_page":"3248",
                "author_name":"mann bj","key":"Art1"}"#,
        )
        .expect("valid citation query");

        let query = pubmed_client::CitationQuery::from(dto);
        assert_eq!(query.journal, "science");
        assert_eq!(query.key, "Art1");
    }

    #[test]
    fn citation_queries_default_missing_fields_to_empty() {
        let dto: CitationQueryDto =
            serde_json::from_str(r#"{"journal":"science"}"#).expect("partial citation query");
        let query = pubmed_client::CitationQuery::from(dto);
        assert_eq!(query.journal, "science");
        assert_eq!(query.year, "");
    }

    #[test]
    fn citation_queries_reject_unknown_keys() {
        serde_json::from_str::<CitationQueryDto>(r#"{"jrnl":"science"}"#)
            .expect_err("a typo must not be silently ignored");
    }
}
