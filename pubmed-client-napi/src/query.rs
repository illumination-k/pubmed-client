use napi::bindgen_prelude::*;
use napi_derive::napi;
use pubmed_client::pubmed::{
    ArticleType, Language, PubDate, SearchQuery as RustSearchQuery, SortOrder,
};

// ================================================================================================
// Date Input
// ================================================================================================

/// A date at year, month or day precision
///
/// Used by the date-range methods that accept more than a bare year. Passing a
/// plain number is equivalent to `{ year }`.
///
/// @example
/// ```typescript
/// query.entryDateBetween({ year: 2023, month: 3 }, { year: 2024, month: 12, day: 31 });
/// ```
#[napi(object)]
pub struct DateInput {
    /// Four-digit year (1800-3000)
    pub year: u32,
    /// Month, 1-12
    pub month: Option<u32>,
    /// Day of month, 1-31 (ignored unless `month` is also set)
    pub day: Option<u32>,
}

// ================================================================================================
// Helper Functions
// ================================================================================================

fn validate_year(year: u32) -> Result<()> {
    pubmed_client::validate_year(year).map_err(Error::from_reason)
}

/// Convert a JS date argument into a `PubDate`, validating each component.
fn to_pub_date(date: Either<u32, DateInput>) -> Result<PubDate> {
    let (year, month, day) = match date {
        Either::A(year) => (year, None, None),
        Either::B(input) => (input.year, input.month, input.day),
    };
    validate_year(year)?;

    let Some(month) = month else {
        return Ok(PubDate::new(year));
    };
    if !(1..=12).contains(&month) {
        return Err(Error::from_reason(format!(
            "Month must be between 1 and 12, got {month}"
        )));
    }

    let Some(day) = day else {
        return Ok(PubDate::with_month(year, month));
    };
    if !(1..=31).contains(&day) {
        return Err(Error::from_reason(format!(
            "Day must be between 1 and 31, got {day}"
        )));
    }
    Ok(PubDate::with_day(year, month, day))
}

fn str_to_article_type(s: &str) -> Result<ArticleType> {
    ArticleType::from_str_insensitive(s).map_err(Error::from_reason)
}

fn str_to_sort_order(s: &str) -> Result<SortOrder> {
    SortOrder::from_str_insensitive(s).map_err(Error::from_reason)
}

fn str_to_language(s: &str) -> Language {
    Language::from_str_insensitive(s)
}

// ================================================================================================
// SearchQuery Builder
// ================================================================================================

/// Builder for constructing PubMed search queries programmatically
///
/// Provides a fluent API for building complex PubMed search queries with support for:
/// - Basic search terms
/// - Date filtering
/// - Article type and language filtering
/// - Open access filtering
/// - Boolean logic operations (AND, OR, NOT)
/// - MeSH terms and author filtering
///
/// @example
/// ```typescript
/// const query = new SearchQuery()
///   .query("covid-19")
///   .publishedInYear(2024)
///   .articleType("Clinical Trial")
///   .freeFullTextOnly();
///
/// const articles = await client.executeQuery(query);
/// ```
#[napi]
pub struct SearchQuery {
    pub(crate) inner: RustSearchQuery,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl SearchQuery {
    /// Create a new empty search query builder
    #[napi(constructor)]
    pub fn new() -> Self {
        SearchQuery {
            inner: RustSearchQuery::new(),
        }
    }

    // ============================================================================================
    // Basic Methods
    // ============================================================================================

    /// Add a search term to the query
    ///
    /// Terms are accumulated and will be space-separated in the final query.
    ///
    /// @param term - Search term string
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("covid-19")
    ///   .query("treatment");
    /// query.build(); // "covid-19 treatment"
    /// ```
    #[napi]
    pub fn query(&mut self, term: String) -> &Self {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().query(trimmed);
        }
        self
    }

    /// Add multiple search terms at once
    ///
    /// Each term is processed like query(). Empty strings are filtered out.
    ///
    /// @param terms - Array of search term strings
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .terms(["covid-19", "vaccine", "efficacy"]);
    /// query.build(); // "covid-19 vaccine efficacy"
    /// ```
    #[napi]
    pub fn terms(&mut self, terms: Vec<String>) -> &Self {
        for term in terms {
            let trimmed = term.trim();
            if !trimmed.is_empty() {
                self.inner = self.inner.clone().query(trimmed);
            }
        }
        self
    }

    /// Set the maximum number of results to return
    ///
    /// @param limit - Maximum number of results (clamped to 1-10,000 range)
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .setLimit(50);
    /// ```
    #[napi]
    pub fn set_limit(&mut self, limit: u32) -> &Self {
        // Clamp the limit to valid range instead of throwing
        let clamped_limit = limit.clamp(1, 10000) as usize;
        self.inner = self.inner.clone().limit(clamped_limit);
        self
    }

    /// Build the final PubMed query string
    ///
    /// @returns Query string for PubMed E-utilities API
    /// @throws Error if no search terms have been added
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("covid-19")
    ///   .query("treatment");
    /// query.build(); // "covid-19 treatment"
    /// ```
    #[napi]
    pub fn build(&self) -> Result<String> {
        let query_string = self.inner.build();
        if query_string.trim().is_empty() {
            return Err(Error::from_reason(
                "Cannot build query: no search terms provided",
            ));
        }
        Ok(query_string)
    }

    /// Get the limit for this query
    ///
    /// @returns Maximum number of results (default: 20)
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery().query("cancer").limit(100);
    /// query.getLimit(); // 100
    /// ```
    #[napi(getter)]
    pub fn get_limit(&self) -> u32 {
        self.inner.get_limit() as u32
    }

    // ============================================================================================
    // Date Filtering Methods
    // ============================================================================================

    /// Filter to articles published in a specific year
    ///
    /// @param year - Year to filter by (must be between 1800 and 3000)
    /// @returns Self for method chaining
    /// @throws Error if year is outside valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("covid-19")
    ///   .publishedInYear(2024);
    /// ```
    #[napi]
    pub fn published_in_year(&mut self, year: u32) -> Result<&Self> {
        validate_year(year)?;
        self.inner = self.inner.clone().published_in_year(year);
        Ok(self)
    }

    /// Filter by publication date range
    ///
    /// @param startYear - Start year (inclusive)
    /// @param endYear - End year (inclusive, optional)
    /// @returns Self for method chaining
    /// @throws Error if years are outside valid range
    ///
    /// @example
    /// ```typescript
    /// // Filter to 2020-2024
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .publishedBetween(2020, 2024);
    ///
    /// // Filter from 2020 onwards
    /// const query2 = new SearchQuery()
    ///   .query("treatment")
    ///   .publishedBetween(2020);
    /// ```
    #[napi]
    pub fn published_between(&mut self, start_year: u32, end_year: Option<u32>) -> Result<&Self> {
        validate_year(start_year)?;

        if let Some(end) = end_year {
            validate_year(end)?;
            if start_year > end {
                return Err(Error::from_reason(format!(
                    "Start year ({}) must be <= end year ({})",
                    start_year, end
                )));
            }
        }

        self.inner = self.inner.clone().published_between(start_year, end_year);
        Ok(self)
    }

    /// Filter to articles published after a specific year
    ///
    /// @param year - Year after which articles were published
    /// @returns Self for method chaining
    /// @throws Error if year is outside valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("crispr")
    ///   .publishedAfter(2020);
    /// ```
    #[napi]
    pub fn published_after(&mut self, year: u32) -> Result<&Self> {
        validate_year(year)?;
        self.inner = self.inner.clone().published_after(year);
        Ok(self)
    }

    /// Filter to articles published before a specific year
    ///
    /// @param year - Year before which articles were published
    /// @returns Self for method chaining
    /// @throws Error if year is outside valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genome")
    ///   .publishedBefore(2020);
    /// ```
    #[napi]
    pub fn published_before(&mut self, year: u32) -> Result<&Self> {
        validate_year(year)?;
        self.inner = self.inner.clone().published_before(year);
        Ok(self)
    }

    /// Filter by publication date range (`[pdat]`)
    ///
    /// Unlike publishedBetween(), an omitted end year is left open-ended.
    ///
    /// @param startYear - Start year (inclusive)
    /// @param endYear - End year (inclusive, optional)
    /// @returns Self for method chaining
    /// @throws Error if years are outside the valid range or reversed
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("immunotherapy")
    ///   .dateRange(2020, 2023);
    /// ```
    #[napi]
    pub fn date_range(&mut self, start_year: u32, end_year: Option<u32>) -> Result<&Self> {
        validate_year(start_year)?;

        if let Some(end) = end_year {
            validate_year(end)?;
            if start_year > end {
                return Err(Error::from_reason(format!(
                    "Start year ({start_year}) must be <= end year ({end})"
                )));
            }
        }

        self.inner = self.inner.clone().date_range(start_year, end_year);
        Ok(self)
    }

    /// Filter by Entrez entry date (`[edat]`) — when the record was added to PubMed
    ///
    /// @param start - Start date: a year, or `{ year, month?, day? }`
    /// @param end - End date (optional, same format). Omit for an open-ended range.
    /// @returns Self for method chaining
    /// @throws Error if a date component is outside its valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("recent discoveries")
    ///   .entryDateBetween(2023, 2024);
    ///
    /// const precise = new SearchQuery()
    ///   .query("outbreak")
    ///   .entryDateBetween({ year: 2023, month: 3 }, { year: 2023, month: 12, day: 31 });
    /// ```
    #[napi]
    pub fn entry_date_between(
        &mut self,
        start: Either<u32, DateInput>,
        end: Option<Either<u32, DateInput>>,
    ) -> Result<&Self> {
        let start = to_pub_date(start)?;
        let end = end.map(to_pub_date).transpose()?;
        self.inner = self.inner.clone().entry_date_between(start, end);
        Ok(self)
    }

    /// Filter by record modification date (`[mdat]`) — when the record was last updated
    ///
    /// @param start - Start date: a year, or `{ year, month?, day? }`
    /// @param end - End date (optional, same format). Omit for an open-ended range.
    /// @returns Self for method chaining
    /// @throws Error if a date component is outside its valid range
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("updated articles")
    ///   .modificationDateBetween(2023);
    /// ```
    #[napi]
    pub fn modification_date_between(
        &mut self,
        start: Either<u32, DateInput>,
        end: Option<Either<u32, DateInput>>,
    ) -> Result<&Self> {
        let start = to_pub_date(start)?;
        let end = end.map(to_pub_date).transpose()?;
        self.inner = self.inner.clone().modification_date_between(start, end);
        Ok(self)
    }

    // ============================================================================================
    // Article Type and Language Filtering Methods
    // ============================================================================================

    /// Filter by a single article type
    ///
    /// @param typeName - Article type (case-insensitive)
    ///   Supported types: "Clinical Trial", "Review", "Systematic Review",
    ///   "Meta-Analysis", "Case Reports", "Randomized Controlled Trial" (or "RCT"),
    ///   "Observational Study"
    /// @returns Self for method chaining
    /// @throws Error if article type is not recognized
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .articleType("Clinical Trial");
    /// ```
    #[napi]
    pub fn article_type(&mut self, type_name: String) -> Result<&Self> {
        let article_type = str_to_article_type(&type_name)?;
        self.inner = self.inner.clone().article_type(article_type);
        Ok(self)
    }

    /// Filter by multiple article types (OR logic)
    ///
    /// @param types - Array of article type names (case-insensitive)
    /// @returns Self for method chaining
    /// @throws Error if any article type is not recognized
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("treatment")
    ///   .articleTypes(["RCT", "Meta-Analysis"]);
    /// ```
    #[napi]
    pub fn article_types(&mut self, types: Vec<String>) -> Result<&Self> {
        if types.is_empty() {
            return Ok(self);
        }

        let article_types: std::result::Result<Vec<ArticleType>, Error> =
            types.iter().map(|s| str_to_article_type(s)).collect();

        let article_types = article_types?;
        self.inner = self.inner.clone().article_types(&article_types);
        Ok(self)
    }

    /// Filter by language
    ///
    /// @param lang - Language name (case-insensitive)
    ///   Supported: "English", "Japanese", "German", "French", "Spanish", etc.
    ///   Unknown languages are passed through as custom values.
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .language("English");
    /// ```
    #[napi]
    pub fn language(&mut self, lang: String) -> &Self {
        let language = str_to_language(&lang);
        self.inner = self.inner.clone().language(language);
        self
    }

    // ============================================================================================
    // Open Access Filtering Methods
    // ============================================================================================

    /// Filter to articles with free full text (open access)
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .freeFullTextOnly();
    /// ```
    #[napi]
    pub fn free_full_text_only(&mut self) -> &Self {
        self.inner = self.inner.clone().free_full_text_only();
        self
    }

    /// Filter to articles with full text links (including subscription-based)
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("diabetes")
    ///   .fullTextOnly();
    /// ```
    #[napi]
    pub fn full_text_only(&mut self) -> &Self {
        self.inner = self.inner.clone().full_text_only();
        self
    }

    /// Filter to articles with PMC full text
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genomics")
    ///   .pmcOnly();
    /// ```
    #[napi]
    pub fn pmc_only(&mut self) -> &Self {
        self.inner = self.inner.clone().pmc_only();
        self
    }

    /// Filter to articles that have abstracts
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genetics")
    ///   .hasAbstract();
    /// ```
    #[napi]
    pub fn has_abstract(&mut self) -> &Self {
        self.inner = self.inner.clone().has_abstract();
        self
    }

    // ============================================================================================
    // Field-Specific Search Methods
    // ============================================================================================

    /// Search in article titles only
    ///
    /// @param text - Title text to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .titleContains("machine learning");
    /// ```
    #[napi]
    pub fn title_contains(&mut self, text: String) -> &Self {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().title_contains(trimmed);
        }
        self
    }

    /// Search in article abstracts only
    ///
    /// @param text - Abstract text to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .abstractContains("neural networks");
    /// ```
    #[napi]
    pub fn abstract_contains(&mut self, text: String) -> &Self {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().abstract_contains(trimmed);
        }
        self
    }

    /// Search in both title and abstract
    ///
    /// @param text - Text to search for in title or abstract
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .titleOrAbstract("CRISPR gene editing");
    /// ```
    #[napi]
    pub fn title_or_abstract(&mut self, text: String) -> &Self {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().title_or_abstract(trimmed);
        }
        self
    }

    /// Filter by journal name
    ///
    /// @param name - Journal name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .journal("Nature");
    /// ```
    #[napi]
    pub fn journal(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().journal(trimmed);
        }
        self
    }

    /// Filter by grant number
    ///
    /// @param grantNumber - Grant number to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .grantNumber("R01AI123456");
    /// ```
    #[napi]
    pub fn grant_number(&mut self, grant_number: String) -> &Self {
        let trimmed = grant_number.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().grant_number(trimmed);
        }
        self
    }

    /// Filter by journal abbreviation
    ///
    /// Uses the same `[ta]` field as journal(); pass the NLM title abbreviation.
    ///
    /// @param abbreviation - Journal abbreviation (e.g. "Nat Med")
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("stem cells")
    ///   .journalAbbreviation("Nat Med");
    /// ```
    #[napi]
    pub fn journal_abbreviation(&mut self, abbreviation: String) -> &Self {
        let trimmed = abbreviation.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().journal_abbreviation(trimmed);
        }
        self
    }

    /// Filter by ISBN
    ///
    /// @param isbn - ISBN to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery().isbn("978-0123456789");
    /// ```
    #[napi]
    pub fn isbn(&mut self, isbn: String) -> &Self {
        let trimmed = isbn.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().isbn(trimmed);
        }
        self
    }

    /// Filter by ISSN
    ///
    /// @param issn - ISSN to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery().issn("1234-5678");
    /// ```
    #[napi]
    pub fn issn(&mut self, issn: String) -> &Self {
        let trimmed = issn.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().issn(trimmed);
        }
        self
    }

    // ============================================================================================
    // Advanced Search Methods (MeSH, Author, etc.)
    // ============================================================================================

    /// Filter by MeSH term
    ///
    /// @param term - MeSH term to filter by
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshTerm("Neoplasms");
    /// ```
    #[napi]
    pub fn mesh_term(&mut self, term: String) -> &Self {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().mesh_term(trimmed);
        }
        self
    }

    /// Filter by MeSH major topic
    ///
    /// @param term - MeSH term to filter by as a major topic
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshMajorTopic("Diabetes Mellitus, Type 2");
    /// ```
    #[napi]
    pub fn mesh_major_topic(&mut self, term: String) -> &Self {
        let trimmed = term.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().mesh_major_topic(trimmed);
        }
        self
    }

    /// Filter by multiple MeSH terms
    ///
    /// @param terms - Array of MeSH terms to filter by
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshTerms(["Neoplasms", "Antineoplastic Agents"]);
    /// ```
    #[napi]
    pub fn mesh_terms(&mut self, terms: Vec<String>) -> &Self {
        let valid_terms: Vec<&str> = terms
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !valid_terms.is_empty() {
            self.inner = self.inner.clone().mesh_terms(&valid_terms);
        }
        self
    }

    /// Filter by MeSH subheading
    ///
    /// @param subheading - MeSH subheading to filter by
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .meshTerm("Diabetes Mellitus")
    ///   .meshSubheading("drug therapy");
    /// ```
    #[napi]
    pub fn mesh_subheading(&mut self, subheading: String) -> &Self {
        let trimmed = subheading.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().mesh_subheading(trimmed);
        }
        self
    }

    /// Filter by author name
    ///
    /// @param name - Author name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("machine learning")
    ///   .author("Williams K");
    /// ```
    #[napi]
    pub fn author(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().author(trimmed);
        }
        self
    }

    /// Filter by first author
    ///
    /// @param name - First author name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .firstAuthor("Smith J");
    /// ```
    #[napi]
    pub fn first_author(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().first_author(trimmed);
        }
        self
    }

    /// Filter by last author
    ///
    /// @param name - Last author name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("genomics")
    ///   .lastAuthor("Johnson M");
    /// ```
    #[napi]
    pub fn last_author(&mut self, name: String) -> &Self {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().last_author(trimmed);
        }
        self
    }

    /// Filter by institution/affiliation
    ///
    /// @param institution - Institution name to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cardiology")
    ///   .affiliation("Harvard Medical School");
    /// ```
    #[napi]
    pub fn affiliation(&mut self, institution: String) -> &Self {
        let trimmed = institution.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().affiliation(trimmed);
        }
        self
    }

    /// Filter by ORCID identifier
    ///
    /// @param orcidId - ORCID identifier to search for
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .orcid("0000-0001-2345-6789");
    /// ```
    #[napi]
    pub fn orcid(&mut self, orcid_id: String) -> &Self {
        let trimmed = orcid_id.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().orcid(trimmed);
        }
        self
    }

    /// Filter to human studies only
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("drug treatment")
    ///   .humanStudiesOnly();
    /// ```
    #[napi]
    pub fn human_studies_only(&mut self) -> &Self {
        self.inner = self.inner.clone().human_studies_only();
        self
    }

    /// Filter to animal studies only
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("preclinical research")
    ///   .animalStudiesOnly();
    /// ```
    #[napi]
    pub fn animal_studies_only(&mut self) -> &Self {
        self.inner = self.inner.clone().animal_studies_only();
        self
    }

    /// Filter by age group
    ///
    /// @param ageGroup - Age group to filter by (e.g., "Child", "Adult", "Aged")
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("pediatric medicine")
    ///   .ageGroup("Child");
    /// ```
    #[napi]
    pub fn age_group(&mut self, age_group: String) -> &Self {
        let trimmed = age_group.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().age_group(trimmed);
        }
        self
    }

    /// Filter by organism MeSH term
    ///
    /// @param organism - Scientific or common organism name (e.g. "Mus musculus", "Mice")
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("gene expression")
    ///   .organismMesh("Mus musculus");
    /// ```
    #[napi]
    pub fn organism_mesh(&mut self, organism: String) -> &Self {
        let trimmed = organism.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().organism_mesh(trimmed);
        }
        self
    }

    /// Add a custom filter
    ///
    /// @param filter - Custom filter string in PubMed syntax
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("research")
    ///   .customFilter("humans[mh]");
    /// ```
    #[napi]
    pub fn custom_filter(&mut self, filter: String) -> &Self {
        let trimmed = filter.trim();
        if !trimmed.is_empty() {
            self.inner = self.inner.clone().custom_filter(trimmed);
        }
        self
    }

    // ============================================================================================
    // Boolean Logic Methods
    // ============================================================================================

    /// Combine this query with another using AND logic
    ///
    /// @param other - Another SearchQuery to combine with
    /// @returns New SearchQuery with combined logic
    ///
    /// @example
    /// ```typescript
    /// const q1 = new SearchQuery().query("covid-19");
    /// const q2 = new SearchQuery().query("vaccine");
    /// const combined = q1.and(q2);
    /// combined.build(); // "(covid-19) AND (vaccine)"
    /// ```
    #[napi]
    pub fn and(&self, other: &SearchQuery) -> SearchQuery {
        let combined = self.inner.clone().and(other.inner.clone());
        SearchQuery { inner: combined }
    }

    /// Combine this query with another using OR logic
    ///
    /// @param other - Another SearchQuery to combine with
    /// @returns New SearchQuery with combined logic
    ///
    /// @example
    /// ```typescript
    /// const q1 = new SearchQuery().query("diabetes");
    /// const q2 = new SearchQuery().query("hypertension");
    /// const combined = q1.or(q2);
    /// combined.build(); // "(diabetes) OR (hypertension)"
    /// ```
    #[napi]
    pub fn or(&self, other: &SearchQuery) -> SearchQuery {
        let combined = self.inner.clone().or(other.inner.clone());
        SearchQuery { inner: combined }
    }

    /// Negate this query using NOT logic
    ///
    /// @returns New SearchQuery with NOT logic
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery().query("cancer").negate();
    /// query.build(); // "NOT (cancer)"
    /// ```
    #[napi]
    pub fn negate(&self) -> SearchQuery {
        let negated = self.inner.clone().negate();
        SearchQuery { inner: negated }
    }

    /// Exclude articles matching the given query
    ///
    /// @param excluded - SearchQuery representing articles to exclude
    /// @returns New SearchQuery with exclusion logic
    ///
    /// @example
    /// ```typescript
    /// const base = new SearchQuery().query("cancer treatment");
    /// const exclude = new SearchQuery().query("animal studies");
    /// const filtered = base.exclude(exclude);
    /// filtered.build(); // "(cancer treatment) NOT (animal studies)"
    /// ```
    #[napi]
    pub fn exclude(&self, excluded: &SearchQuery) -> SearchQuery {
        let filtered = self.inner.clone().exclude(excluded.inner.clone());
        SearchQuery { inner: filtered }
    }

    /// Add parentheses around the current query for grouping
    ///
    /// @returns New SearchQuery wrapped in parentheses
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .or(new SearchQuery().query("tumor"))
    ///   .group();
    /// query.build(); // "((cancer) OR (tumor))"
    /// ```
    #[napi]
    pub fn group(&self) -> SearchQuery {
        let grouped = self.inner.clone().group();
        SearchQuery { inner: grouped }
    }

    // ============================================================================================
    // Sort Methods
    // ============================================================================================

    /// Set the sort order for search results
    ///
    /// @param sortOrder - Sort order (case-insensitive)
    ///   Supported: "relevance", "pub_date", "author", "journal"
    /// @returns Self for method chaining
    /// @throws Error if sort order is not recognized
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("cancer")
    ///   .sort("pub_date");
    /// ```
    #[napi]
    pub fn sort(&mut self, sort_order: String) -> Result<&Self> {
        let sort = str_to_sort_order(&sort_order)?;
        self.inner = self.inner.clone().sort(sort);
        Ok(self)
    }

    // ============================================================================================
    // Validation and Optimization
    // ============================================================================================

    /// Validate the query structure and parameters
    ///
    /// Checks that the query is non-empty, the limit is in range, the built
    /// string is under 4,000 characters, and parentheses are balanced.
    ///
    /// @returns Self for method chaining
    /// @throws Error describing the first problem found
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery().query("covid-19");
    /// query.validate(); // does not throw
    ///
    /// new SearchQuery().validate(); // throws: "Query cannot be empty"
    /// ```
    #[napi]
    pub fn validate(&self) -> Result<&Self> {
        self.inner
            .validate()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(self)
    }

    /// Optimize the query in place
    ///
    /// Sorts and de-duplicates terms and filters and drops empty ones. Note that
    /// this reorders the built query string.
    ///
    /// @returns Self for method chaining
    ///
    /// @example
    /// ```typescript
    /// const query = new SearchQuery()
    ///   .query("covid-19")
    ///   .query("covid-19")
    ///   .optimize();
    /// query.build(); // "covid-19"
    /// ```
    #[napi]
    pub fn optimize(&mut self) -> &Self {
        self.inner = self.inner.clone().optimize();
        self
    }
}
