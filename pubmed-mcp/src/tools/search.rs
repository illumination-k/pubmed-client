//! Search tool for PubMed MCP server

use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::info;

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

/// Search PubMed for articles with advanced filtering
pub async fn search_pubmed(
    server: &super::PubMedServer,
    Parameters(params): Parameters<SearchRequest>,
) -> Result<CallToolResult, ErrorData> {
    let max = params.max_results.unwrap_or(10).min(100);
    let include_abstract = params.include_abstract.unwrap_or(true);

    // Build search query with filters
    let mut search_query = SearchQuery::new().query(&params.query);

    // Apply study type filter
    if let Some(ref study_type) = params.study_type {
        let article_type = study_type.to_article_type();
        search_query = search_query.article_type(article_type);
    }

    // Apply text availability filter
    if let Some(ref text_availability) = params.text_availability {
        search_query = match text_availability {
            TextAvailability::FreeFullText => search_query.free_full_text_only(),
            TextAvailability::FullText => search_query.full_text_only(),
            TextAvailability::PmcOnly => search_query.pmc_only(),
        };
    }

    // Apply date range filter
    if let Some(start_year) = params.start_year {
        search_query = search_query.date_range(start_year, params.end_year);
    }

    // Apply sort order
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
        .map_err(|e| ErrorData {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Search failed: {}", e)),
            data: None,
        })?;

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
        // Format article with PMC ID if available
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

        // Add abstract preview if available and requested
        if include_abstract && let Some(ref abstract_text) = article.abstract_text {
            let preview = if abstract_text.len() > 200 {
                format!("{}...", &abstract_text[..200])
            } else {
                abstract_text.clone()
            };
            result.push_str(&format!("   Abstract: {}\n", preview));
        }
        result.push('\n');
    }

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
