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
            Language::Other(lang) => format!("{}[lang]", lang),
        }
    }
}
