use pubmed_parser::common::PublicationDate;
use pubmed_parser::pmc::PmcArticle;
use serde::Serialize;

use super::entities::clean_content;

#[derive(Debug, Clone, Serialize)]
pub(super) struct ArticleMetadata {
    title: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    authors: Vec<String>,
    journal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    journal_abbrev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub_date: Option<String>,
    pmcid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pmid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    doi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    article_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    publisher: Option<String>,
}

pub(super) fn format_first_pub_date(dates: &[PublicationDate]) -> Option<String> {
    let d = dates.first()?;
    let year = d.year?;
    match (d.month, d.day) {
        (Some(m), Some(day)) => Some(format!("{year}-{m:02}-{day:02}")),
        (Some(m), None) => Some(format!("{year}-{m:02}")),
        _ => Some(year.to_string()),
    }
}

pub(super) fn generate_yaml_frontmatter(article: &PmcArticle) -> String {
    let metadata = ArticleMetadata {
        title: clean_content(article.title().unwrap_or("Untitled")),
        authors: article
            .authors()
            .iter()
            .map(|a| clean_content(&a.full_name))
            .collect(),
        journal: clean_content(
            article
                .journal()
                .title
                .as_deref()
                .unwrap_or("Unknown Journal"),
        ),
        journal_abbrev: article
            .journal()
            .abbreviation
            .as_ref()
            .map(|a| clean_content(a)),
        pub_date: format_first_pub_date(article.pub_dates()),
        pmcid: article.pmcid().as_str(),
        pmid: article.pmid().map(|p| p.as_str()),
        doi: article.doi().map(clean_content),
        article_type: article.article_type.as_ref().map(|t| clean_content(t)),
        keywords: article
            .keywords()
            .iter()
            .map(|k| clean_content(k))
            .collect(),
        volume: article.volume().map(clean_content),
        issue: article.issue().map(clean_content),
        publisher: article
            .journal()
            .publisher
            .as_ref()
            .map(|p| clean_content(p)),
    };

    match serde_yaml::to_string(&metadata) {
        Ok(yaml_content) => format!("---\n{}---\n", yaml_content),
        Err(e) => {
            tracing::warn!("Failed to serialize YAML frontmatter: {}", e);
            "---\n---\n".to_string()
        }
    }
}
