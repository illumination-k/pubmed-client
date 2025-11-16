//! PubMed XML parser module
//!
//! This module provides functionality for parsing PubMed EFetch XML responses into
//! structured article metadata. The parser handles complex XML structures including
//! authors, affiliations, MeSH terms, chemicals, and structured abstracts.
//!
//! # Module Organization
//!
//! - `preprocessing` - XML cleaning and preparation
//! - `deserializers` - Custom serde deserializers for complex fields
//! - `extractors` - Data extraction utilities (email, country, names)
//! - `xml_types` - Internal XML schema deserialization types
//! - `converters` - Conversion from XML types to public API models
//!
//! # Public API
//!
//! The main entry point is [`parse_article_from_xml`], which takes a PubMed EFetch
//! XML response and returns a [`PubMedArticle`].

mod converters;
mod deserializers;
mod extractors;
mod preprocessing;
mod xml_types;

// Re-export preprocessing function for use by PMC parser
pub(crate) use preprocessing::strip_inline_html_tags;

use crate::error::{PubMedError, Result};
use crate::pubmed::models::PubMedArticle;
use quick_xml::de::from_str;
use tracing::instrument;
use xml_types::PubmedArticleSet;

/// Parse article from EFetch XML response
///
/// Parses a PubMed EFetch XML response and extracts article metadata.
///
/// # Arguments
///
/// * `xml` - The raw XML string from PubMed EFetch API
/// * `pmid` - The PubMed ID of the article to extract
///
/// # Returns
///
/// A [`PubMedArticle`] containing the parsed metadata, or an error if parsing fails.
///
/// # Errors
///
/// Returns an error if:
/// - The XML is malformed or doesn't match the expected schema
/// - The specified PMID is not found in the XML
/// - Required fields (like article title) are missing
///
/// # Example
///
/// ```ignore
/// use pubmed_client_rs::pubmed::parser::parse_article_from_xml;
///
/// let xml = r#"<?xml version="1.0"?>
/// <PubmedArticleSet>
///   <PubmedArticle>
///     <MedlineCitation>
///       <PMID>12345678</PMID>
///       <Article>
///         <ArticleTitle>Example Article</ArticleTitle>
///         <Journal><Title>Example Journal</Title></Journal>
///       </Article>
///     </MedlineCitation>
///   </PubmedArticle>
/// </PubmedArticleSet>"#;
///
/// let article = parse_article_from_xml(xml, "12345678")?;
/// assert_eq!(article.title, "Example Article");
/// # Ok::<(), pubmed_client_rs::error::PubMedError>(())
/// ```
#[instrument(skip(xml), fields(pmid = %pmid, xml_size = xml.len()))]
pub fn parse_article_from_xml(xml: &str, pmid: &str) -> Result<PubMedArticle> {
    // Preprocess XML to remove inline HTML tags that can cause parsing issues
    // This handles tags like <i>, <sup>, <sub>, <b> that appear in abstracts and titles
    let cleaned_xml = strip_inline_html_tags(xml);

    // Parse the XML using quick-xml serde
    let article_set: PubmedArticleSet =
        from_str(&cleaned_xml).map_err(|e| PubMedError::XmlParseError {
            message: format!("Failed to deserialize XML: {}", e),
        })?;

    // Find the article with the matching PMID
    let article_xml = article_set
        .articles
        .into_iter()
        .find(|a| {
            a.medline_citation
                .pmid
                .as_ref()
                .is_some_and(|p| p.value == pmid)
        })
        .ok_or_else(|| PubMedError::ArticleNotFound {
            pmid: pmid.to_string(),
        })?;

    article_xml.into_article(pmid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_term_parsing() {
        let xml = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2023//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_230101.dtd">
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation Status="PubMed-not-MEDLINE" Owner="NLM">
        <PMID Version="1">12345678</PMID>
        <Article>
            <ArticleTitle>Test Article with MeSH Terms</ArticleTitle>
            <Abstract>
                <AbstractText>This is a test abstract.</AbstractText>
            </Abstract>
            <AuthorList>
                <Author>
                    <LastName>Doe</LastName>
                    <ForeName>John</ForeName>
                    <Initials>JA</Initials>
                    <AffiliationInfo>
                        <Affiliation>Department of Medicine, Harvard Medical School, Boston, MA, USA. john.doe@hms.harvard.edu</Affiliation>
                    </AffiliationInfo>
                    <Identifier Source="ORCID">0000-0001-2345-6789</Identifier>
                </Author>
            </AuthorList>
            <Journal>
                <Title>Test Journal</Title>
            </Journal>
        </Article>
        <MeshHeadingList>
            <MeshHeading>
                <DescriptorName UI="D003920" MajorTopicYN="Y">Diabetes Mellitus</DescriptorName>
                <QualifierName UI="Q000188" MajorTopicYN="N">drug therapy</QualifierName>
            </MeshHeading>
            <MeshHeading>
                <DescriptorName UI="D007333" MajorTopicYN="N">Insulin</DescriptorName>
            </MeshHeading>
        </MeshHeadingList>
        <ChemicalList>
            <Chemical>
                <RegistryNumber>11061-68-0</RegistryNumber>
                <NameOfSubstance UI="D007328">Insulin</NameOfSubstance>
            </Chemical>
        </ChemicalList>
        <KeywordList>
            <Keyword>diabetes treatment</Keyword>
            <Keyword>insulin therapy</Keyword>
        </KeywordList>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "12345678").unwrap();

        // Test MeSH headings
        assert!(article.mesh_headings.is_some());
        let mesh_headings = article.mesh_headings.as_ref().unwrap();
        assert_eq!(mesh_headings.len(), 2);

        // Test first MeSH heading (major topic with qualifier)
        let first_heading = &mesh_headings[0];
        assert_eq!(first_heading.mesh_terms.len(), 1);
        let diabetes_term = &first_heading.mesh_terms[0];
        assert_eq!(diabetes_term.descriptor_name, "Diabetes Mellitus");
        assert_eq!(diabetes_term.descriptor_ui, "D003920");
        assert!(diabetes_term.major_topic);
        assert_eq!(diabetes_term.qualifiers.len(), 1);
        assert_eq!(diabetes_term.qualifiers[0].qualifier_name, "drug therapy");
        assert_eq!(diabetes_term.qualifiers[0].qualifier_ui, "Q000188");
        assert!(!diabetes_term.qualifiers[0].major_topic);

        // Test second MeSH heading (non-major topic)
        let second_heading = &mesh_headings[1];
        assert_eq!(second_heading.mesh_terms.len(), 1);
        let insulin_term = &second_heading.mesh_terms[0];
        assert_eq!(insulin_term.descriptor_name, "Insulin");
        assert_eq!(insulin_term.descriptor_ui, "D007333");
        assert!(!insulin_term.major_topic);
        assert_eq!(insulin_term.qualifiers.len(), 0);

        // Test chemicals
        assert!(article.chemical_list.is_some());
        let chemicals = article.chemical_list.as_ref().unwrap();
        assert_eq!(chemicals.len(), 1);
        assert_eq!(chemicals[0].name, "Insulin");
        assert_eq!(chemicals[0].registry_number, Some("11061-68-0".to_string()));
        assert_eq!(chemicals[0].ui, Some("D007328".to_string()));

        // Test author parsing
        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.author_count, 1);
        let author = &article.authors[0];
        assert_eq!(author.surname, Some("Doe".to_string()));
        assert_eq!(author.given_names, Some("John".to_string()));
        assert_eq!(author.initials, Some("JA".to_string()));
        assert_eq!(author.full_name, "John Doe");
        assert_eq!(author.orcid, Some("0000-0001-2345-6789".to_string()));
        assert_eq!(author.affiliations.len(), 1);
        assert!(author.affiliations[0]
            .institution
            .as_ref()
            .unwrap()
            .contains("Harvard Medical School"));

        // Test keywords
        assert!(article.keywords.is_some());
        let keywords = article.keywords.as_ref().unwrap();
        assert_eq!(keywords.len(), 2);
        assert_eq!(keywords[0], "diabetes treatment");
        assert_eq!(keywords[1], "insulin therapy");
    }

    #[test]
    fn test_structured_abstract_parsing() {
        let xml = r#"
        <PubmedArticleSet>
            <PubmedArticle>
                <MedlineCitation>
                    <PMID>32887691</PMID>
                    <Article>
                        <ArticleTitle>A living WHO guideline on drugs for covid-19.</ArticleTitle>
                        <Abstract>
                            <AbstractText Label="UPDATES">This is the fourteenth version (thirteenth update) of the living guideline, replacing earlier versions.</AbstractText>
                            <AbstractText Label="CLINICAL QUESTION">What is the role of drugs in the treatment of patients with covid-19?</AbstractText>
                            <AbstractText Label="CONTEXT">The evidence base for therapeutics for covid-19 is evolving with numerous randomised controlled trials.</AbstractText>
                        </Abstract>
                        <Journal>
                            <Title>BMJ (Clinical research ed.)</Title>
                            <JournalIssue>
                                <PubDate>
                                    <Year>2020</Year>
                                    <Month>Sep</Month>
                                </PubDate>
                            </JournalIssue>
                        </Journal>
                    </Article>
                </MedlineCitation>
            </PubmedArticle>
        </PubmedArticleSet>"#;

        let result = parse_article_from_xml(xml, "32887691");
        assert!(result.is_ok());

        let article = result.unwrap();
        assert_eq!(article.pmid, "32887691");
        assert_eq!(
            article.title,
            "A living WHO guideline on drugs for covid-19."
        );

        // Verify that all three abstract sections are concatenated
        let abstract_text = article.abstract_text.unwrap();
        assert!(abstract_text.contains("This is the fourteenth version"));
        assert!(abstract_text.contains("What is the role of drugs"));
        assert!(abstract_text.contains("The evidence base for therapeutics"));

        // Verify they are properly concatenated with spaces
        assert!(abstract_text.contains("earlier versions. What is the role"));
        assert!(abstract_text.contains("covid-19? The evidence base"));
    }

    #[test]
    fn test_abstract_with_inline_html_tags() {
        // Test that abstracts with inline HTML tags (like <i>, <sub>, <sup>) parse successfully
        // without errors. This was causing CI failures in Python tests.
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>41111388</PMID>
        <Article>
            <ArticleTitle>Breath analysis with inline formatting</ArticleTitle>
            <Abstract>
                <AbstractText>This study presents a novel approach (<i>e.g.</i>, machine learning algorithms) for comprehensive analysis. The method uses H<sub>2</sub>O and CO<sub>2</sub> detection with sensitivity of 10<sup>-9</sup> parts per billion.</AbstractText>
            </Abstract>
            <Journal>
                <Title>Test Journal</Title>
                <JournalIssue>
                    <PubDate>
                        <Year>2024</Year>
                    </PubDate>
                </JournalIssue>
            </Journal>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        // The critical test: parsing should succeed without errors
        let result = parse_article_from_xml(xml, "41111388");
        assert!(
            result.is_ok(),
            "Failed to parse XML with inline HTML tags: {:?}",
            result
        );

        let article = result.unwrap();
        assert_eq!(article.pmid, "41111388");

        // Verify we extracted abstract text (even if some inline content might be lost)
        let abstract_text = article.abstract_text.as_ref();
        assert!(abstract_text.is_some(), "Abstract text should not be None");

        let text = abstract_text.unwrap();

        // Verify we get the main content (note: text from inline elements may be partially lost
        // due to quick-xml's mixed content handling, but we should get surrounding text)
        assert!(
            text.contains("machine learning algorithms"),
            "Abstract should contain main text content. Got: {}",
            text
        );
        assert!(
            text.contains("comprehensive analysis"),
            "Abstract should contain regular text. Got: {}",
            text
        );
        assert!(
            text.contains("parts per billion"),
            "Abstract should contain ending text. Got: {}",
            text
        );
    }

    #[test]
    fn test_structured_abstract_with_inline_tags() {
        // Test structured abstracts (with Label attributes) that also contain inline HTML tags
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>99999999</PMID>
        <Article>
            <ArticleTitle>Study with formatted abstract sections</ArticleTitle>
            <Abstract>
                <AbstractText Label="BACKGROUND">CRISPR-Cas systems (<i>e.g.</i>, Cas9) are revolutionary.</AbstractText>
                <AbstractText Label="METHODS">We used <sup>13</sup>C isotope labeling and analyzed pH levels between 5.0-7.5.</AbstractText>
                <AbstractText Label="RESULTS">Efficacy improved by 10<sup>3</sup>-fold with <i>in vitro</i> conditions.</AbstractText>
            </Abstract>
            <Journal>
                <Title>Test Journal</Title>
            </Journal>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let result = parse_article_from_xml(xml, "99999999");
        assert!(
            result.is_ok(),
            "Failed to parse structured abstract with inline tags"
        );

        let article = result.unwrap();
        let abstract_text = article.abstract_text.unwrap();

        // Verify key content from labeled sections was extracted
        assert!(
            abstract_text.contains("CRISPR-Cas systems"),
            "Should extract BACKGROUND content"
        );
        assert!(
            abstract_text.contains("Cas9"),
            "Should extract text adjacent to inline tags"
        );
        assert!(
            abstract_text.contains("isotope labeling"),
            "Should extract METHODS content"
        );

        // Verify multiple sections are present (sections should be concatenated)
        assert!(
            abstract_text.contains("revolutionary") && abstract_text.contains("isotope"),
            "Should concatenate all sections"
        );
    }

    #[test]
    fn test_article_without_mesh_terms() {
        let xml = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2023//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_230101.dtd">
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation Status="PubMed-not-MEDLINE" Owner="NLM">
        <PMID Version="1">87654321</PMID>
        <Article>
            <ArticleTitle>Article Without MeSH Terms</ArticleTitle>
            <AuthorList>
                <Author>
                    <LastName>Smith</LastName>
                    <ForeName>Jane</ForeName>
                </Author>
            </AuthorList>
            <Journal>
                <Title>Another Journal</Title>
            </Journal>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "87654321").unwrap();

        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.author_count, 1);
        assert_eq!(article.authors[0].full_name, "Jane Smith");
        assert!(article.mesh_headings.is_none());
        assert!(article.chemical_list.is_none());
        assert!(article.keywords.is_none());
    }
}
