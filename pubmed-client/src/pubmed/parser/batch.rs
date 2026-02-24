//! Batch parsing for multiple PubMed articles
//!
//! This module provides functionality for parsing multiple PubMed articles
//! from a single EFetch XML response, typically used with the history server.

use super::preprocessing::strip_inline_html_tags;
use super::xml_types::PubmedArticleSet;
use crate::error::{PubMedError, Result};
use crate::pubmed::models::PubMedArticle;
use quick_xml::de::from_str;
use tracing::{instrument, warn};

/// Parse multiple PubMed articles from EFetch XML response
///
/// This function parses an XML response containing multiple `<PubmedArticle>` elements,
/// typically returned when fetching from the NCBI history server.
///
/// # Arguments
///
/// * `xml` - The raw XML string from PubMed EFetch API containing multiple articles
///
/// # Returns
///
/// A `Result<Vec<PubMedArticle>>` containing all successfully parsed articles.
/// Articles that fail to parse are logged and skipped.
///
/// # Example
///
/// ```ignore
/// use pubmed_client::pubmed::parser::parse_articles_from_xml;
///
/// let xml = r#"<?xml version="1.0"?>
/// <PubmedArticleSet>
///   <PubmedArticle>...</PubmedArticle>
///   <PubmedArticle>...</PubmedArticle>
/// </PubmedArticleSet>"#;
///
/// let articles = parse_articles_from_xml(xml)?;
/// println!("Parsed {} articles", articles.len());
/// ```
#[instrument(skip(xml), fields(xml_size = xml.len()))]
pub fn parse_articles_from_xml(xml: &str) -> Result<Vec<PubMedArticle>> {
    // Preprocess XML to remove inline HTML tags that can cause parsing issues
    let cleaned_xml = strip_inline_html_tags(xml);

    // Parse the XML using quick-xml serde
    let article_set: PubmedArticleSet = from_str(&cleaned_xml)
        .map_err(|e| PubMedError::XmlError(format!("Failed to deserialize XML: {}", e)))?;

    // Convert all articles, skipping those that fail
    let articles: Vec<PubMedArticle> = article_set
        .articles
        .into_iter()
        .filter_map(|article_xml| {
            let pmid = article_xml
                .medline_citation
                .pmid
                .as_ref()
                .map(|p| p.value.clone())?;

            match article_xml.into_article(&pmid) {
                Ok(article) => Some(article),
                Err(e) => {
                    warn!(pmid = %pmid, error = %e, "Failed to parse article, skipping");
                    None
                }
            }
        })
        .collect();

    Ok(articles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multiple_articles() {
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>12345678</PMID>
        <Article>
            <ArticleTitle>First Article</ArticleTitle>
            <Journal><Title>Journal One</Title></Journal>
        </Article>
    </MedlineCitation>
</PubmedArticle>
<PubmedArticle>
    <MedlineCitation>
        <PMID>87654321</PMID>
        <Article>
            <ArticleTitle>Second Article</ArticleTitle>
            <Journal><Title>Journal Two</Title></Journal>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let articles = parse_articles_from_xml(xml).unwrap();
        assert_eq!(articles.len(), 2);
        assert_eq!(articles[0].pmid, "12345678");
        assert_eq!(articles[0].title, "First Article");
        assert_eq!(articles[1].pmid, "87654321");
        assert_eq!(articles[1].title, "Second Article");
    }

    #[test]
    fn test_parse_empty_set() {
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
</PubmedArticleSet>"#;

        let articles = parse_articles_from_xml(xml).unwrap();
        assert!(articles.is_empty());
    }

    #[test]
    fn test_parse_with_inline_html() {
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>11111111</PMID>
        <Article>
            <ArticleTitle>Article with <i>italic</i> text</ArticleTitle>
            <Abstract>
                <AbstractText>Abstract with H<sub>2</sub>O formula</AbstractText>
            </Abstract>
            <Journal><Title>Test Journal</Title></Journal>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let articles = parse_articles_from_xml(xml).unwrap();
        assert_eq!(articles.len(), 1);
        assert!(articles[0].title.contains("italic"));
    }
}
