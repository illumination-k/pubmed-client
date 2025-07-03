//! Filter types and enums for PubMed query filtering

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
    pub(crate) fn to_query_string(&self) -> String {
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
            Language::Other(lang) => format!("{lang}[lang]"),
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
            (Language::English, "English[lang]"),
            (Language::Japanese, "Japanese[lang]"),
            (Language::German, "German[lang]"),
            (Language::French, "French[lang]"),
            (Language::Spanish, "Spanish[lang]"),
            (Language::Italian, "Italian[lang]"),
            (Language::Chinese, "Chinese[lang]"),
            (Language::Russian, "Russian[lang]"),
            (Language::Portuguese, "Portuguese[lang]"),
            (Language::Arabic, "Arabic[lang]"),
            (Language::Dutch, "Dutch[lang]"),
            (Language::Korean, "Korean[lang]"),
            (Language::Polish, "Polish[lang]"),
            (Language::Swedish, "Swedish[lang]"),
            (Language::Danish, "Danish[lang]"),
            (Language::Norwegian, "Norwegian[lang]"),
            (Language::Finnish, "Finnish[lang]"),
            (Language::Turkish, "Turkish[lang]"),
            (Language::Hebrew, "Hebrew[lang]"),
            (Language::Czech, "Czech[lang]"),
            (Language::Hungarian, "Hungarian[lang]"),
            (Language::Greek, "Greek[lang]"),
        ];

        for (language, expected) in test_cases {
            assert_eq!(language.to_query_string(), expected);
        }
    }

    #[test]
    fn test_language_other_variant() {
        let custom_lang = Language::Other("Esperanto".to_string());
        assert_eq!(custom_lang.to_query_string(), "Esperanto[lang]");

        let another_custom = Language::Other("Klingon".to_string());
        assert_eq!(another_custom.to_query_string(), "Klingon[lang]");
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
        assert_eq!(empty_lang.to_query_string(), "[lang]");
    }

    #[test]
    fn test_language_other_special_characters() {
        let special_lang = Language::Other("中文-汉语".to_string());
        assert_eq!(special_lang.to_query_string(), "中文-汉语[lang]");

        let symbol_lang = Language::Other("Lang@#$%".to_string());
        assert_eq!(symbol_lang.to_query_string(), "Lang@#$%[lang]");
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
