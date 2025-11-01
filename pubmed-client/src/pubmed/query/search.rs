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
        self.filters.push(format!("{}[ti]", title.into()));
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
        self.filters.push(format!("{}[tiab]", abstract_text.into()));
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
        self.filters.push(format!("{}[tiab]", text.into()));
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
        self.filters.push(format!("{}[ta]", journal.into()));
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
        self.filters.push(format!("{}[ta]", abbreviation.into()));
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
        self.filters.push(format!("{}[gr]", grant_number.into()));
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

    /// Filter to articles with free full text only
    ///
    /// Includes PMC, Bookshelf, and publishers' websites.
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("cancer")
    ///     .free_full_text_only();
    /// ```
    pub fn free_full_text_only(mut self) -> Self {
        self.filters.push("free full text[sb]".to_string());
        self
    }

    /// Filter to articles with full text links (including subscription-based)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("machine learning")
    ///     .full_text_only();
    /// ```
    pub fn full_text_only(mut self) -> Self {
        self.filters.push("full text[sb]".to_string());
        self
    }

    /// Filter to articles with PMC full text only
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::pubmed::SearchQuery;
    ///
    /// let query = SearchQuery::new()
    ///     .query("diabetes")
    ///     .pmc_only();
    /// ```
    pub fn pmc_only(mut self) -> Self {
        self.filters.push("pmc[sb]".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_contains() {
        let query = SearchQuery::new().title_contains("machine learning");
        assert_eq!(query.build(), "machine learning[ti]");
    }

    #[test]
    fn test_abstract_contains() {
        let query = SearchQuery::new().abstract_contains("deep learning neural networks");
        assert_eq!(query.build(), "deep learning neural networks[tiab]");
    }

    #[test]
    fn test_title_or_abstract() {
        let query = SearchQuery::new().title_or_abstract("CRISPR gene editing");
        assert_eq!(query.build(), "CRISPR gene editing[tiab]");
    }

    #[test]
    fn test_journal() {
        let query = SearchQuery::new().journal("Nature");
        assert_eq!(query.build(), "Nature[ta]");
    }

    #[test]
    fn test_journal_abbreviation() {
        let query = SearchQuery::new().journal_abbreviation("Nat Med");
        assert_eq!(query.build(), "Nat Med[ta]");
    }

    #[test]
    fn test_grant_number() {
        let query = SearchQuery::new().grant_number("R01AI123456");
        assert_eq!(query.build(), "R01AI123456[gr]");
    }

    #[test]
    fn test_isbn() {
        let query = SearchQuery::new().isbn("978-0123456789");
        assert_eq!(query.build(), "978-0123456789[ISBN]");
    }

    #[test]
    fn test_issn() {
        let query = SearchQuery::new().issn("1234-5678");
        assert_eq!(query.build(), "1234-5678[ISSN]");
    }

    #[test]
    fn test_free_full_text_only() {
        let query = SearchQuery::new().free_full_text_only();
        assert_eq!(query.build(), "free full text[sb]");
    }

    #[test]
    fn test_full_text_only() {
        let query = SearchQuery::new().full_text_only();
        assert_eq!(query.build(), "full text[sb]");
    }

    #[test]
    fn test_pmc_only() {
        let query = SearchQuery::new().pmc_only();
        assert_eq!(query.build(), "pmc[sb]");
    }

    #[test]
    fn test_has_abstract() {
        let query = SearchQuery::new().has_abstract();
        assert_eq!(query.build(), "hasabstract");
    }

    #[test]
    fn test_single_article_type() {
        let query = SearchQuery::new().article_type(ArticleType::ClinicalTrial);
        assert_eq!(query.build(), "Clinical Trial[pt]");
    }

    #[test]
    fn test_multiple_article_types() {
        let types = [ArticleType::ClinicalTrial, ArticleType::Review];
        let query = SearchQuery::new().article_types(&types);
        assert_eq!(query.build(), "(Clinical Trial[pt] OR Review[pt])");
    }

    #[test]
    fn test_empty_article_types() {
        let types: &[ArticleType] = &[];
        let query = SearchQuery::new().article_types(types);
        assert_eq!(query.build(), "");
    }

    #[test]
    fn test_single_article_type_via_array() {
        let types = [ArticleType::Review];
        let query = SearchQuery::new().article_types(&types);
        assert_eq!(query.build(), "Review[pt]");
    }

    #[test]
    fn test_language() {
        let query = SearchQuery::new().language(Language::English);
        assert_eq!(query.build(), "English[la]");
    }

    #[test]
    fn test_language_other() {
        let query = SearchQuery::new().language(Language::Other("Esperanto".to_string()));
        assert_eq!(query.build(), "Esperanto[la]");
    }

    #[test]
    fn test_combined_search_filters() {
        let query = SearchQuery::new()
            .query("cancer treatment")
            .title_contains("immunotherapy")
            .journal("Nature")
            .free_full_text_only()
            .article_type(ArticleType::ClinicalTrial)
            .language(Language::English);

        let expected = "cancer treatment AND immunotherapy[ti] AND Nature[ta] AND free full text[sb] AND Clinical Trial[pt] AND English[la]";
        assert_eq!(query.build(), expected);
    }

    #[test]
    fn test_multiple_journal_filters() {
        let query = SearchQuery::new().journal("Nature").journal("Science");
        assert_eq!(query.build(), "Nature[ta] AND Science[ta]");
    }

    #[test]
    fn test_title_and_abstract_separate() {
        let query = SearchQuery::new()
            .title_contains("machine learning")
            .abstract_contains("neural networks");
        assert_eq!(
            query.build(),
            "machine learning[ti] AND neural networks[tiab]"
        );
    }

    #[test]
    fn test_all_text_availability_filters() {
        let query = SearchQuery::new()
            .query("research")
            .has_abstract()
            .full_text_only()
            .free_full_text_only()
            .pmc_only();
        assert_eq!(
            query.build(),
            "research AND hasabstract AND full text[sb] AND free full text[sb] AND pmc[sb]"
        );
    }

    #[test]
    fn test_many_article_types() {
        let types = [
            ArticleType::ClinicalTrial,
            ArticleType::Review,
            ArticleType::MetaAnalysis,
            ArticleType::SystematicReview,
        ];
        let query = SearchQuery::new().article_types(&types);
        let expected =
            "(Clinical Trial[pt] OR Review[pt] OR Meta-Analysis[pt] OR Systematic Review[pt])";
        assert_eq!(query.build(), expected);
    }

    #[test]
    fn test_identifier_fields() {
        let query = SearchQuery::new()
            .grant_number("R01CA123456")
            .isbn("978-0123456789")
            .issn("0028-0836");

        let expected = "R01CA123456[gr] AND 978-0123456789[ISBN] AND 0028-0836[ISSN]";
        assert_eq!(query.build(), expected);
    }
}
