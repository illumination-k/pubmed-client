//! Search tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use tracing::info;

use super::common::{internal_error, text_result};
use pubmed_client::{ArticleType, SearchQuery, SortOrder};

/// Study type filter for PubMed searches
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StudyType {
    /// Randomized controlled trials
    RandomizedControlledTrial,
    /// Clinical trials
    ClinicalTrial,
    /// Meta-analysis
    MetaAnalysis,
    /// Systematic reviews
    SystematicReview,
    /// Review articles
    Review,
    /// Observational studies
    ObservationalStudy,
    /// Case reports
    CaseReport,
}

impl StudyType {
    fn to_article_type(&self) -> ArticleType {
        match self {
            StudyType::RandomizedControlledTrial => ArticleType::RandomizedControlledTrial,
            StudyType::ClinicalTrial => ArticleType::ClinicalTrial,
            StudyType::MetaAnalysis => ArticleType::MetaAnalysis,
            StudyType::SystematicReview => ArticleType::SystematicReview,
            StudyType::Review => ArticleType::Review,
            StudyType::ObservationalStudy => ArticleType::ObservationalStudy,
            StudyType::CaseReport => ArticleType::CaseReport,
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            StudyType::RandomizedControlledTrial => "RCT",
            StudyType::ClinicalTrial => "Clinical Trial",
            StudyType::MetaAnalysis => "Meta-Analysis",
            StudyType::SystematicReview => "Systematic Review",
            StudyType::Review => "Review",
            StudyType::ObservationalStudy => "Observational Study",
            StudyType::CaseReport => "Case Report",
        }
    }
}

/// Text availability filter for PubMed searches
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextAvailability {
    /// Free full text available (includes PMC, Bookshelf, and publishers' websites)
    FreeFullText,
    /// Full text available (any full text link)
    FullText,
    /// PMC full text available
    PmcOnly,
}

impl TextAvailability {
    fn display_name(&self) -> &'static str {
        match self {
            TextAvailability::FreeFullText => "Free Full Text",
            TextAvailability::FullText => "Full Text",
            TextAvailability::PmcOnly => "PMC Only",
        }
    }
}

/// Sort order for search results
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchSortOrder {
    /// Sort by relevance (default)
    Relevance,
    /// Sort by publication date (newest first)
    PublicationDate,
    /// Sort by first author name (alphabetical)
    FirstAuthor,
    /// Sort by journal name (alphabetical)
    JournalName,
}

impl SearchSortOrder {
    fn to_sort_order(&self) -> SortOrder {
        match self {
            SearchSortOrder::Relevance => SortOrder::Relevance,
            SearchSortOrder::PublicationDate => SortOrder::PublicationDate,
            SearchSortOrder::FirstAuthor => SortOrder::FirstAuthor,
            SearchSortOrder::JournalName => SortOrder::JournalName,
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            SearchSortOrder::Relevance => "Relevance",
            SearchSortOrder::PublicationDate => "Publication Date",
            SearchSortOrder::FirstAuthor => "First Author",
            SearchSortOrder::JournalName => "Journal Name",
        }
    }
}

/// Search request parameters with filters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchRequest {
    #[schemars(description = "Search query (e.g., 'COVID-19', 'cancer[ti]')")]
    pub query: String,

    #[schemars(description = "Maximum number of results (default: 10, max: 100)")]
    pub max_results: Option<usize>,

    #[schemars(
        description = "Filter by study type (e.g., randomized_controlled_trial, meta_analysis)"
    )]
    pub study_type: Option<StudyType>,

    #[schemars(description = "Filter by text availability (free_full_text, full_text, pmc_only)")]
    pub text_availability: Option<TextAvailability>,

    #[schemars(description = "Start year for date range filter (inclusive)")]
    pub start_year: Option<u32>,

    #[schemars(description = "End year for date range filter (inclusive, optional)")]
    pub end_year: Option<u32>,

    #[schemars(description = "Include abstract preview in results (default: true)")]
    pub include_abstract: Option<bool>,

    #[schemars(
        description = "Sort order for results (relevance, publication_date, first_author, journal_name)"
    )]
    pub sort: Option<SearchSortOrder>,
}

/// Maximum number of characters shown in an abstract preview.
const ABSTRACT_PREVIEW_CHARS: usize = 200;

/// Build a truncated preview of an abstract, safe for multi-byte UTF-8 text.
///
/// Truncation happens at a character boundary rather than a fixed byte index;
/// a byte-index slice would panic when the cut point lands in the middle of a
/// multi-byte character (e.g. Greek letters, accents, en/em dashes that appear
/// routinely in PubMed abstracts).
fn abstract_preview(abstract_text: &str) -> String {
    match abstract_text.char_indices().nth(ABSTRACT_PREVIEW_CHARS) {
        Some((idx, _)) => format!("{}...", &abstract_text[..idx]),
        None => abstract_text.to_string(),
    }
}

/// Search PubMed for articles with advanced filtering
pub async fn search_pubmed(
    server: &super::PubMedServer,
    Parameters(params): Parameters<SearchRequest>,
) -> Result<CallToolResult, ErrorData> {
    let max = params.max_results.unwrap_or(10).min(100);
    let include_abstract = params.include_abstract.unwrap_or(true);

    // Build search query with filters
    let mut search_query = SearchQuery::new().query(&params.query);

    if let Some(ref study_type) = params.study_type {
        let article_type = study_type.to_article_type();
        search_query = search_query.article_type(article_type);
    }

    if let Some(ref text_availability) = params.text_availability {
        search_query = match text_availability {
            TextAvailability::FreeFullText => search_query.free_full_text_only(),
            TextAvailability::FullText => search_query.full_text_only(),
            TextAvailability::PmcOnly => search_query.pmc_only(),
        };
    }

    if let Some(start_year) = params.start_year {
        search_query = search_query.date_range(start_year, params.end_year);
    }

    if let Some(ref sort_order) = params.sort {
        search_query = search_query.sort(sort_order.to_sort_order());
    }

    let query_string = search_query.build();

    info!(
        query = %query_string,
        max_results = max,
        study_type = ?params.study_type,
        text_availability = ?params.text_availability,
        start_year = ?params.start_year,
        end_year = ?params.end_year,
        sort = ?params.sort,
        "Searching PubMed with filters"
    );

    let articles = server
        .client
        .pubmed
        .search_and_fetch(&query_string, max, search_query.get_sort())
        .await
        .map_err(|e| internal_error(format!("Search failed: {}", e)))?;

    let mut result = String::new();

    // Add filter information to the result
    let mut filters_applied = Vec::new();
    if let Some(ref study_type) = params.study_type {
        filters_applied.push(study_type.display_name().to_string());
    }
    if let Some(ref text_availability) = params.text_availability {
        filters_applied.push(text_availability.display_name().to_string());
    }
    if let Some(start) = params.start_year {
        if let Some(end) = params.end_year {
            filters_applied.push(format!("Published {}-{}", start, end));
        } else {
            filters_applied.push(format!("Published after {}", start));
        }
    }
    if let Some(ref sort_order) = params.sort {
        filters_applied.push(format!("Sorted by {}", sort_order.display_name()));
    }

    if !filters_applied.is_empty() {
        result.push_str(&format!(
            "Filters applied: {}\n",
            filters_applied.join(", ")
        ));
    }

    result.push_str(&format!("Found {} articles:\n\n", articles.len()));

    for (i, article) in articles.iter().enumerate() {
        if let Some(ref pmc_id) = article.pmc_id {
            result.push_str(&format!(
                "{}. {} (PMID: {} | PMC: {})\n",
                i + 1,
                article.title,
                article.pmid,
                pmc_id
            ));
        } else {
            result.push_str(&format!(
                "{}. {} (PMID: {})\n",
                i + 1,
                article.title,
                article.pmid
            ));
        }

        if include_abstract && let Some(ref abstract_text) = article.abstract_text {
            result.push_str(&format!(
                "   Abstract: {}\n",
                abstract_preview(abstract_text)
            ));
        }
        result.push('\n');
    }

    text_result(result)
}

#[cfg(test)]
mod tests {
    use super::{ABSTRACT_PREVIEW_CHARS, abstract_preview};

    #[test]
    fn short_abstract_is_returned_unchanged() {
        let text = "A short abstract.";
        assert_eq!(abstract_preview(text), text);
    }

    #[test]
    fn long_ascii_abstract_is_truncated_with_ellipsis() {
        let text = "a".repeat(500);
        let preview = abstract_preview(&text);
        assert_eq!(
            preview,
            format!("{}...", "a".repeat(ABSTRACT_PREVIEW_CHARS))
        );
    }

    #[test]
    fn multibyte_abstract_does_not_panic_at_boundary() {
        // Each 'µ' is 2 bytes, so byte 200 falls in the middle of a character.
        // A byte-index slice would panic here; the char-boundary version must not.
        let text = "µ".repeat(300);
        let preview = abstract_preview(&text);
        assert_eq!(
            preview,
            format!("{}...", "µ".repeat(ABSTRACT_PREVIEW_CHARS))
        );
    }

    #[test]
    fn abstract_exactly_at_limit_is_not_truncated() {
        let text = "x".repeat(ABSTRACT_PREVIEW_CHARS);
        assert_eq!(abstract_preview(&text), text);
    }
}
