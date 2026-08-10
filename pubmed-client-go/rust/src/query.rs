//! Query-builder replay.
//!
//! Go's `SearchQuery` does not build PubMed syntax itself. It records the
//! builder calls the caller made and ships them here as a JSON operation list,
//! which is replayed against the real [`SearchQuery`]. Field tags, date
//! formatting and boolean grouping therefore have exactly one implementation
//! across every binding, and a fix to a tag in `pubmed-client` reaches Go
//! without a matching edit here.
//!
//! The call is pure: no client handle, no runtime, no network.

use std::ffi::c_char;

use serde::{Deserialize, Serialize};

use pubmed_client::pubmed::PubDate;
use pubmed_client::{ArticleType, Language, SearchQuery, SortOrder};

use crate::error::{ShimError, ShimResult};
use crate::ffi::{guard, parse_json_arg, to_json};

/// A date at year, month or day precision.
///
/// Mirrors [`PubDate`], which has no `Deserialize` impl of its own.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct DateDto {
    year: u32,
    #[serde(default)]
    month: Option<u32>,
    #[serde(default)]
    day: Option<u32>,
}

impl From<DateDto> for PubDate {
    fn from(date: DateDto) -> Self {
        match (date.month, date.day) {
            (Some(month), Some(day)) => PubDate::with_day(date.year, month, day),
            (Some(month), None) => PubDate::with_month(date.year, month),
            _ => PubDate::new(date.year),
        }
    }
}

/// One recorded builder call.
///
/// The `op` tag matches the Rust method name, so the Go builder, this enum and
/// [`SearchQuery`] all read the same.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum QueryOp {
    // --- terms ---
    Query {
        value: String,
    },
    Terms {
        values: Vec<String>,
    },
    TitleContains {
        value: String,
    },
    AbstractContains {
        value: String,
    },
    TitleOrAbstract {
        value: String,
    },

    // --- people and places ---
    Author {
        value: String,
    },
    FirstAuthor {
        value: String,
    },
    LastAuthor {
        value: String,
    },
    Affiliation {
        value: String,
    },
    Orcid {
        value: String,
    },

    // --- source ---
    Journal {
        value: String,
    },
    JournalAbbreviation {
        value: String,
    },
    GrantNumber {
        value: String,
    },
    Isbn {
        value: String,
    },
    Issn {
        value: String,
    },

    // --- MeSH ---
    MeshTerm {
        value: String,
    },
    MeshTerms {
        values: Vec<String>,
    },
    MeshMajorTopic {
        value: String,
    },
    MeshSubheading {
        value: String,
    },
    OrganismMesh {
        value: String,
    },
    AgeGroup {
        value: String,
    },

    // --- flags ---
    HumanStudiesOnly,
    AnimalStudiesOnly,
    FreeFullTextOnly,
    FullTextOnly,
    PmcOnly,
    HasAbstract,

    // --- classification ---
    ArticleType {
        value: String,
    },
    ArticleTypes {
        values: Vec<String>,
    },
    Language {
        value: String,
    },
    CustomFilter {
        value: String,
    },

    // --- dates ---
    PublishedInYear {
        year: u32,
    },
    DateRange {
        start: u32,
        end: Option<u32>,
    },
    PublishedBetween {
        start: DateDto,
        end: Option<DateDto>,
    },
    PublishedAfter {
        date: DateDto,
    },
    PublishedBefore {
        date: DateDto,
    },
    EntryDateBetween {
        start: DateDto,
        end: Option<DateDto>,
    },
    ModificationDateBetween {
        start: DateDto,
        end: Option<DateDto>,
    },

    // --- boolean composition ---
    And {
        ops: Vec<QueryOp>,
    },
    Or {
        ops: Vec<QueryOp>,
    },
    Exclude {
        ops: Vec<QueryOp>,
    },
    Negate,
    Group,

    // --- execution hints ---
    Limit {
        value: usize,
    },
    Sort {
        value: String,
    },
    Optimize,
}

/// The whole request: an operation list plus what to do with the result.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryRequest {
    ops: Vec<QueryOp>,
    /// Run `SearchQuery::validate` before returning. Go sets this for
    /// `Validate()` and leaves it off for `Build()`, which mirrors the Rust API
    /// where building never validates.
    #[serde(default)]
    validate: bool,
}

/// What Go gets back.
#[derive(Debug, Serialize, Deserialize)]
struct QueryResponse {
    /// The assembled PubMed query string.
    query: String,
    /// The limit recorded on the builder, or the `SearchQuery` default.
    limit: usize,
    /// The recorded sort order as its API parameter name, if any.
    sort: Option<String>,
}

/// The API parameter name for a sort order.
///
/// `SortOrder::as_api_param` is crate-private in `pubmed-client`, so the
/// mapping is repeated here. It is covered by a test that round-trips every
/// name back through `from_str_insensitive`.
fn sort_name(sort: &SortOrder) -> &'static str {
    match sort {
        SortOrder::Relevance => "relevance",
        SortOrder::PublicationDate => "pub_date",
        SortOrder::FirstAuthor => "author",
        SortOrder::JournalName => "journal",
    }
}

/// Parse a sort order, reporting an unknown name as an invalid argument.
pub fn parse_sort(value: &str) -> ShimResult<SortOrder> {
    SortOrder::from_str_insensitive(value).map_err(ShimError::invalid_argument)
}

/// Replay `ops` onto `query`.
fn apply(mut query: SearchQuery, ops: Vec<QueryOp>) -> ShimResult<SearchQuery> {
    for op in ops {
        query = match op {
            QueryOp::Query { value } => query.query(value),
            QueryOp::Terms { values } => query.terms(&values),
            QueryOp::TitleContains { value } => query.title_contains(value),
            QueryOp::AbstractContains { value } => query.abstract_contains(value),
            QueryOp::TitleOrAbstract { value } => query.title_or_abstract(value),

            QueryOp::Author { value } => query.author(value),
            QueryOp::FirstAuthor { value } => query.first_author(value),
            QueryOp::LastAuthor { value } => query.last_author(value),
            QueryOp::Affiliation { value } => query.affiliation(value),
            QueryOp::Orcid { value } => query.orcid(value),

            QueryOp::Journal { value } => query.journal(value),
            QueryOp::JournalAbbreviation { value } => query.journal_abbreviation(value),
            QueryOp::GrantNumber { value } => query.grant_number(value),
            QueryOp::Isbn { value } => query.isbn(value),
            QueryOp::Issn { value } => query.issn(value),

            QueryOp::MeshTerm { value } => query.mesh_term(value),
            QueryOp::MeshTerms { values } => query.mesh_terms(&values),
            QueryOp::MeshMajorTopic { value } => query.mesh_major_topic(value),
            QueryOp::MeshSubheading { value } => query.mesh_subheading(value),
            QueryOp::OrganismMesh { value } => query.organism_mesh(value),
            QueryOp::AgeGroup { value } => query.age_group(value),

            QueryOp::HumanStudiesOnly => query.human_studies_only(),
            QueryOp::AnimalStudiesOnly => query.animal_studies_only(),
            QueryOp::FreeFullTextOnly => query.free_full_text_only(),
            QueryOp::FullTextOnly => query.full_text_only(),
            QueryOp::PmcOnly => query.pmc_only(),
            QueryOp::HasAbstract => query.has_abstract(),

            QueryOp::ArticleType { value } => query.article_type(parse_article_type(&value)?),
            QueryOp::ArticleTypes { values } => {
                let types = values
                    .iter()
                    .map(|value| parse_article_type(value))
                    .collect::<ShimResult<Vec<ArticleType>>>()?;
                query.article_types(&types)
            }
            // Unrecognised languages fall back to `Language::Other`, so this
            // never fails.
            QueryOp::Language { value } => query.language(Language::from_str_insensitive(&value)),
            QueryOp::CustomFilter { value } => query.custom_filter(value),

            QueryOp::PublishedInYear { year } => query.published_in_year(year),
            QueryOp::DateRange { start, end } => query.date_range(start, end),
            QueryOp::PublishedBetween { start, end } => {
                query.published_between(PubDate::from(start), end.map(PubDate::from))
            }
            QueryOp::PublishedAfter { date } => query.published_after(PubDate::from(date)),
            QueryOp::PublishedBefore { date } => query.published_before(PubDate::from(date)),
            QueryOp::EntryDateBetween { start, end } => {
                query.entry_date_between(PubDate::from(start), end.map(PubDate::from))
            }
            QueryOp::ModificationDateBetween { start, end } => {
                query.modification_date_between(PubDate::from(start), end.map(PubDate::from))
            }

            QueryOp::And { ops } => query.and(apply(SearchQuery::new(), ops)?),
            QueryOp::Or { ops } => query.or(apply(SearchQuery::new(), ops)?),
            QueryOp::Exclude { ops } => query.exclude(apply(SearchQuery::new(), ops)?),
            QueryOp::Negate => query.negate(),
            QueryOp::Group => query.group(),

            QueryOp::Limit { value } => query.limit(value),
            QueryOp::Sort { value } => query.sort(parse_sort(&value)?),
            QueryOp::Optimize => query.optimize(),
        };
    }
    Ok(query)
}

/// Parse an article type, reporting an unknown name as an invalid argument.
fn parse_article_type(value: &str) -> ShimResult<ArticleType> {
    ArticleType::from_str_insensitive(value).map_err(ShimError::invalid_argument)
}

/// Build a PubMed query string from a recorded operation list.
///
/// `request_json` is `{"ops": [...], "validate": bool}`; the result is
/// `{"query": string, "limit": number, "sort": string|null}`.
///
/// # Safety
///
/// `request_json` must be a valid NUL-terminated string and `out_err` must be
/// null or point to a writable `*mut c_char`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_query_build(
    request_json: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let request: QueryRequest = unsafe { parse_json_arg(request_json, "request_json") }?;
        let query = apply(SearchQuery::new(), request.ops)?;

        if request.validate {
            query.validate()?;
        }

        to_json(&QueryResponse {
            query: query.build(),
            limit: query.get_limit(),
            sort: query.get_sort().map(|sort| sort_name(sort).to_string()),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::ffi::pubmed_string_free;
    use std::ffi::{CStr, CString};
    use std::ptr;

    /// Run `pubmed_query_build` over a JSON request, returning the response.
    fn build(request: &str) -> Result<QueryResponse, String> {
        let request = CString::new(request).expect("no NUL");
        let mut err: *mut c_char = ptr::null_mut();
        let raw = unsafe { pubmed_query_build(request.as_ptr(), &mut err) };

        if raw.is_null() {
            let message = unsafe { CStr::from_ptr(err) }.to_string_lossy().to_string();
            unsafe { pubmed_string_free(err) };
            return Err(message);
        }

        let response = unsafe { CStr::from_ptr(raw) }.to_string_lossy().to_string();
        unsafe { pubmed_string_free(raw) };
        Ok(serde_json::from_str(&response).expect("a QueryResponse"))
    }

    #[test]
    fn terms_and_filters_compose_the_way_the_rust_builder_does() {
        let response = build(
            r#"{"ops":[
                {"op":"query","value":"covid-19"},
                {"op":"title_contains","value":"vaccine"},
                {"op":"published_in_year","year":2023}
            ]}"#,
        )
        .expect("a valid query");

        assert_eq!(
            response.query,
            SearchQuery::new()
                .query("covid-19")
                .title_contains("vaccine")
                .published_in_year(2023)
                .build()
        );
    }

    #[test]
    fn limit_and_sort_are_reported_back() {
        let response = build(
            r#"{"ops":[{"op":"query","value":"crispr"},{"op":"limit","value":25},
                      {"op":"sort","value":"pub_date"}]}"#,
        )
        .expect("a valid query");

        assert_eq!(response.limit, 25);
        assert_eq!(response.sort.as_deref(), Some("pub_date"));
    }

    #[test]
    fn an_unset_limit_falls_back_to_the_builder_default() {
        let response = build(r#"{"ops":[{"op":"query","value":"crispr"}]}"#).expect("valid");
        assert_eq!(response.limit, SearchQuery::new().get_limit());
        assert_eq!(response.sort, None);
    }

    #[test]
    fn dates_keep_their_precision() {
        let response = build(
            r#"{"ops":[{"op":"query","value":"covid"},
                      {"op":"published_between","start":{"year":2020,"month":3},
                       "end":{"year":2021,"month":6,"day":15}}]}"#,
        )
        .expect("valid");

        assert_eq!(
            response.query,
            SearchQuery::new()
                .query("covid")
                .published_between(
                    PubDate::with_month(2020, 3),
                    Some(PubDate::with_day(2021, 6, 15))
                )
                .build()
        );
    }

    #[test]
    fn nested_boolean_ops_are_replayed_as_subqueries() {
        let response = build(
            r#"{"ops":[{"op":"query","value":"cancer"},
                      {"op":"or","ops":[{"op":"query","value":"tumor"}]},
                      {"op":"exclude","ops":[{"op":"query","value":"review"}]}]}"#,
        )
        .expect("valid");

        assert_eq!(
            response.query,
            SearchQuery::new()
                .query("cancer")
                .or(SearchQuery::new().query("tumor"))
                .exclude(SearchQuery::new().query("review"))
                .build()
        );
    }

    #[test]
    fn article_types_are_resolved_through_the_shared_parser() {
        let response = build(
            r#"{"ops":[{"op":"query","value":"asthma"},
                      {"op":"article_types","values":["Review","RCT"]}]}"#,
        )
        .expect("valid");

        assert_eq!(
            response.query,
            SearchQuery::new()
                .query("asthma")
                .article_types(&[ArticleType::Review, ArticleType::RandomizedControlledTrial])
                .build()
        );
    }

    #[test]
    fn an_unknown_article_type_is_an_invalid_argument() {
        let message = build(r#"{"ops":[{"op":"article_type","value":"blog post"}]}"#)
            .expect_err("not a PubMed article type");
        assert!(message.contains("invalid_argument"), "{message}");
    }

    #[test]
    fn an_unknown_op_is_rejected_rather_than_ignored() {
        let message =
            build(r#"{"ops":[{"op":"summon_demon","value":"x"}]}"#).expect_err("unknown op");
        assert!(message.contains("invalid_argument"), "{message}");
    }

    #[test]
    fn validation_runs_only_when_requested() {
        // An empty query is invalid, but building one is still allowed —
        // matching the Rust API, where `build` never validates.
        build(r#"{"ops":[]}"#).expect("building an empty query is allowed");

        let message = build(r#"{"ops":[],"validate":true}"#).expect_err("empty query");
        assert!(message.contains("invalid_query"), "{message}");
    }

    #[test]
    fn sort_names_round_trip_through_the_shared_parser() {
        for sort in [
            SortOrder::Relevance,
            SortOrder::PublicationDate,
            SortOrder::FirstAuthor,
            SortOrder::JournalName,
        ] {
            let name = sort_name(&sort);
            assert_eq!(
                parse_sort(name).expect("a name this crate emits must parse"),
                sort,
                "{name} did not round-trip"
            );
        }
    }

    #[test]
    fn parse_sort_rejects_nonsense() {
        assert_eq!(
            parse_sort("sideways").expect_err("not a sort order").kind,
            ErrorKind::InvalidArgument
        );
    }
}
