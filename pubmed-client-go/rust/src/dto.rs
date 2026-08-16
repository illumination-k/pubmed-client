//! Projections of `pubmed-client` types onto the JSON shapes Go decodes.
//!
//! Most `pubmed-client` types are already `Serialize` and cross the boundary
//! untouched. The types here exist for the three cases where they are not:
//! the JATS domain tree (too deeply nested to mirror in Go), tuple-shaped
//! results, and types that derive neither `Serialize` nor `Deserialize`.

use serde::{Deserialize, Serialize};

use pubmed_client::{Author, Figure, JournalMeta, PmcArticle, PubMedArticle, Reference, Section};

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
