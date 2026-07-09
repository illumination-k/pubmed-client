use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use crate::JsPromiseArticles;
use crate::client::WasmPubMedClient;
use crate::error::to_js_err;
use crate::models::JsArticle;

/// Map legacy snake_case article-type aliases to the canonical names accepted by
/// `ArticleType::from_str_insensitive`. Unknown values pass through unchanged.
fn normalize_article_type(article_type: &str) -> &str {
    match article_type {
        "clinical_trial" => "Clinical Trial",
        "meta_analysis" => "Meta-Analysis",
        "randomized_controlled_trial" => "Randomized Controlled Trial",
        "systematic_review" => "Systematic Review",
        "case_report" => "Case Reports",
        "observational_study" => "Observational Study",
        other => other,
    }
}

/// Search query builder for constructing complex PubMed queries
///
/// Provides a fluent API for building PubMed search queries with filters,
/// date ranges, article types, and boolean logic.
#[wasm_bindgen]
pub struct WasmSearchQuery {
    inner: pubmed_client::SearchQuery,
}

#[wasm_bindgen]
impl WasmSearchQuery {
    /// Create a new empty search query builder
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: pubmed_client::SearchQuery::new(),
        }
    }

    /// Set the main search query text
    pub fn query(mut self, query: &str) -> Self {
        self.inner = self.inner.query(query);
        self
    }

    /// Search in article titles only
    pub fn title(mut self, title: &str) -> Self {
        self.inner = self.inner.title_contains(title);
        self
    }

    /// Search in title and abstract
    pub fn title_abstract(mut self, text: &str) -> Self {
        self.inner = self.inner.title_or_abstract(text);
        self
    }

    /// Search by author name
    pub fn author(mut self, author: &str) -> Self {
        self.inner = self.inner.author(author);
        self
    }

    /// Search by first author name
    pub fn first_author(mut self, author: &str) -> Self {
        self.inner = self.inner.first_author(author);
        self
    }

    /// Search by journal name
    pub fn journal(mut self, journal: &str) -> Self {
        self.inner = self.inner.journal(journal);
        self
    }

    /// Search by MeSH term
    pub fn mesh_term(mut self, term: &str) -> Self {
        self.inner = self.inner.mesh_term(term);
        self
    }

    /// Search by MeSH major topic
    pub fn mesh_major_topic(mut self, term: &str) -> Self {
        self.inner = self.inner.mesh_major_topic(term);
        self
    }

    /// Filter to only free full text articles
    pub fn free_full_text_only(mut self) -> Self {
        self.inner = self.inner.free_full_text_only();
        self
    }

    /// Filter to only articles available in PMC
    pub fn pmc_only(mut self) -> Self {
        self.inner = self.inner.pmc_only();
        self
    }

    /// Filter to only articles with full text available
    pub fn full_text_only(mut self) -> Self {
        self.inner = self.inner.full_text_only();
        self
    }

    /// Filter to articles published after a given year (must be 1800–3000)
    pub fn published_after(mut self, year: u32) -> Result<Self, JsValue> {
        pubmed_client::validate_year(year).map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.published_after(year);
        Ok(self)
    }

    /// Filter to articles published in a date range (years must be 1800–3000)
    pub fn date_range(mut self, start_year: u32, end_year: Option<u32>) -> Result<Self, JsValue> {
        pubmed_client::validate_year(start_year).map_err(|e| JsValue::from_str(&e))?;
        if let Some(end) = end_year {
            pubmed_client::validate_year(end).map_err(|e| JsValue::from_str(&e))?;
            if start_year > end {
                return Err(JsValue::from_str(&format!(
                    "Start year ({}) must be <= end year ({})",
                    start_year, end
                )));
            }
        }
        self.inner = self.inner.date_range(start_year, end_year);
        Ok(self)
    }

    /// Filter by article type string (case-insensitive).
    ///
    /// Accepted values: `"Clinical Trial"`, `"Review"`, `"Systematic Review"`,
    /// `"Meta-Analysis"` / `"Meta Analysis"`, `"Case Reports"` / `"Case Report"`,
    /// `"Randomized Controlled Trial"` / `"RCT"`, `"Observational Study"`.
    /// Also accepts the legacy snake_case aliases used in previous versions.
    pub fn article_type_str(mut self, article_type: &str) -> Result<Self, JsValue> {
        let normalized = normalize_article_type(article_type);
        let at = pubmed_client::ArticleType::from_str_insensitive(normalized)
            .map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.article_type(at);
        Ok(self)
    }

    /// Filter to human-only studies
    pub fn humans_only(mut self) -> Self {
        self.inner = self.inner.human_studies_only();
        self
    }

    /// Filter by language string (case-insensitive, accepts full names and ISO 639-2 codes).
    ///
    /// Unknown values fall back to `Language::Other(s)` rather than being silently ignored.
    pub fn language_str(mut self, language: &str) -> Self {
        let lang = pubmed_client::Language::from_str_insensitive(language);
        self.inner = self.inner.language(lang);
        self
    }

    /// Set the sort order for results (case-insensitive).
    ///
    /// Accepted values: `"relevance"`, `"pub_date"` / `"publication_date"` / `"date"`,
    /// `"author"` / `"first_author"`, `"journal"` / `"journal_name"`.
    pub fn sort_str(mut self, sort: &str) -> Result<Self, JsValue> {
        let order = pubmed_client::SortOrder::from_str_insensitive(sort)
            .map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.sort(order);
        Ok(self)
    }

    /// Add multiple search terms (joined with the default AND logic)
    pub fn terms(mut self, terms: Vec<String>) -> Self {
        self.inner = self.inner.terms(&terms);
        self
    }

    /// Set the maximum number of results to retrieve
    pub fn limit(mut self, limit: usize) -> Self {
        self.inner = self.inner.limit(limit);
        self
    }

    /// Get the currently configured result limit
    pub fn get_limit(&self) -> usize {
        self.inner.get_limit()
    }

    /// Search in article abstracts only
    pub fn abstract_contains(mut self, text: &str) -> Self {
        self.inner = self.inner.abstract_contains(text);
        self
    }

    /// Filter to only articles that have an abstract
    pub fn has_abstract(mut self) -> Self {
        self.inner = self.inner.has_abstract();
        self
    }

    /// Search by journal abbreviation (e.g. "N Engl J Med")
    pub fn journal_abbreviation(mut self, abbreviation: &str) -> Self {
        self.inner = self.inner.journal_abbreviation(abbreviation);
        self
    }

    /// Search by grant number
    pub fn grant_number(mut self, grant_number: &str) -> Self {
        self.inner = self.inner.grant_number(grant_number);
        self
    }

    /// Search by ISBN
    pub fn isbn(mut self, isbn: &str) -> Self {
        self.inner = self.inner.isbn(isbn);
        self
    }

    /// Search by ISSN
    pub fn issn(mut self, issn: &str) -> Self {
        self.inner = self.inner.issn(issn);
        self
    }

    /// Search by last author name
    pub fn last_author(mut self, author: &str) -> Self {
        self.inner = self.inner.last_author(author);
        self
    }

    /// Search by author affiliation / institution
    pub fn affiliation(mut self, institution: &str) -> Self {
        self.inner = self.inner.affiliation(institution);
        self
    }

    /// Search by author ORCID identifier
    pub fn orcid(mut self, orcid_id: &str) -> Self {
        self.inner = self.inner.orcid(orcid_id);
        self
    }

    /// Search by multiple MeSH terms (OR logic)
    pub fn mesh_terms(mut self, terms: Vec<String>) -> Self {
        self.inner = self.inner.mesh_terms(&terms);
        self
    }

    /// Search by MeSH subheading
    pub fn mesh_subheading(mut self, subheading: &str) -> Self {
        self.inner = self.inner.mesh_subheading(subheading);
        self
    }

    /// Filter to animal-only studies
    pub fn animal_studies_only(mut self) -> Self {
        self.inner = self.inner.animal_studies_only();
        self
    }

    /// Filter by age group (e.g. "Adult", "Child")
    pub fn age_group(mut self, age_group: &str) -> Self {
        self.inner = self.inner.age_group(age_group);
        self
    }

    /// Filter by organism using its MeSH term
    pub fn organism_mesh(mut self, organism: &str) -> Self {
        self.inner = self.inner.organism_mesh(organism);
        self
    }

    /// Add a raw custom filter clause appended verbatim to the query
    pub fn custom_filter(mut self, filter: &str) -> Self {
        self.inner = self.inner.custom_filter(filter);
        self
    }

    /// Filter by multiple article types (OR logic, case-insensitive).
    ///
    /// Accepts the same values as `article_type_str`, including the legacy
    /// snake_case aliases. An empty array is a no-op.
    pub fn article_types_str(mut self, article_types: Vec<String>) -> Result<Self, JsValue> {
        if article_types.is_empty() {
            return Ok(self);
        }
        let mut parsed = Vec::with_capacity(article_types.len());
        for at in &article_types {
            let normalized = normalize_article_type(at);
            parsed.push(
                pubmed_client::ArticleType::from_str_insensitive(normalized)
                    .map_err(|e| JsValue::from_str(&e))?,
            );
        }
        self.inner = self.inner.article_types(&parsed);
        Ok(self)
    }

    /// Filter to a single publication year (must be 1800–3000)
    pub fn published_in_year(mut self, year: u32) -> Result<Self, JsValue> {
        pubmed_client::validate_year(year).map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.published_in_year(year);
        Ok(self)
    }

    /// Filter to a publication-year range (years must be 1800–3000)
    pub fn published_between(
        mut self,
        start_year: u32,
        end_year: Option<u32>,
    ) -> Result<Self, JsValue> {
        pubmed_client::validate_year(start_year).map_err(|e| JsValue::from_str(&e))?;
        if let Some(end) = end_year {
            pubmed_client::validate_year(end).map_err(|e| JsValue::from_str(&e))?;
            if start_year > end {
                return Err(JsValue::from_str(&format!(
                    "Start year ({}) must be <= end year ({})",
                    start_year, end
                )));
            }
        }
        self.inner = self.inner.published_between(start_year, end_year);
        Ok(self)
    }

    /// Filter to articles published before a given year (must be 1800–3000)
    pub fn published_before(mut self, year: u32) -> Result<Self, JsValue> {
        pubmed_client::validate_year(year).map_err(|e| JsValue::from_str(&e))?;
        self.inner = self.inner.published_before(year);
        Ok(self)
    }

    /// Combine this query with another using AND logic, returning a new query
    pub fn and(&self, other: &WasmSearchQuery) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().and(other.inner.clone()),
        }
    }

    /// Combine this query with another using OR logic, returning a new query
    pub fn or(&self, other: &WasmSearchQuery) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().or(other.inner.clone()),
        }
    }

    /// Negate this query using NOT logic, returning a new query
    pub fn negate(&self) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().negate(),
        }
    }

    /// Exclude articles matching another query, returning a new query
    pub fn exclude(&self, excluded: &WasmSearchQuery) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().exclude(excluded.inner.clone()),
        }
    }

    /// Wrap this query in parentheses for grouping, returning a new query
    pub fn group(&self) -> WasmSearchQuery {
        WasmSearchQuery {
            inner: self.inner.clone().group(),
        }
    }

    /// Build the query string
    pub fn build(&self) -> String {
        self.inner.build()
    }

    /// Search and fetch articles using this query
    pub fn search_and_fetch(&self, client: &WasmPubMedClient, limit: usize) -> JsPromiseArticles {
        let query_string = self.inner.build();
        let sort = self.inner.get_sort().cloned();
        let rust_client = client.client.clone();
        future_to_promise(async move {
            match rust_client
                .pubmed
                .search_and_fetch(&query_string, limit, sort.as_ref())
                .await
            {
                Ok(articles) => {
                    let js_articles: Vec<JsArticle> =
                        articles.into_iter().map(JsArticle::from).collect();
                    Ok(serde_wasm_bindgen::to_value(&js_articles)?)
                }
                Err(e) => Err(to_js_err(e)),
            }
        })
        .unchecked_into()
    }
}

impl Default for WasmSearchQuery {
    fn default() -> Self {
        Self::new()
    }
}
