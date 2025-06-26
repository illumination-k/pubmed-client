//! Search methods for field-specific filtering and content access

use super::{ArticleType, Language, SearchQuery};

impl SearchQuery {
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

    /// Filter by a single article type (convenience method)
    ///
    /// # Arguments
    ///
    /// * `article_type` - Article type to filter by
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
}
