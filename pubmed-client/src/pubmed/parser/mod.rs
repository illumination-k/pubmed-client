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

mod batch;
mod converters;
mod deserializers;
mod extractors;
mod preprocessing;
mod xml_types;

// Re-export preprocessing function for use by PMC parser
pub(crate) use preprocessing::strip_inline_html_tags;

// Re-export batch parsing function
pub use batch::parse_articles_from_xml;

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
/// use pubmed_client::pubmed::parser::parse_article_from_xml;
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
/// # Ok::<(), pubmed_client::error::PubMedError>(())
/// ```
#[instrument(skip(xml), fields(pmid = %pmid, xml_size = xml.len()))]
pub fn parse_article_from_xml(xml: &str, pmid: &str) -> Result<PubMedArticle> {
    // Preprocess XML to remove inline HTML tags that can cause parsing issues
    // This handles tags like <i>, <sup>, <sub>, <b> that appear in abstracts and titles
    let cleaned_xml = strip_inline_html_tags(xml);

    // Parse the XML using quick-xml serde
    let article_set: PubmedArticleSet = from_str(&cleaned_xml)
        .map_err(|e| PubMedError::XmlError(format!("Failed to deserialize XML: {}", e)))?;

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

    // ==================================================================================
    // Bibliographic field tests
    //
    // Strategy to guard against silent None on Optional fields:
    //   (A) XML cross-check: if the raw XML contains a given element tag, the parsed
    //       field MUST be Some — catches parser regressions that silently drop data.
    //   (B) Statistical assertion: parse multiple articles that all have the fields in
    //       XML, assert 100% extraction rate — catches systematic failures.
    //   (C) Known-value assertion: hardcode expected values for specific test articles
    //       — catches incorrect extraction or value corruption.
    // ==================================================================================

    /// Cross-check helper: if a given XML element tag is present in the source XML,
    /// the corresponding parsed field must be Some (not silently dropped).
    fn assert_xml_field_extracted(
        xml: &str,
        xml_tag: &str,
        field_value: &Option<String>,
        field_name: &str,
    ) {
        let tag_open = format!("<{}", xml_tag);
        if xml.contains(&tag_open) {
            assert!(
                field_value.is_some(),
                "XML contains <{}> but parsed `{}` is None — parser silently dropped the field",
                xml_tag,
                field_name,
            );
        }
    }

    /// Apply cross-check to all 6 bibliographic fields at once.
    fn assert_bibliographic_fields_cross_check(xml: &str, article: &PubMedArticle) {
        assert_xml_field_extracted(xml, "Volume", &article.volume, "volume");
        assert_xml_field_extracted(xml, "Issue", &article.issue, "issue");
        assert_xml_field_extracted(xml, "MedlinePgn", &article.pages, "pages");
        assert_xml_field_extracted(xml, "Language", &article.language, "language");
        assert_xml_field_extracted(
            xml,
            "ISOAbbreviation",
            &article.journal_abbreviation,
            "journal_abbreviation",
        );
        // ISSN tag can appear in other contexts; only check within Journal
        if xml.contains("<ISSN") && xml.contains("<Journal>") {
            assert!(
                article.issn.is_some(),
                "XML contains <ISSN> within <Journal> but parsed `issn` is None",
            );
        }
    }

    #[test]
    fn test_bibliographic_fields_all_present() {
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>31978945</PMID>
        <Article>
            <Journal>
                <ISSN IssnType="Electronic">1476-4687</ISSN>
                <JournalIssue CitedMedium="Internet">
                    <Volume>579</Volume>
                    <Issue>7798</Issue>
                    <PubDate>
                        <Year>2020</Year>
                        <Month>Mar</Month>
                    </PubDate>
                </JournalIssue>
                <Title>Nature</Title>
                <ISOAbbreviation>Nature</ISOAbbreviation>
            </Journal>
            <ArticleTitle>A pneumonia outbreak associated with a new coronavirus of probable bat origin.</ArticleTitle>
            <Pagination>
                <MedlinePgn>270-273</MedlinePgn>
            </Pagination>
            <Language>eng</Language>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "31978945").unwrap();

        // (A) Cross-check: XML elements present → parsed field must be Some
        assert_bibliographic_fields_cross_check(xml, &article);

        // (C) Exact value assertions
        assert_eq!(article.volume.as_deref(), Some("579"));
        assert_eq!(article.issue.as_deref(), Some("7798"));
        assert_eq!(article.pages.as_deref(), Some("270-273"));
        assert_eq!(article.language.as_deref(), Some("eng"));
        assert_eq!(article.journal_abbreviation.as_deref(), Some("Nature"));
        assert_eq!(article.issn.as_deref(), Some("1476-4687"));
    }

    #[test]
    fn test_bibliographic_fields_all_absent() {
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>99990001</PMID>
        <Article>
            <Journal>
                <Title>Minimal Journal</Title>
            </Journal>
            <ArticleTitle>Minimal Article</ArticleTitle>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "99990001").unwrap();

        // Cross-check still holds (no XML tags → None is correct)
        assert_bibliographic_fields_cross_check(xml, &article);

        // All fields must actually be None
        assert!(article.volume.is_none());
        assert!(article.issue.is_none());
        assert!(article.pages.is_none());
        assert!(article.language.is_none());
        assert!(article.journal_abbreviation.is_none());
        assert!(article.issn.is_none());
    }

    #[test]
    fn test_bibliographic_fields_partial() {
        // Volume + ISOAbbreviation + ISSN + Language present; Issue + Pagination absent
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>99990002</PMID>
        <Article>
            <Journal>
                <ISSN IssnType="Print">0028-0836</ISSN>
                <JournalIssue>
                    <Volume>100</Volume>
                    <PubDate>
                        <Year>2023</Year>
                    </PubDate>
                </JournalIssue>
                <Title>Test Journal of Medicine</Title>
                <ISOAbbreviation>Test J Med</ISOAbbreviation>
            </Journal>
            <ArticleTitle>Partial Fields Article</ArticleTitle>
            <Language>jpn</Language>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "99990002").unwrap();

        // (A) Cross-check: present tags must yield Some
        assert_bibliographic_fields_cross_check(xml, &article);

        // (C) Exact values for present fields
        assert_eq!(article.volume.as_deref(), Some("100"));
        assert_eq!(article.language.as_deref(), Some("jpn"));
        assert_eq!(article.journal_abbreviation.as_deref(), Some("Test J Med"));
        assert_eq!(article.issn.as_deref(), Some("0028-0836"));

        // Absent fields must be None
        assert!(article.issue.is_none(), "No <Issue> in XML, must be None");
        assert!(
            article.pages.is_none(),
            "No <MedlinePgn> in XML, must be None"
        );
    }

    #[test]
    fn test_bibliographic_fields_batch_extraction_rate() {
        // (B) Statistical approach: parse 3 articles, all have all 6 fields in XML,
        //     assert 100% extraction rate.  If any field silently returns None,
        //     the count will be < 3 and the assertion fails.
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>10000001</PMID>
        <Article>
            <Journal>
                <ISSN IssnType="Electronic">1111-2222</ISSN>
                <JournalIssue>
                    <Volume>10</Volume>
                    <Issue>1</Issue>
                    <PubDate><Year>2020</Year></PubDate>
                </JournalIssue>
                <Title>Journal Alpha</Title>
                <ISOAbbreviation>J Alpha</ISOAbbreviation>
            </Journal>
            <ArticleTitle>Article One</ArticleTitle>
            <Pagination><MedlinePgn>1-10</MedlinePgn></Pagination>
            <Language>eng</Language>
        </Article>
    </MedlineCitation>
</PubmedArticle>
<PubmedArticle>
    <MedlineCitation>
        <PMID>10000002</PMID>
        <Article>
            <Journal>
                <ISSN IssnType="Print">3333-4444</ISSN>
                <JournalIssue>
                    <Volume>25</Volume>
                    <Issue>12</Issue>
                    <PubDate><Year>2021</Year><Month>Dec</Month></PubDate>
                </JournalIssue>
                <Title>Journal Beta</Title>
                <ISOAbbreviation>J Beta</ISOAbbreviation>
            </Journal>
            <ArticleTitle>Article Two</ArticleTitle>
            <Pagination><MedlinePgn>100-115</MedlinePgn></Pagination>
            <Language>fre</Language>
        </Article>
    </MedlineCitation>
</PubmedArticle>
<PubmedArticle>
    <MedlineCitation>
        <PMID>10000003</PMID>
        <Article>
            <Journal>
                <ISSN IssnType="Electronic">5555-6666</ISSN>
                <JournalIssue>
                    <Volume>8</Volume>
                    <Issue>4</Issue>
                    <PubDate><Year>2023</Year><Month>Apr</Month><Day>01</Day></PubDate>
                </JournalIssue>
                <Title>Journal Gamma</Title>
                <ISOAbbreviation>J Gamma</ISOAbbreviation>
            </Journal>
            <ArticleTitle>Article Three</ArticleTitle>
            <Pagination><MedlinePgn>e2023001</MedlinePgn></Pagination>
            <Language>jpn</Language>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        use crate::pubmed::parser::batch::parse_articles_from_xml;
        let articles = parse_articles_from_xml(xml).unwrap();
        assert_eq!(articles.len(), 3, "Should parse all 3 articles");

        let mut counts = [0u32; 6]; // volume, issue, pages, language, abbreviation, issn

        for article in &articles {
            if article.volume.is_some() {
                counts[0] += 1;
            }
            if article.issue.is_some() {
                counts[1] += 1;
            }
            if article.pages.is_some() {
                counts[2] += 1;
            }
            if article.language.is_some() {
                counts[3] += 1;
            }
            if article.journal_abbreviation.is_some() {
                counts[4] += 1;
            }
            if article.issn.is_some() {
                counts[5] += 1;
            }
        }

        let n = articles.len() as u32;
        let field_names = [
            "volume",
            "issue",
            "pages",
            "language",
            "journal_abbreviation",
            "issn",
        ];
        for (i, name) in field_names.iter().enumerate() {
            assert_eq!(
                counts[i], n,
                "All {} articles have <{}> in XML but only {} were extracted (expected {})",
                n, name, counts[i], n,
            );
        }

        // (C) Spot-check exact values on first and last article
        let a1 = articles.iter().find(|a| a.pmid == "10000001").unwrap();
        assert_eq!(a1.volume.as_deref(), Some("10"));
        assert_eq!(a1.issue.as_deref(), Some("1"));
        assert_eq!(a1.pages.as_deref(), Some("1-10"));
        assert_eq!(a1.language.as_deref(), Some("eng"));
        assert_eq!(a1.journal_abbreviation.as_deref(), Some("J Alpha"));
        assert_eq!(a1.issn.as_deref(), Some("1111-2222"));

        let a3 = articles.iter().find(|a| a.pmid == "10000003").unwrap();
        assert_eq!(a3.volume.as_deref(), Some("8"));
        assert_eq!(a3.issue.as_deref(), Some("4"));
        assert_eq!(a3.pages.as_deref(), Some("e2023001"));
        assert_eq!(a3.language.as_deref(), Some("jpn"));
        assert_eq!(a3.journal_abbreviation.as_deref(), Some("J Gamma"));
        assert_eq!(a3.issn.as_deref(), Some("5555-6666"));
    }

    #[test]
    fn test_bibliographic_fields_nlm_citation() {
        // End-to-end: parse → cross-check → construct NLM citation string
        let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation>
        <PMID>99990003</PMID>
        <Article>
            <Journal>
                <ISSN IssnType="Electronic">1234-5678</ISSN>
                <JournalIssue CitedMedium="Internet">
                    <Volume>45</Volume>
                    <Issue>3</Issue>
                    <PubDate>
                        <Year>2024</Year>
                        <Month>Jun</Month>
                        <Day>15</Day>
                    </PubDate>
                </JournalIssue>
                <Title>Journal of Biological Chemistry</Title>
                <ISOAbbreviation>J Biol Chem</ISOAbbreviation>
            </Journal>
            <ArticleTitle>Complete Citation Test Article.</ArticleTitle>
            <Pagination>
                <MedlinePgn>e100234</MedlinePgn>
            </Pagination>
            <AuthorList>
                <Author>
                    <LastName>Tanaka</LastName>
                    <ForeName>Yuki</ForeName>
                </Author>
                <Author>
                    <LastName>Suzuki</LastName>
                    <ForeName>Kenji</ForeName>
                </Author>
            </AuthorList>
            <Language>eng</Language>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "99990003").unwrap();

        // (A) Cross-check
        assert_bibliographic_fields_cross_check(xml, &article);

        // Construct NLM citation and verify
        let citation = format!(
            "{}. {}. {} {};{}({}):{}.",
            article.authors[0].full_name,
            article.title,
            article
                .journal_abbreviation
                .as_deref()
                .unwrap_or(&article.journal),
            "2024",
            article.volume.as_deref().unwrap_or(""),
            article.issue.as_deref().unwrap_or(""),
            article.pages.as_deref().unwrap_or(""),
        );
        assert_eq!(
            citation,
            "Yuki Tanaka. Complete Citation Test Article.. J Biol Chem 2024;45(3):e100234."
        );
    }

    #[test]
    fn test_parse_article_with_abstract() {
        let xml = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2025//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_250101.dtd">
<PubmedArticleSet>
<PubmedArticle>
<MedlineCitation Status="MEDLINE" Owner="NLM" IndexingMethod="Manual">
<PMID Version="1">31978945</PMID>
<Article PubModel="Print-Electronic">
<Journal>
<Title>The New England journal of medicine</Title>
</Journal>
<ArticleTitle>A Novel Coronavirus from Patients with Pneumonia in China, 2019.</ArticleTitle>
<Abstract>
<AbstractText>In December 2019, a cluster of patients with pneumonia of unknown cause was linked to a seafood wholesale market in Wuhan, China. A previously unknown betacoronavirus was discovered through the use of unbiased sequencing in samples from patients with pneumonia. Human airway epithelial cells were used to isolate a novel coronavirus, named 2019-nCoV, which formed a clade within the subgenus sarbecovirus, Orthocoronavirinae subfamily. Different from both MERS-CoV and SARS-CoV, 2019-nCoV is the seventh member of the family of coronaviruses that infect humans. Enhanced surveillance and further investigation are ongoing. (Funded by the National Key Research and Development Program of China and the National Major Project for Control and Prevention of Infectious Disease in China.).</AbstractText>
</Abstract>
<AuthorList CompleteYN="Y">
<Author ValidYN="Y">
<LastName>Zhu</LastName>
<ForeName>Na</ForeName>
</Author>
<Author ValidYN="Y">
<LastName>Zhang</LastName>
<ForeName>Dingyu</ForeName>
</Author>
</AuthorList>
<PublicationTypeList>
<PublicationType UI="D016428">Journal Article</PublicationType>
</PublicationTypeList>
</Article>
</MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "31978945").unwrap();

        assert_eq!(article.pmid, "31978945");
        assert_eq!(
            article.title,
            "A Novel Coronavirus from Patients with Pneumonia in China, 2019."
        );
        assert_eq!(article.journal, "The New England journal of medicine");
        assert_eq!(article.authors.len(), 2);
        assert_eq!(article.authors[0].full_name, "Na Zhu");
        assert_eq!(article.authors[1].full_name, "Dingyu Zhang");
        assert_eq!(article.article_types, vec!["Journal Article"]);

        assert!(article.abstract_text.is_some());
        let abstract_text = article.abstract_text.unwrap();
        assert!(abstract_text.contains("In December 2019"));
        assert!(abstract_text.contains("2019-nCoV"));
        assert!(
            abstract_text.contains("Enhanced surveillance and further investigation are ongoing")
        );
    }

    #[test]
    fn test_parse_article_without_abstract() {
        let xml = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2025//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_250101.dtd">
<PubmedArticleSet>
<PubmedArticle>
<MedlineCitation Status="MEDLINE" Owner="NLM" IndexingMethod="Manual">
<PMID Version="1">33515491</PMID>
<Article PubModel="Print-Electronic">
<Journal>
<Title>Lancet (London, England)</Title>
</Journal>
<ArticleTitle>Resurgence of COVID-19 in Manaus, Brazil, despite high seroprevalence.</ArticleTitle>
<AuthorList CompleteYN="Y">
<Author ValidYN="Y">
<LastName>Sabino</LastName>
<ForeName>Ester C</ForeName>
</Author>
</AuthorList>
<PublicationTypeList>
<PublicationType UI="D016428">Journal Article</PublicationType>
</PublicationTypeList>
</Article>
</MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "33515491").unwrap();

        assert_eq!(article.pmid, "33515491");
        assert_eq!(
            article.title,
            "Resurgence of COVID-19 in Manaus, Brazil, despite high seroprevalence."
        );
        assert_eq!(article.journal, "Lancet (London, England)");
        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.authors[0].full_name, "Ester C Sabino");
        assert!(article.abstract_text.is_none());
    }

    #[test]
    fn test_parse_invalid_xml() {
        let invalid_xml = "<invalid>xml</not_closed>";
        let result = parse_article_from_xml(invalid_xml, "12345");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_xml() {
        let empty_xml = r#"<?xml version="1.0" ?>
    <PubmedArticleSet>
    </PubmedArticleSet>"#;
        let result = parse_article_from_xml(empty_xml, "12345");

        assert!(
            matches!(
                result,
                Err(PubMedError::ArticleNotFound { ref pmid }) if pmid == "12345"
            ),
            "Expected ArticleNotFound error for PMID 12345"
        );
    }
}
