//! Query builder for constructing PubMed search queries with filters

use super::{PubMedArticle, PubMedClient};
use crate::error::Result;

/// Represents a date for PubMed searches with varying precision
#[derive(Debug, Clone, PartialEq)]
pub struct PubDate {
    year: u32,
    month: Option<u32>,
    day: Option<u32>,
}

impl PubDate {
    /// Create a new PubDate with year only
    pub fn new(year: u32) -> Self {
        Self {
            year,
            month: None,
            day: None,
        }
    }

    /// Create a new PubDate with year and month
    pub fn with_month(year: u32, month: u32) -> Self {
        Self {
            year,
            month: Some(month),
            day: None,
        }
    }

    /// Create a new PubDate with year, month, and day
    pub fn with_day(year: u32, month: u32, day: u32) -> Self {
        Self {
            year,
            month: Some(month),
            day: Some(day),
        }
    }

    /// Format as PubMed date string
    pub fn to_pubmed_string(&self) -> String {
        match (self.month, self.day) {
            (Some(month), Some(day)) => format!("{}/{:02}/{:02}", self.year, month, day),
            (Some(month), None) => format!("{}/{:02}", self.year, month),
            _ => self.year.to_string(),
        }
    }
}

impl From<u32> for PubDate {
    fn from(year: u32) -> Self {
        Self::new(year)
    }
}

impl From<(u32, u32)> for PubDate {
    fn from((year, month): (u32, u32)) -> Self {
        Self::with_month(year, month)
    }
}

impl From<(u32, u32, u32)> for PubDate {
    fn from((year, month, day): (u32, u32, u32)) -> Self {
        Self::with_day(year, month, day)
    }
}

/// Article types that can be filtered in PubMed searches
#[derive(Debug, Clone, PartialEq)]
pub enum ArticleType {
    /// Clinical trials
    ClinicalTrial,
    /// Review articles
    Review,
    /// Systematic reviews
    SystematicReview,
    /// Meta-analysis
    MetaAnalysis,
    /// Case reports
    CaseReport,
    /// Randomized controlled trials
    RandomizedControlledTrial,
    /// Observational studies
    ObservationalStudy,
}

impl ArticleType {
    fn to_query_string(&self) -> &'static str {
        match self {
            ArticleType::ClinicalTrial => "Clinical Trial[pt]",
            ArticleType::Review => "Review[pt]",
            ArticleType::SystematicReview => "Systematic Review[pt]",
            ArticleType::MetaAnalysis => "Meta-Analysis[pt]",
            ArticleType::CaseReport => "Case Reports[pt]",
            ArticleType::RandomizedControlledTrial => "Randomized Controlled Trial[pt]",
            ArticleType::ObservationalStudy => "Observational Study[pt]",
        }
    }
}

/// Language options for filtering articles
#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    English,
    Japanese,
    German,
    French,
    Spanish,
    Italian,
    Chinese,
    Russian,
    Portuguese,
    Arabic,
    Dutch,
    Korean,
    Polish,
    Swedish,
    Danish,
    Norwegian,
    Finnish,
    Turkish,
    Hebrew,
    Czech,
    Hungarian,
    Greek,
    Other(String),
}

impl Language {
    fn to_query_string(&self) -> String {
        match self {
            Language::English => "English[lang]".to_string(),
            Language::Japanese => "Japanese[lang]".to_string(),
            Language::German => "German[lang]".to_string(),
            Language::French => "French[lang]".to_string(),
            Language::Spanish => "Spanish[lang]".to_string(),
            Language::Italian => "Italian[lang]".to_string(),
            Language::Chinese => "Chinese[lang]".to_string(),
            Language::Russian => "Russian[lang]".to_string(),
            Language::Portuguese => "Portuguese[lang]".to_string(),
            Language::Arabic => "Arabic[lang]".to_string(),
            Language::Dutch => "Dutch[lang]".to_string(),
            Language::Korean => "Korean[lang]".to_string(),
            Language::Polish => "Polish[lang]".to_string(),
            Language::Swedish => "Swedish[lang]".to_string(),
            Language::Danish => "Danish[lang]".to_string(),
            Language::Norwegian => "Norwegian[lang]".to_string(),
            Language::Finnish => "Finnish[lang]".to_string(),
            Language::Turkish => "Turkish[lang]".to_string(),
            Language::Hebrew => "Hebrew[lang]".to_string(),
            Language::Czech => "Czech[lang]".to_string(),
            Language::Hungarian => "Hungarian[lang]".to_string(),
            Language::Greek => "Greek[lang]".to_string(),
            Language::Other(lang) => format!("{}[lang]", lang),
        }
    }
}

/// Builder for constructing PubMed search queries
#[derive(Debug, Clone)]
pub struct SearchQuery {
    terms: Vec<String>,
    filters: Vec<String>,
    limit: Option<usize>,
}

impl SearchQuery {
    /// Create a new search query builder
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new();
    /// ```
    pub fn new() -> Self {
        Self {
            terms: Vec::new(),
            filters: Vec::new(),
            limit: None,
        }
    }

    /// Add search terms
    ///
    /// # Arguments
    ///
    /// * `terms` - Search terms to add
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("covid-19 treatment");
    /// ```
    pub fn query<S: Into<String>>(mut self, terms: S) -> Self {
        self.terms.push(terms.into());
        self
    }

    /// Search in article titles only
    ///
    /// # Arguments
    ///
    /// * `title` - Title text to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .title_contains("machine learning");
    /// ```
    pub fn title_contains<S: Into<String>>(mut self, title: S) -> Self {
        self.filters.push(format!("{}[Title]", title.into()));
        self
    }

    /// Search in article abstracts only
    ///
    /// # Arguments
    ///
    /// * `abstract_text` - Abstract text to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .abstract_contains("deep learning neural networks");
    /// ```
    pub fn abstract_contains<S: Into<String>>(mut self, abstract_text: S) -> Self {
        self.filters
            .push(format!("{}[Abstract]", abstract_text.into()));
        self
    }

    /// Search in both title and abstract
    ///
    /// # Arguments
    ///
    /// * `text` - Text to search for in title or abstract
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .title_or_abstract("CRISPR gene editing");
    /// ```
    pub fn title_or_abstract<S: Into<String>>(mut self, text: S) -> Self {
        self.filters
            .push(format!("{}[Title/Abstract]", text.into()));
        self
    }

    /// Filter by journal name
    ///
    /// # Arguments
    ///
    /// * `journal` - Journal name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer treatment")
    ///     .journal("Nature");
    /// ```
    pub fn journal<S: Into<String>>(mut self, journal: S) -> Self {
        self.filters.push(format!("{}[Journal]", journal.into()));
        self
    }

    /// Filter by journal title abbreviation
    ///
    /// # Arguments
    ///
    /// * `abbreviation` - Journal abbreviation to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("stem cells")
    ///     .journal_abbreviation("Nat Med");
    /// ```
    pub fn journal_abbreviation<S: Into<String>>(mut self, abbreviation: S) -> Self {
        self.filters.push(format!(
            "{}[Journal Title Abbreviation]",
            abbreviation.into()
        ));
        self
    }

    /// Filter by grant number
    ///
    /// # Arguments
    ///
    /// * `grant_number` - Grant number to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .grant_number("R01AI123456");
    /// ```
    pub fn grant_number<S: Into<String>>(mut self, grant_number: S) -> Self {
        self.filters
            .push(format!("{}[Grant Number]", grant_number.into()));
        self
    }

    /// Filter by ISBN
    ///
    /// # Arguments
    ///
    /// * `isbn` - ISBN to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .isbn("978-0123456789");
    /// ```
    pub fn isbn<S: Into<String>>(mut self, isbn: S) -> Self {
        self.filters.push(format!("{}[ISBN]", isbn.into()));
        self
    }

    /// Filter by ISSN
    ///
    /// # Arguments
    ///
    /// * `issn` - ISSN to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .issn("1234-5678");
    /// ```
    pub fn issn<S: Into<String>>(mut self, issn: S) -> Self {
        self.filters.push(format!("{}[ISSN]", issn.into()));
        self
    }

    /// Add multiple search terms
    ///
    /// # Arguments
    ///
    /// * `terms` - Multiple search terms
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .terms(&["covid-19", "treatment", "vaccine"]);
    /// ```
    pub fn terms<S: AsRef<str>>(mut self, terms: &[S]) -> Self {
        for term in terms {
            self.terms.push(term.as_ref().to_string());
        }
        self
    }

    /// Filter to open access articles only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .open_access_only();
    /// ```
    pub fn open_access_only(mut self) -> Self {
        self.filters.push("free full text[sb]".to_string());
        self
    }

    /// Filter to articles with free full text
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("diabetes")
    ///     .free_full_text();
    /// ```
    pub fn free_full_text(mut self) -> Self {
        self.filters.push("free full text[sb]".to_string());
        self
    }

    /// Filter to articles with any full text (including subscription-based)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .has_full_text();
    /// ```
    pub fn has_full_text(mut self) -> Self {
        self.filters.push("full text[sb]".to_string());
        self
    }

    /// Filter to articles with abstracts
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("genetics")
    ///     .has_abstract();
    /// ```
    pub fn has_abstract(mut self) -> Self {
        self.filters.push("hasabstract".to_string());
        self
    }

    /// Filter by article types
    ///
    /// # Arguments
    ///
    /// * `types` - Article types to include
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::{SearchQuery, ArticleType};
    ///
    /// let query = SearchQuery::new()
    ///     .query("hypertension")
    ///     .article_types(&[ArticleType::ClinicalTrial, ArticleType::Review]);
    /// ```
    pub fn article_types(mut self, types: &[ArticleType]) -> Self {
        if !types.is_empty() {
            let type_filters: Vec<String> = types
                .iter()
                .map(|t| t.to_query_string().to_string())
                .collect();

            if type_filters.len() == 1 {
                self.filters.push(type_filters[0].clone());
            } else {
                // Multiple types: (type1[pt] OR type2[pt] OR ...)
                let combined = format!("({})", type_filters.join(" OR "));
                self.filters.push(combined);
            }
        }
        self
    }

    /// Filter by language
    ///
    /// # Arguments
    ///
    /// * `language` - Language to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::{SearchQuery, Language};
    ///
    /// let query = SearchQuery::new()
    ///     .query("stem cells")
    ///     .language(Language::English);
    /// ```
    pub fn language(mut self, language: Language) -> Self {
        self.filters.push(language.to_query_string());
        self
    }

    /// Filter by publication date range
    ///
    /// # Arguments
    ///
    /// * `start_year` - Start year (inclusive)
    /// * `end_year` - End year (inclusive, optional)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("immunotherapy")
    ///     .date_range(2020, Some(2023));
    /// ```
    pub fn date_range(mut self, start_year: u32, end_year: Option<u32>) -> Self {
        let date_filter = match end_year {
            Some(end) => format!("{}:{}[pdat]", start_year, end),
            None => format!("{}:3000[pdat]", start_year), // Far future date
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter by publication date range with flexible precision
    ///
    /// # Arguments
    ///
    /// * `start` - Start date (can be year, (year, month), or (year, month, day))
    /// * `end` - End date (optional, same format as start)
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// // Year range
    /// let query = SearchQuery::new()
    ///     .query("covid vaccines")
    ///     .published_between(2020, Some(2023));
    ///
    /// // Month precision
    /// let query = SearchQuery::new()
    ///     .query("pandemic response")
    ///     .published_between((2020, 3), Some((2021, 12)));
    ///
    /// // Day precision
    /// let query = SearchQuery::new()
    ///     .query("outbreak analysis")
    ///     .published_between((2020, 3, 15), Some((2020, 12, 31)));
    ///
    /// // Open-ended (from date onwards)
    /// let query = SearchQuery::new()
    ///     .query("recent research")
    ///     .published_between(2023, None);
    /// ```
    pub fn published_between<S, E>(mut self, start: S, end: Option<E>) -> Self
    where
        S: Into<PubDate>,
        E: Into<PubDate>,
    {
        let start_date = start.into();
        let date_filter = match end {
            Some(end_date) => {
                let end_date = end_date.into();
                format!(
                    "{}:{}[pdat]",
                    start_date.to_pubmed_string(),
                    end_date.to_pubmed_string()
                )
            }
            None => format!("{}:3000[pdat]", start_date.to_pubmed_string()),
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter to articles published in a specific year
    ///
    /// # Arguments
    ///
    /// * `year` - Year to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("artificial intelligence")
    ///     .published_in_year(2023);
    /// ```
    pub fn published_in_year(mut self, year: u32) -> Self {
        self.filters.push(format!("{}[pdat]", year));
        self
    }

    /// Filter by entry date (when added to PubMed database)
    ///
    /// # Arguments
    ///
    /// * `start` - Start date (can be year, (year, month), or (year, month, day))
    /// * `end` - End date (optional, same format as start)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("recent discoveries")
    ///     .entry_date_between(2023, Some(2024));
    /// ```
    pub fn entry_date_between<S, E>(mut self, start: S, end: Option<E>) -> Self
    where
        S: Into<PubDate>,
        E: Into<PubDate>,
    {
        let start_date = start.into();
        let date_filter = match end {
            Some(end_date) => {
                let end_date = end_date.into();
                format!(
                    "{}:{}[edat]",
                    start_date.to_pubmed_string(),
                    end_date.to_pubmed_string()
                )
            }
            None => format!("{}:3000[edat]", start_date.to_pubmed_string()),
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter by modification date (when last updated in PubMed database)
    ///
    /// # Arguments
    ///
    /// * `start` - Start date (can be year, (year, month), or (year, month, day))
    /// * `end` - End date (optional, same format as start)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("updated articles")
    ///     .modification_date_between(2023, None);
    /// ```
    pub fn modification_date_between<S, E>(mut self, start: S, end: Option<E>) -> Self
    where
        S: Into<PubDate>,
        E: Into<PubDate>,
    {
        let start_date = start.into();
        let date_filter = match end {
            Some(end_date) => {
                let end_date = end_date.into();
                format!(
                    "{}:{}[mdat]",
                    start_date.to_pubmed_string(),
                    end_date.to_pubmed_string()
                )
            }
            None => format!("{}:3000[mdat]", start_date.to_pubmed_string()),
        };
        self.filters.push(date_filter);
        self
    }

    /// Filter to articles published after a specific date
    ///
    /// # Arguments
    ///
    /// * `date` - Date after which articles were published (can be year, (year, month), or (year, month, day))
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// // After a specific year
    /// let query = SearchQuery::new()
    ///     .query("crispr")
    ///     .published_after(2020);
    ///
    /// // After a specific month
    /// let query = SearchQuery::new()
    ///     .query("covid treatment")
    ///     .published_after((2020, 3));
    ///
    /// // After a specific date
    /// let query = SearchQuery::new()
    ///     .query("pandemic response")
    ///     .published_after((2020, 3, 15));
    /// ```
    pub fn published_after<D>(self, date: D) -> Self
    where
        D: Into<PubDate>,
    {
        self.published_between(date, None::<u32>)
    }

    /// Filter to articles published before a specific date
    ///
    /// # Arguments
    ///
    /// * `date` - Date before which articles were published (can be year, (year, month), or (year, month, day))
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// // Before a specific year
    /// let query = SearchQuery::new()
    ///     .query("genome sequencing")
    ///     .published_before(2020);
    ///
    /// // Before a specific month
    /// let query = SearchQuery::new()
    ///     .query("early research")
    ///     .published_before((2020, 3));
    ///
    /// // Before a specific date
    /// let query = SearchQuery::new()
    ///     .query("pre-pandemic studies")
    ///     .published_before((2020, 3, 15));
    /// ```
    pub fn published_before<D>(self, date: D) -> Self
    where
        D: Into<PubDate>,
    {
        self.published_between(1900, Some(date))
    }

    /// Set maximum number of results
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results to return
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("alzheimer")
    ///     .limit(50);
    /// ```
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add a custom filter
    ///
    /// # Arguments
    ///
    /// * `filter` - Custom PubMed search filter
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .custom_filter("humans[mh]");
    /// ```
    pub fn custom_filter<S: Into<String>>(mut self, filter: S) -> Self {
        self.filters.push(filter.into());
        self
    }

    /// Filter by MeSH major topic
    ///
    /// # Arguments
    ///
    /// * `mesh_term` - MeSH term to filter by as a major topic
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_major_topic("Diabetes Mellitus, Type 2");
    /// ```
    pub fn mesh_major_topic<S: Into<String>>(mut self, mesh_term: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Major Topic]", mesh_term.into()));
        self
    }

    /// Filter by MeSH term
    ///
    /// # Arguments
    ///
    /// * `mesh_term` - MeSH term to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_term("Neoplasms");
    /// ```
    pub fn mesh_term<S: Into<String>>(mut self, mesh_term: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Terms]", mesh_term.into()));
        self
    }

    /// Filter by multiple MeSH terms
    ///
    /// # Arguments
    ///
    /// * `mesh_terms` - MeSH terms to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_terms(&["Neoplasms", "Antineoplastic Agents"]);
    /// ```
    pub fn mesh_terms<S: AsRef<str>>(mut self, mesh_terms: &[S]) -> Self {
        for term in mesh_terms {
            self = self.mesh_term(term.as_ref());
        }
        self
    }

    /// Filter by MeSH subheading
    ///
    /// # Arguments
    ///
    /// * `subheading` - MeSH subheading to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .mesh_term("Diabetes Mellitus")
    ///     .mesh_subheading("drug therapy");
    /// ```
    pub fn mesh_subheading<S: Into<String>>(mut self, subheading: S) -> Self {
        self.filters
            .push(format!("{}[MeSH Subheading]", subheading.into()));
        self
    }

    /// Filter by a single article type (convenience method)
    ///
    /// # Arguments
    ///
    /// * `article_type` - The article type to filter by
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::{SearchQuery, ArticleType};
    ///
    /// let query = SearchQuery::new()
    ///     .query("diabetes treatment")
    ///     .article_type(ArticleType::ClinicalTrial);
    /// ```
    pub fn article_type(self, article_type: ArticleType) -> Self {
        self.article_types(&[article_type])
    }

    /// Filter by first author
    ///
    /// # Arguments
    ///
    /// * `author` - First author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer treatment")
    ///     .first_author("Smith J");
    /// ```
    pub fn first_author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters
            .push(format!("{}[First Author]", author.into()));
        self
    }

    /// Filter by last author
    ///
    /// # Arguments
    ///
    /// * `author` - Last author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("genomics")
    ///     .last_author("Johnson M");
    /// ```
    pub fn last_author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters.push(format!("{}[Last Author]", author.into()));
        self
    }

    /// Filter by any author
    ///
    /// # Arguments
    ///
    /// * `author` - Author name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .author("Williams K");
    /// ```
    pub fn author<S: Into<String>>(mut self, author: S) -> Self {
        self.filters.push(format!("{}[Author]", author.into()));
        self
    }

    /// Filter by institution/affiliation
    ///
    /// # Arguments
    ///
    /// * `institution` - Institution name to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cardiology research")
    ///     .affiliation("Harvard Medical School");
    /// ```
    pub fn affiliation<S: Into<String>>(mut self, institution: S) -> Self {
        self.filters
            .push(format!("{}[Affiliation]", institution.into()));
        self
    }

    /// Filter by ORCID identifier
    ///
    /// # Arguments
    ///
    /// * `orcid_id` - ORCID identifier to search for
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("computational biology")
    ///     .orcid("0000-0001-2345-6789");
    /// ```
    pub fn orcid<S: Into<String>>(mut self, orcid_id: S) -> Self {
        self.filters
            .push(format!("{}[Author - Identifier]", orcid_id.into()));
        self
    }

    /// Combine this query with another using AND logic
    ///
    /// # Arguments
    ///
    /// * `other` - Another SearchQuery to combine with
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query1 = SearchQuery::new().query("covid-19");
    /// let query2 = SearchQuery::new().query("vaccine");
    /// let combined = query1.and(query2);
    /// ```
    pub fn and(mut self, other: SearchQuery) -> Self {
        // Combine the queries by wrapping each in parentheses
        let self_query = self.build();
        let other_query = other.build();

        if !self_query.is_empty() && !other_query.is_empty() {
            // Create a new query with the combined result
            let combined_query = format!("({}) AND ({})", self_query, other_query);
            self.terms = vec![combined_query];
            self.filters = Vec::new();
        } else if !other_query.is_empty() {
            self.terms = vec![other_query];
            self.filters = Vec::new();
        }

        // Use the higher limit if set
        if other.limit.is_some() && (self.limit.is_none() || other.limit > self.limit) {
            self.limit = other.limit;
        }

        self
    }

    /// Combine this query with another using OR logic
    ///
    /// # Arguments
    ///
    /// * `other` - Another SearchQuery to combine with
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query1 = SearchQuery::new().query("diabetes");
    /// let query2 = SearchQuery::new().query("hypertension");
    /// let combined = query1.or(query2);
    /// ```
    pub fn or(mut self, other: SearchQuery) -> Self {
        // Combine the queries by wrapping each in parentheses
        let self_query = self.build();
        let other_query = other.build();

        if !self_query.is_empty() && !other_query.is_empty() {
            // Create a new query with the combined result
            let combined_query = format!("({}) OR ({})", self_query, other_query);
            self.terms = vec![combined_query];
            self.filters = Vec::new();
        } else if !other_query.is_empty() {
            self.terms = vec![other_query];
            self.filters = Vec::new();
        }

        // Use the higher limit if set
        if other.limit.is_some() && (self.limit.is_none() || other.limit > self.limit) {
            self.limit = other.limit;
        }

        self
    }

    /// Negate this query using NOT logic
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .negate();
    /// ```
    pub fn negate(mut self) -> Self {
        let self_query = self.build();

        if !self_query.is_empty() {
            let negated_query = format!("NOT ({})", self_query);
            self.terms = vec![negated_query];
            self.filters = Vec::new();
        }

        self
    }

    /// Exclude articles matching the given query
    ///
    /// # Arguments
    ///
    /// * `excluded` - SearchQuery representing articles to exclude
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let base_query = SearchQuery::new().query("cancer treatment");
    /// let exclude_query = SearchQuery::new().query("animal studies");
    /// let filtered = base_query.exclude(exclude_query);
    /// ```
    pub fn exclude(mut self, excluded: SearchQuery) -> Self {
        let self_query = self.build();
        let excluded_query = excluded.build();

        if !self_query.is_empty() && !excluded_query.is_empty() {
            let combined_query = format!("({}) NOT ({})", self_query, excluded_query);
            self.terms = vec![combined_query];
            self.filters = Vec::new();
        }

        self
    }

    /// Add parentheses around the current query for grouping
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .or(SearchQuery::new().query("tumor"))
    ///     .group();
    /// ```
    pub fn group(mut self) -> Self {
        let self_query = self.build();

        if !self_query.is_empty() {
            let grouped_query = format!("({})", self_query);
            self.terms = vec![grouped_query];
            self.filters = Vec::new();
        }

        self
    }

    /// Filter to human studies only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("drug treatment")
    ///     .human_studies_only();
    /// ```
    pub fn human_studies_only(mut self) -> Self {
        self.filters.push("humans[mh]".to_string());
        self
    }

    /// Filter to animal studies only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("preclinical research")
    ///     .animal_studies_only();
    /// ```
    pub fn animal_studies_only(mut self) -> Self {
        self.filters.push("animals[mh]".to_string());
        self
    }

    /// Filter by age group
    ///
    /// # Arguments
    ///
    /// * `age_group` - Age group to filter by (e.g., "Child", "Adult", "Aged")
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("pediatric medicine")
    ///     .age_group("Child");
    /// ```
    pub fn age_group<S: Into<String>>(mut self, age_group: S) -> Self {
        self.filters.push(format!("{}[mh]", age_group.into()));
        self
    }

    /// Validate the query structure and parameters
    ///
    /// # Returns
    ///
    /// Returns an error if the query is invalid, Ok(()) otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new().query("covid-19");
    /// assert!(query.validate().is_ok());
    /// ```
    pub fn validate(&self) -> crate::error::Result<()> {
        // Check if query is completely empty
        if self.terms.is_empty() && self.filters.is_empty() {
            return Err(crate::error::PubMedError::InvalidQuery(
                "Query cannot be empty".to_string(),
            ));
        }

        // Validate limit is reasonable
        if let Some(limit) = self.limit {
            if limit == 0 {
                return Err(crate::error::PubMedError::InvalidQuery(
                    "Limit must be greater than 0".to_string(),
                ));
            }
            if limit > 10000 {
                return Err(crate::error::PubMedError::InvalidQuery(
                    "Limit should not exceed 10,000 for performance reasons".to_string(),
                ));
            }
        }

        // Check for potentially problematic patterns
        let query_string = self.build();
        if query_string.len() > 4000 {
            return Err(crate::error::PubMedError::InvalidQuery(
                "Query string is too long (>4000 characters)".to_string(),
            ));
        }

        // Check for unbalanced parentheses
        let open_parens = query_string.matches('(').count();
        let close_parens = query_string.matches(')').count();
        if open_parens != close_parens {
            return Err(crate::error::PubMedError::InvalidQuery(
                "Unbalanced parentheses in query".to_string(),
            ));
        }

        Ok(())
    }

    /// Optimize the query for better performance
    ///
    /// # Returns
    ///
    /// Returns an optimized version of the query
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let optimized = SearchQuery::new()
    ///     .query("covid-19")
    ///     .published_after(2020)
    ///     .optimize();
    /// ```
    pub fn optimize(mut self) -> Self {
        // Remove duplicate filters
        self.filters.sort();
        self.filters.dedup();

        // Remove duplicate terms
        self.terms.sort();
        self.terms.dedup();

        // Remove empty terms and filters
        self.terms.retain(|term| !term.trim().is_empty());
        self.filters.retain(|filter| !filter.trim().is_empty());

        self
    }

    /// Get query statistics and information
    ///
    /// # Returns
    ///
    /// Returns a tuple of (term_count, filter_count, estimated_complexity)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .published_after(2020)
    ///     .free_full_text();
    ///
    /// let (terms, filters, complexity) = query.get_stats();
    /// ```
    pub fn get_stats(&self) -> (usize, usize, usize) {
        let term_count = self.terms.len();
        let filter_count = self.filters.len();

        // Estimate complexity based on query structure
        let query_string = self.build();
        let complexity = query_string.matches(" AND ").count()
            + query_string.matches(" OR ").count() * 2
            + query_string.matches(" NOT ").count() * 2
            + query_string.matches('(').count()
            + 1; // Base complexity

        (term_count, filter_count, complexity)
    }

    /// Build the final query string
    ///
    /// # Returns
    ///
    /// Returns the constructed PubMed query string
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query_string = SearchQuery::new()
    ///     .query("covid-19")
    ///     .open_access_only()
    ///     .published_after(2020)
    ///     .build();
    ///
    /// assert!(query_string.contains("covid-19"));
    /// assert!(query_string.contains("free full text[sb]"));
    /// ```
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        // Add search terms
        if !self.terms.is_empty() {
            parts.push(self.terms.join(" "));
        }

        // Add filters
        parts.extend(self.filters.clone());

        parts.join(" AND ")
    }

    /// Get the limit for this query
    pub fn get_limit(&self) -> usize {
        self.limit.unwrap_or(20)
    }

    /// Execute the search using the provided PubMed client
    ///
    /// # Arguments
    ///
    /// * `client` - PubMed client to use for the search
    ///
    /// # Returns
    ///
    /// Returns a list of PMIDs matching the query
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::{PubMedClient, pubmed::SearchQuery};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let pmids = SearchQuery::new()
    ///         .query("covid-19")
    ///         .open_access_only()
    ///         .limit(10)
    ///         .search(&client)
    ///         .await?;
    ///
    ///     println!("Found {} articles", pmids.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn search(&self, client: &PubMedClient) -> Result<Vec<String>> {
        let query_string = self.build();
        client
            .search_articles(&query_string, self.get_limit())
            .await
    }

    /// Execute the search and fetch full article metadata
    ///
    /// # Arguments
    ///
    /// * `client` - PubMed client to use for the search
    ///
    /// # Returns
    ///
    /// Returns a list of PubMed articles with metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::{PubMedClient, pubmed::SearchQuery};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let articles = SearchQuery::new()
    ///         .query("machine learning medicine")
    ///         .free_full_text()
    ///         .published_after(2022)
    ///         .limit(5)
    ///         .search_and_fetch(&client)
    ///         .await?;
    ///
    ///     for article in articles {
    ///         println!("{}: {}", article.pmid, article.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_and_fetch(&self, client: &PubMedClient) -> Result<Vec<PubMedArticle>> {
        let query_string = self.build();
        client
            .search_and_fetch(&query_string, self.get_limit())
            .await
    }
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_query_builder() {
        let query = SearchQuery::new().query("covid-19").build();

        assert_eq!(query, "covid-19");
    }

    #[test]
    fn test_query_with_open_access_filter() {
        let query = SearchQuery::new()
            .query("cancer")
            .open_access_only()
            .build();

        assert_eq!(query, "cancer AND free full text[sb]");
    }

    #[test]
    fn test_complex_query() {
        let query = SearchQuery::new()
            .query("machine learning")
            .free_full_text()
            .published_after(2020)
            .article_types(&[ArticleType::Review, ArticleType::SystematicReview])
            .language(Language::English)
            .build();

        let expected = "machine learning AND free full text[sb] AND 2020:3000[pdat] AND (Review[pt] OR Systematic Review[pt]) AND English[lang]";
        assert_eq!(query, expected);
    }

    #[test]
    fn test_date_range() {
        let query = SearchQuery::new()
            .query("diabetes")
            .date_range(2020, Some(2023))
            .build();

        assert_eq!(query, "diabetes AND 2020:2023[pdat]");
    }

    #[test]
    fn test_multiple_terms() {
        let query = SearchQuery::new()
            .terms(&["cancer", "treatment", "immunotherapy"])
            .build();

        assert_eq!(query, "cancer treatment immunotherapy");
    }

    #[test]
    fn test_custom_filter() {
        let query = SearchQuery::new()
            .query("genetics")
            .custom_filter("humans[mh]")
            .build();

        assert_eq!(query, "genetics AND humans[mh]");
    }

    #[test]
    fn test_empty_query() {
        let query = SearchQuery::new().build();
        assert_eq!(query, "");
    }

    #[test]
    fn test_limit() {
        let query = SearchQuery::new().query("test").limit(50);

        assert_eq!(query.get_limit(), 50);
    }

    #[test]
    fn test_default_limit() {
        let query = SearchQuery::new().query("test");

        assert_eq!(query.get_limit(), 20);
    }

    #[test]
    fn test_mesh_major_topic() {
        let query = SearchQuery::new()
            .mesh_major_topic("Diabetes Mellitus")
            .build();

        assert_eq!(query, "Diabetes Mellitus[MeSH Major Topic]");
    }

    #[test]
    fn test_mesh_term() {
        let query = SearchQuery::new().mesh_term("Hypertension").build();

        assert_eq!(query, "Hypertension[MeSH Terms]");
    }

    #[test]
    fn test_mesh_terms_multiple() {
        let query = SearchQuery::new()
            .mesh_terms(&["Cancer", "Chemotherapy"])
            .build();

        assert_eq!(query, "Cancer[MeSH Terms] AND Chemotherapy[MeSH Terms]");
    }

    #[test]
    fn test_mesh_subheading() {
        let query = SearchQuery::new().mesh_subheading("drug therapy").build();

        assert_eq!(query, "drug therapy[MeSH Subheading]");
    }

    #[test]
    fn test_clinical_trials_only() {
        let query = SearchQuery::new()
            .query("treatment")
            .article_type(ArticleType::ClinicalTrial)
            .build();

        assert_eq!(query, "treatment AND Clinical Trial[pt]");
    }

    #[test]
    fn test_mesh_complex_query() {
        let query = SearchQuery::new()
            .mesh_major_topic("COVID-19")
            .mesh_subheading("prevention & control")
            .published_after(2022)
            .free_full_text()
            .build();

        assert_eq!(
            query,
            "COVID-19[MeSH Major Topic] AND prevention & control[MeSH Subheading] AND 2022:3000[pdat] AND free full text[sb]"
        );
    }

    #[test]
    fn test_first_author_filter() {
        let query = SearchQuery::new()
            .query("cancer treatment")
            .first_author("Smith J")
            .build();

        assert_eq!(query, "cancer treatment AND Smith J[First Author]");
    }

    #[test]
    fn test_last_author_filter() {
        let query = SearchQuery::new()
            .query("genomics")
            .last_author("Johnson M")
            .build();

        assert_eq!(query, "genomics AND Johnson M[Last Author]");
    }

    #[test]
    fn test_author_filter() {
        let query = SearchQuery::new()
            .query("machine learning")
            .author("Williams K")
            .build();

        assert_eq!(query, "machine learning AND Williams K[Author]");
    }

    #[test]
    fn test_affiliation_filter() {
        let query = SearchQuery::new()
            .query("cardiology research")
            .affiliation("Harvard Medical School")
            .build();

        assert_eq!(
            query,
            "cardiology research AND Harvard Medical School[Affiliation]"
        );
    }

    #[test]
    fn test_orcid_filter() {
        let query = SearchQuery::new()
            .query("computational biology")
            .orcid("0000-0001-2345-6789")
            .build();

        assert_eq!(
            query,
            "computational biology AND 0000-0001-2345-6789[Author - Identifier]"
        );
    }

    #[test]
    fn test_author_complex_query() {
        let query = SearchQuery::new()
            .query("diabetes treatment")
            .first_author("Smith J")
            .affiliation("Harvard Medical School")
            .published_after(2020)
            .free_full_text()
            .build();

        assert_eq!(
            query,
            "diabetes treatment AND Smith J[First Author] AND Harvard Medical School[Affiliation] AND 2020:3000[pdat] AND free full text[sb]"
        );
    }

    // Tests for PubDate type
    #[test]
    fn test_pubdate_creation() {
        let year_only = PubDate::from(2023);
        assert_eq!(year_only.to_pubmed_string(), "2023");

        let year_month = PubDate::from((2023, 6));
        assert_eq!(year_month.to_pubmed_string(), "2023/06");

        let year_month_day = PubDate::from((2023, 6, 15));
        assert_eq!(year_month_day.to_pubmed_string(), "2023/06/15");
    }

    #[test]
    fn test_pubdate_constructors() {
        let date1 = PubDate::new(2023);
        let date2 = PubDate::with_month(2023, 6);
        let date3 = PubDate::with_day(2023, 6, 15);

        assert_eq!(date1.to_pubmed_string(), "2023");
        assert_eq!(date2.to_pubmed_string(), "2023/06");
        assert_eq!(date3.to_pubmed_string(), "2023/06/15");
    }

    // New tests for field-specific searches
    #[test]
    fn test_title_contains() {
        let query = SearchQuery::new()
            .title_contains("machine learning")
            .build();

        assert_eq!(query, "machine learning[Title]");
    }

    #[test]
    fn test_abstract_contains() {
        let query = SearchQuery::new()
            .abstract_contains("neural networks")
            .build();

        assert_eq!(query, "neural networks[Abstract]");
    }

    #[test]
    fn test_title_or_abstract() {
        let query = SearchQuery::new().title_or_abstract("CRISPR").build();

        assert_eq!(query, "CRISPR[Title/Abstract]");
    }

    #[test]
    fn test_journal_filter() {
        let query = SearchQuery::new().query("cancer").journal("Nature").build();

        assert_eq!(query, "cancer AND Nature[Journal]");
    }

    #[test]
    fn test_journal_abbreviation() {
        let query = SearchQuery::new().journal_abbreviation("Nat Med").build();

        assert_eq!(query, "Nat Med[Journal Title Abbreviation]");
    }

    #[test]
    fn test_grant_number() {
        let query = SearchQuery::new().grant_number("R01AI123456").build();

        assert_eq!(query, "R01AI123456[Grant Number]");
    }

    #[test]
    fn test_isbn_issn() {
        let query = SearchQuery::new()
            .isbn("978-0123456789")
            .issn("1234-5678")
            .build();

        assert_eq!(query, "978-0123456789[ISBN] AND 1234-5678[ISSN]");
    }

    // Tests for publication type convenience methods
    #[test]
    fn test_review_articles_only() {
        let query = SearchQuery::new()
            .query("cancer")
            .article_type(ArticleType::Review)
            .build();

        assert_eq!(query, "cancer AND Review[pt]");
    }

    #[test]
    fn test_systematic_reviews_only() {
        let query = SearchQuery::new()
            .query("treatment")
            .article_type(ArticleType::SystematicReview)
            .build();

        assert_eq!(query, "treatment AND Systematic Review[pt]");
    }

    #[test]
    fn test_meta_analyses_only() {
        let query = SearchQuery::new()
            .query("efficacy")
            .article_type(ArticleType::MetaAnalysis)
            .build();

        assert_eq!(query, "efficacy AND Meta-Analysis[pt]");
    }

    #[test]
    fn test_case_reports_only() {
        let query = SearchQuery::new()
            .query("rare disease")
            .article_type(ArticleType::CaseReport)
            .build();

        assert_eq!(query, "rare disease AND Case Reports[pt]");
    }

    #[test]
    fn test_randomized_controlled_trials_only() {
        let query = SearchQuery::new()
            .query("new drug")
            .article_type(ArticleType::RandomizedControlledTrial)
            .build();

        assert_eq!(query, "new drug AND Randomized Controlled Trial[pt]");
    }

    #[test]
    fn test_observational_studies_only() {
        let query = SearchQuery::new()
            .query("population health")
            .article_type(ArticleType::ObservationalStudy)
            .build();

        assert_eq!(query, "population health AND Observational Study[pt]");
    }

    // Tests for advanced date filtering
    #[test]
    fn test_published_in_year() {
        let query = SearchQuery::new()
            .query("AI")
            .published_in_year(2023)
            .build();

        assert_eq!(query, "AI AND 2023[pdat]");
    }

    // Tests for new simplified date API
    #[test]
    fn test_published_between_years() {
        let query = SearchQuery::new()
            .query("covid")
            .published_between(2020, Some(2023))
            .build();

        assert_eq!(query, "covid AND 2020:2023[pdat]");
    }

    #[test]
    fn test_published_between_months() {
        let query = SearchQuery::new()
            .query("pandemic")
            .published_between((2020, 3), Some((2021, 12)))
            .build();

        assert_eq!(query, "pandemic AND 2020/03:2021/12[pdat]");
    }

    #[test]
    fn test_published_between_days() {
        let query = SearchQuery::new()
            .query("outbreak")
            .published_between((2020, 3, 15), Some((2020, 12, 31)))
            .build();

        assert_eq!(query, "outbreak AND 2020/03/15:2020/12/31[pdat]");
    }

    #[test]
    fn test_published_between_open_ended() {
        let query = SearchQuery::new()
            .query("recent")
            .published_between(2023, None::<u32>)
            .build();

        assert_eq!(query, "recent AND 2023:3000[pdat]");
    }

    #[test]
    fn test_published_after_year() {
        let query = SearchQuery::new().query("ai").published_after(2020).build();

        assert_eq!(query, "ai AND 2020:3000[pdat]");
    }

    #[test]
    fn test_published_after_month() {
        let query = SearchQuery::new()
            .query("covid")
            .published_after((2020, 3))
            .build();

        assert_eq!(query, "covid AND 2020/03:3000[pdat]");
    }

    #[test]
    fn test_published_after_day() {
        let query = SearchQuery::new()
            .query("pandemic")
            .published_after((2020, 3, 15))
            .build();

        assert_eq!(query, "pandemic AND 2020/03/15:3000[pdat]");
    }

    #[test]
    fn test_published_before_year() {
        let query = SearchQuery::new()
            .query("historical")
            .published_before(2020)
            .build();

        assert_eq!(query, "historical AND 1900:2020[pdat]");
    }

    #[test]
    fn test_published_before_month() {
        let query = SearchQuery::new()
            .query("early")
            .published_before((2020, 3))
            .build();

        assert_eq!(query, "early AND 1900:2020/03[pdat]");
    }

    #[test]
    fn test_published_before_day() {
        let query = SearchQuery::new()
            .query("pre-pandemic")
            .published_before((2020, 3, 15))
            .build();

        assert_eq!(query, "pre-pandemic AND 1900:2020/03/15[pdat]");
    }

    #[test]
    fn test_entry_date_between() {
        let query = SearchQuery::new()
            .query("recent")
            .entry_date_between(2023, Some(2024))
            .build();

        assert_eq!(query, "recent AND 2023:2024[edat]");
    }

    #[test]
    fn test_entry_date_between_with_precision() {
        let query = SearchQuery::new()
            .query("new entries")
            .entry_date_between((2023, 6), None::<u32>)
            .build();

        assert_eq!(query, "new entries AND 2023/06:3000[edat]");
    }

    #[test]
    fn test_modification_date_between() {
        let query = SearchQuery::new()
            .query("updated")
            .modification_date_between(2023, None::<u32>)
            .build();

        assert_eq!(query, "updated AND 2023:3000[mdat]");
    }

    #[test]
    fn test_modification_date_between_with_precision() {
        let query = SearchQuery::new()
            .query("recently modified")
            .modification_date_between((2023, 1, 1), Some((2023, 12, 31)))
            .build();

        assert_eq!(query, "recently modified AND 2023/01/01:2023/12/31[mdat]");
    }

    // Tests for boolean query combinations
    #[test]
    fn test_and_combination() {
        let query1 = SearchQuery::new().query("covid-19");
        let query2 = SearchQuery::new().query("vaccine");
        let combined = query1.and(query2).build();

        assert_eq!(combined, "(covid-19) AND (vaccine)");
    }

    #[test]
    fn test_or_combination() {
        let query1 = SearchQuery::new().query("diabetes");
        let query2 = SearchQuery::new().query("hypertension");
        let combined = query1.or(query2).build();

        assert_eq!(combined, "(diabetes) OR (hypertension)");
    }

    #[test]
    fn test_not_query() {
        let query = SearchQuery::new().query("cancer").negate().build();

        assert_eq!(query, "NOT (cancer)");
    }

    #[test]
    fn test_exclude_query() {
        let base_query = SearchQuery::new().query("cancer treatment");
        let exclude_query = SearchQuery::new().query("animal studies");
        let filtered = base_query.exclude(exclude_query).build();

        assert_eq!(filtered, "(cancer treatment) NOT (animal studies)");
    }

    #[test]
    fn test_group_query() {
        let query = SearchQuery::new().query("cancer").group().build();

        assert_eq!(query, "(cancer)");
    }

    #[test]
    fn test_complex_boolean_query() {
        let query1 = SearchQuery::new().title_contains("machine learning");
        let query2 = SearchQuery::new().mesh_term("Artificial Intelligence");
        let query3 = SearchQuery::new().mesh_term("Deep Learning");

        let combined = query1
            .and(query2.or(query3).group())
            .published_after(2020)
            .build();

        assert!(combined.contains("machine learning[Title]"));
        assert!(combined.contains("Artificial Intelligence[MeSH Terms]"));
        assert!(combined.contains("Deep Learning[MeSH Terms]"));
        assert!(combined.contains("2020:3000[pdat]"));
    }

    // Tests for species and age filtering
    #[test]
    fn test_human_studies_only() {
        let query = SearchQuery::new()
            .query("drug treatment")
            .human_studies_only()
            .build();

        assert_eq!(query, "drug treatment AND humans[mh]");
    }

    #[test]
    fn test_animal_studies_only() {
        let query = SearchQuery::new()
            .query("preclinical")
            .animal_studies_only()
            .build();

        assert_eq!(query, "preclinical AND animals[mh]");
    }

    #[test]
    fn test_age_group() {
        let query = SearchQuery::new()
            .query("pediatric")
            .age_group("Child")
            .build();

        assert_eq!(query, "pediatric AND Child[mh]");
    }

    // Tests for expanded language support
    #[test]
    fn test_additional_languages() {
        let languages = vec![
            (Language::Russian, "Russian[lang]"),
            (Language::Portuguese, "Portuguese[lang]"),
            (Language::Arabic, "Arabic[lang]"),
            (Language::Korean, "Korean[lang]"),
            (Language::Turkish, "Turkish[lang]"),
        ];

        for (lang, expected) in languages {
            let query = SearchQuery::new().query("test").language(lang).build();
            assert_eq!(query, format!("test AND {}", expected));
        }
    }

    // Tests for validation
    #[test]
    fn test_validate_empty_query() {
        let query = SearchQuery::new();
        assert!(query.validate().is_err());
    }

    #[test]
    fn test_validate_valid_query() {
        let query = SearchQuery::new().query("covid-19");
        assert!(query.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_limit() {
        let query = SearchQuery::new().query("test").limit(0);
        assert!(query.validate().is_err());
    }

    #[test]
    fn test_validate_excessive_limit() {
        let query = SearchQuery::new().query("test").limit(20000);
        assert!(query.validate().is_err());
    }

    // Tests for optimization
    #[test]
    fn test_optimize_removes_duplicates() {
        let mut query = SearchQuery::new();
        query.terms = vec!["cancer".to_string(), "cancer".to_string()];
        query.filters = vec!["Review[pt]".to_string(), "Review[pt]".to_string()];

        let optimized = query.optimize();
        assert_eq!(optimized.terms.len(), 1);
        assert_eq!(optimized.filters.len(), 1);
    }

    #[test]
    fn test_optimize_removes_empty() {
        let mut query = SearchQuery::new();
        query.terms = vec!["cancer".to_string(), "".to_string(), "   ".to_string()];
        query.filters = vec!["Review[pt]".to_string(), "".to_string()];

        let optimized = query.optimize();
        assert_eq!(optimized.terms.len(), 1);
        assert_eq!(optimized.filters.len(), 1);
    }

    // Tests for query statistics
    #[test]
    fn test_get_stats() {
        let query = SearchQuery::new()
            .query("machine learning")
            .published_after(2020)
            .free_full_text();

        let (terms, filters, complexity) = query.get_stats();
        assert_eq!(terms, 1);
        assert_eq!(filters, 2);
        assert!(complexity > 0);
    }

    // Integration test for complex real-world query
    #[test]
    fn test_comprehensive_real_world_query() {
        let ai_query = SearchQuery::new()
            .title_contains("machine learning")
            .or(SearchQuery::new().mesh_term("Artificial Intelligence"));

        let medical_query = SearchQuery::new()
            .mesh_term("Medicine")
            .or(SearchQuery::new().mesh_term("Healthcare"));

        let final_query = ai_query
            .and(medical_query)
            .published_after(2020)
            .article_type(ArticleType::Review)
            .human_studies_only()
            .free_full_text()
            .language(Language::English)
            .limit(50);

        let query_string = final_query.build();

        // Verify key components are present
        assert!(query_string.contains("machine learning[Title]"));
        assert!(query_string.contains("Artificial Intelligence[MeSH Terms]"));
        assert!(query_string.contains("Medicine[MeSH Terms]"));
        assert!(query_string.contains("Review[pt]"));
        assert!(query_string.contains("humans[mh]"));
        assert!(query_string.contains("free full text[sb]"));
        assert!(query_string.contains("English[lang]"));
        assert!(query_string.contains("2020:3000[pdat]"));

        // Verify boolean logic structure
        assert!(query_string.contains(" AND "));
        assert!(query_string.contains(" OR "));

        assert!(final_query.validate().is_ok());
        assert_eq!(final_query.get_limit(), 50);
    }
}
