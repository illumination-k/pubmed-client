//! Filter types and enums for PubMed query filtering

/// Validate that a year is within the range valid for biomedical publications (1800–3000).
///
/// Returns `Ok(())` if valid, or `Err(String)` with a descriptive message if not.
/// Bindings convert the error message to their native error type.
pub fn validate_year(year: u32) -> Result<(), String> {
    if !(1800..=3000).contains(&year) {
        Err(format!("Year must be between 1800 and 3000, got: {}", year))
    } else {
        Ok(())
    }
}

/// Sort order for PubMed search results
///
/// Controls how ESearch results are ordered. The default sort (when not specified)
/// is by relevance for most queries.
///
/// See: <https://www.ncbi.nlm.nih.gov/books/NBK25499/#chapter4.ESearch>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    /// Sort by relevance (default PubMed behavior)
    Relevance,
    /// Sort by publication date (newest first)
    PublicationDate,
    /// Sort by first author name (alphabetical)
    FirstAuthor,
    /// Sort by journal name (alphabetical)
    JournalName,
}

impl SortOrder {
    /// Parse a sort order from a case-insensitive string.
    ///
    /// Accepted values: `"relevance"`, `"pub_date"` / `"publication_date"` / `"date"`,
    /// `"author"` / `"first_author"`, `"journal"` / `"journal_name"`.
    ///
    /// Returns `Err(String)` for unrecognised input; bindings convert to their native error type.
    pub fn from_str_insensitive(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "relevance" => Ok(SortOrder::Relevance),
            "pub_date" | "publication_date" | "date" => Ok(SortOrder::PublicationDate),
            "author" | "first_author" => Ok(SortOrder::FirstAuthor),
            "journal" | "journal_name" => Ok(SortOrder::JournalName),
            _ => Err(format!(
                "Invalid sort order: '{}'. Supported values: relevance, pub_date, author, journal",
                s
            )),
        }
    }

    /// Get the API parameter value for this sort order
    pub(crate) fn as_api_param(&self) -> &str {
        match self {
            SortOrder::Relevance => "relevance",
            SortOrder::PublicationDate => "pub_date",
            SortOrder::FirstAuthor => "Author",
            SortOrder::JournalName => "JournalName",
        }
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
    /// Parse an article type from a case-insensitive string.
    ///
    /// Accepted values (case-insensitive): `"Clinical Trial"`, `"Review"`, `"Systematic Review"`,
    /// `"Meta-Analysis"` / `"Meta Analysis"`, `"Case Reports"` / `"Case Report"`,
    /// `"Randomized Controlled Trial"` / `"RCT"`, `"Observational Study"`.
    ///
    /// Returns `Err(String)` for unrecognised input; bindings convert to their native error type.
    pub fn from_str_insensitive(s: &str) -> Result<Self, String> {
        match s.trim().to_lowercase().as_str() {
            "clinical trial" => Ok(ArticleType::ClinicalTrial),
            "review" => Ok(ArticleType::Review),
            "systematic review" => Ok(ArticleType::SystematicReview),
            "meta-analysis" | "meta analysis" => Ok(ArticleType::MetaAnalysis),
            "case reports" | "case report" => Ok(ArticleType::CaseReport),
            "randomized controlled trial" | "rct" => Ok(ArticleType::RandomizedControlledTrial),
            "observational study" => Ok(ArticleType::ObservationalStudy),
            _ => Err(format!(
                "Invalid article type: '{}'. Supported types: Clinical Trial, Review, Systematic Review, Meta-Analysis, Case Reports, Randomized Controlled Trial, Observational Study",
                s
            )),
        }
    }

    pub(crate) fn to_query_string(&self) -> &'static str {
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
    /// Parse a language from a case-insensitive string.
    ///
    /// Accepts full English names (`"english"`, `"japanese"`, …) and ISO 639-2 three-letter codes
    /// (`"eng"`, `"jpn"`, …). Unrecognised values fall back to `Language::Other(s)` rather than
    /// returning an error, so callers never need to handle the unknown-language case.
    pub fn from_str_insensitive(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "english" | "eng" => Language::English,
            "japanese" | "jpn" => Language::Japanese,
            "german" | "ger" | "deu" => Language::German,
            "french" | "fre" | "fra" => Language::French,
            "spanish" | "spa" => Language::Spanish,
            "italian" | "ita" => Language::Italian,
            "chinese" | "chi" | "zho" => Language::Chinese,
            "russian" | "rus" => Language::Russian,
            "portuguese" | "por" => Language::Portuguese,
            "arabic" | "ara" => Language::Arabic,
            "dutch" | "dut" | "nld" => Language::Dutch,
            "korean" | "kor" => Language::Korean,
            "polish" | "pol" => Language::Polish,
            "swedish" | "swe" => Language::Swedish,
            "danish" | "dan" => Language::Danish,
            "norwegian" | "nor" => Language::Norwegian,
            "finnish" | "fin" => Language::Finnish,
            "turkish" | "tur" => Language::Turkish,
            "hebrew" | "heb" => Language::Hebrew,
            "czech" | "cze" | "ces" => Language::Czech,
            "hungarian" | "hun" => Language::Hungarian,
            "greek" | "gre" | "ell" => Language::Greek,
            _ => Language::Other(s.trim().to_string()),
        }
    }

    pub(crate) fn to_query_string(&self) -> String {
        match self {
            Language::English => "English[la]".to_string(),
            Language::Japanese => "Japanese[la]".to_string(),
            Language::German => "German[la]".to_string(),
            Language::French => "French[la]".to_string(),
            Language::Spanish => "Spanish[la]".to_string(),
            Language::Italian => "Italian[la]".to_string(),
            Language::Chinese => "Chinese[la]".to_string(),
            Language::Russian => "Russian[la]".to_string(),
            Language::Portuguese => "Portuguese[la]".to_string(),
            Language::Arabic => "Arabic[la]".to_string(),
            Language::Dutch => "Dutch[la]".to_string(),
            Language::Korean => "Korean[la]".to_string(),
            Language::Polish => "Polish[la]".to_string(),
            Language::Swedish => "Swedish[la]".to_string(),
            Language::Danish => "Danish[la]".to_string(),
            Language::Norwegian => "Norwegian[la]".to_string(),
            Language::Finnish => "Finnish[la]".to_string(),
            Language::Turkish => "Turkish[la]".to_string(),
            Language::Hebrew => "Hebrew[la]".to_string(),
            Language::Czech => "Czech[la]".to_string(),
            Language::Hungarian => "Hungarian[la]".to_string(),
            Language::Greek => "Greek[la]".to_string(),
            Language::Other(lang) => format!("{lang}[la]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_type_to_query_string() {
        let test_cases = vec![
            (ArticleType::ClinicalTrial, "Clinical Trial[pt]"),
            (ArticleType::Review, "Review[pt]"),
            (ArticleType::SystematicReview, "Systematic Review[pt]"),
            (ArticleType::MetaAnalysis, "Meta-Analysis[pt]"),
            (ArticleType::CaseReport, "Case Reports[pt]"),
            (
                ArticleType::RandomizedControlledTrial,
                "Randomized Controlled Trial[pt]",
            ),
            (ArticleType::ObservationalStudy, "Observational Study[pt]"),
        ];

        for (article_type, expected) in test_cases {
            assert_eq!(article_type.to_query_string(), expected);
        }
    }

    #[test]
    fn test_language_to_query_string() {
        let test_cases = vec![
            (Language::English, "English[la]"),
            (Language::Japanese, "Japanese[la]"),
            (Language::German, "German[la]"),
            (Language::French, "French[la]"),
            (Language::Spanish, "Spanish[la]"),
            (Language::Italian, "Italian[la]"),
            (Language::Chinese, "Chinese[la]"),
            (Language::Russian, "Russian[la]"),
            (Language::Portuguese, "Portuguese[la]"),
            (Language::Arabic, "Arabic[la]"),
            (Language::Dutch, "Dutch[la]"),
            (Language::Korean, "Korean[la]"),
            (Language::Polish, "Polish[la]"),
            (Language::Swedish, "Swedish[la]"),
            (Language::Danish, "Danish[la]"),
            (Language::Norwegian, "Norwegian[la]"),
            (Language::Finnish, "Finnish[la]"),
            (Language::Turkish, "Turkish[la]"),
            (Language::Hebrew, "Hebrew[la]"),
            (Language::Czech, "Czech[la]"),
            (Language::Hungarian, "Hungarian[la]"),
            (Language::Greek, "Greek[la]"),
        ];

        for (language, expected) in test_cases {
            assert_eq!(language.to_query_string(), expected);
        }
    }

    #[test]
    fn test_language_other_variant() {
        let custom_lang = Language::Other("Esperanto".to_string());
        assert_eq!(custom_lang.to_query_string(), "Esperanto[la]");

        let another_custom = Language::Other("Klingon".to_string());
        assert_eq!(another_custom.to_query_string(), "Klingon[la]");
    }

    #[test]
    fn test_article_type_equality() {
        assert_eq!(ArticleType::Review, ArticleType::Review);
        assert_ne!(ArticleType::Review, ArticleType::ClinicalTrial);
        assert_ne!(ArticleType::MetaAnalysis, ArticleType::SystematicReview);
    }

    #[test]
    fn test_language_equality() {
        assert_eq!(Language::English, Language::English);
        assert_ne!(Language::English, Language::Japanese);

        let other1 = Language::Other("Custom".to_string());
        let other2 = Language::Other("Custom".to_string());
        let other3 = Language::Other("Different".to_string());

        assert_eq!(other1, other2);
        assert_ne!(other1, other3);
        assert_ne!(Language::English, other1);
    }

    #[test]
    fn test_debug_formatting() {
        let article_type = ArticleType::Review;
        let debug_str = format!("{:?}", article_type);
        assert!(debug_str.contains("Review"));

        let language = Language::English;
        let debug_str = format!("{:?}", language);
        assert!(debug_str.contains("English"));

        let custom_lang = Language::Other("Test".to_string());
        let debug_str = format!("{:?}", custom_lang);
        assert!(debug_str.contains("Other"));
        assert!(debug_str.contains("Test"));
    }

    #[test]
    fn test_clone_functionality() {
        let original_type = ArticleType::MetaAnalysis;
        let cloned_type = original_type.clone();
        assert_eq!(original_type, cloned_type);
        assert_eq!(
            original_type.to_query_string(),
            cloned_type.to_query_string()
        );

        let original_lang = Language::German;
        let cloned_lang = original_lang.clone();
        assert_eq!(original_lang, cloned_lang);
        assert_eq!(
            original_lang.to_query_string(),
            cloned_lang.to_query_string()
        );

        let original_other = Language::Other("Custom".to_string());
        let cloned_other = original_other.clone();
        assert_eq!(original_other, cloned_other);
        assert_eq!(
            original_other.to_query_string(),
            cloned_other.to_query_string()
        );
    }

    #[test]
    fn test_language_other_empty_string() {
        let empty_lang = Language::Other("".to_string());
        assert_eq!(empty_lang.to_query_string(), "[la]");
    }

    #[test]
    fn test_language_other_special_characters() {
        let special_lang = Language::Other("中文-汉语".to_string());
        assert_eq!(special_lang.to_query_string(), "中文-汉语[la]");

        let symbol_lang = Language::Other("Lang@#$%".to_string());
        assert_eq!(symbol_lang.to_query_string(), "Lang@#$%[la]");
    }

    #[test]
    fn test_all_article_types_unique() {
        let all_types = vec![
            ArticleType::ClinicalTrial,
            ArticleType::Review,
            ArticleType::SystematicReview,
            ArticleType::MetaAnalysis,
            ArticleType::CaseReport,
            ArticleType::RandomizedControlledTrial,
            ArticleType::ObservationalStudy,
        ];

        let mut query_strings = Vec::new();
        for article_type in all_types {
            let query_string = article_type.to_query_string();
            assert!(
                !query_strings.contains(&query_string),
                "Duplicate query string found: {}",
                query_string
            );
            query_strings.push(query_string);
        }
    }

    #[test]
    fn test_sort_order_as_api_param() {
        assert_eq!(SortOrder::Relevance.as_api_param(), "relevance");
        assert_eq!(SortOrder::PublicationDate.as_api_param(), "pub_date");
        assert_eq!(SortOrder::FirstAuthor.as_api_param(), "Author");
        assert_eq!(SortOrder::JournalName.as_api_param(), "JournalName");
    }

    #[test]
    fn test_sort_order_equality() {
        assert_eq!(SortOrder::Relevance, SortOrder::Relevance);
        assert_ne!(SortOrder::Relevance, SortOrder::PublicationDate);
        assert_ne!(SortOrder::FirstAuthor, SortOrder::JournalName);
    }

    #[test]
    fn test_sort_order_clone() {
        let original = SortOrder::PublicationDate;
        let cloned = original.clone();
        assert_eq!(original, cloned);
        assert_eq!(original.as_api_param(), cloned.as_api_param());
    }

    #[test]
    fn test_all_standard_languages_unique() {
        let standard_languages = vec![
            Language::English,
            Language::Japanese,
            Language::German,
            Language::French,
            Language::Spanish,
            Language::Italian,
            Language::Chinese,
            Language::Russian,
            Language::Portuguese,
            Language::Arabic,
            Language::Dutch,
            Language::Korean,
            Language::Polish,
            Language::Swedish,
            Language::Danish,
            Language::Norwegian,
            Language::Finnish,
            Language::Turkish,
            Language::Hebrew,
            Language::Czech,
            Language::Hungarian,
            Language::Greek,
        ];

        let mut query_strings = Vec::new();
        for language in standard_languages {
            let query_string = language.to_query_string();
            assert!(
                !query_strings.contains(&query_string),
                "Duplicate query string found: {}",
                query_string
            );
            query_strings.push(query_string);
        }
    }
}
