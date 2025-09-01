use crate::error::Result;
use crate::pmc::models::PmcFullText;

pub mod author;
pub mod metadata;
pub mod reference;
pub mod section;
pub mod xml_utils;

/// Parse PMC XML content into structured data
///
/// This function acts as a coordinator that delegates parsing tasks
/// to specialized parser modules for better maintainability and separation of concerns.
pub fn parse_pmc_xml(xml_content: &str, pmcid: &str) -> Result<PmcFullText> {
    // Delegate to specialized parsers for clean separation of concerns

    // Extract metadata using metadata module functions
    let title = metadata::extract_title(xml_content);
    let journal = metadata::extract_journal_info(xml_content);
    let pub_date = metadata::extract_pub_date(xml_content);
    let doi = metadata::extract_doi(xml_content);
    let pmid = metadata::extract_pmid(xml_content);
    let article_type = metadata::extract_article_type(xml_content);
    let keywords = metadata::extract_keywords(xml_content);
    let funding = metadata::extract_funding(xml_content);
    let conflict_of_interest = metadata::extract_conflict_of_interest(xml_content);
    let acknowledgments = metadata::extract_acknowledgments(xml_content);
    let data_availability = metadata::extract_data_availability(xml_content);
    let supplementary_materials = metadata::extract_supplementary_materials(xml_content);

    // Extract authors
    let authors = author::extract_authors(xml_content)?;

    // Extract sections using section module functions
    let sections = section::extract_sections_enhanced(xml_content);

    // Extract references using reference module functions
    let references = reference::extract_references_detailed(xml_content).unwrap_or_default();

    Ok(PmcFullText {
        pmcid: pmcid.to_string(),
        pmid,
        title,
        authors,
        journal,
        pub_date,
        doi,
        sections,
        references,
        article_type,
        keywords,
        funding,
        conflict_of_interest,
        acknowledgments,
        data_availability,
        supplementary_materials,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_structure() {
        // Test that the parse method successfully delegates to specialized parsers
        let xml_content = r#"
        <article xmlns:xlink="http://www.w3.org/1999/xlink" article-type="research-article">
            <front>
                <article-meta>
                    <article-id pub-id-type="pmc">PMC123456</article-id>
                    <article-id pub-id-type="doi">10.1234/test</article-id>
                    <title-group>
                        <article-title>Test Article Title</article-title>
                    </title-group>
                    <contrib-group>
                        <contrib>
                            <name>
                                <surname>Doe</surname>
                                <given-names>John</given-names>
                            </name>
                        </contrib>
                    </contrib-group>
                    <pub-date>
                        <year>2023</year>
                        <month>12</month>
                        <day>25</day>
                    </pub-date>
                </article-meta>
            </front>
            <body>
                <sec>
                    <title>Introduction</title>
                    <p>This is the introduction.</p>
                </sec>
            </body>
            <back>
                <ref-list>
                    <ref id="ref1">
                        <element-citation>
                            <article-title>Reference Title</article-title>
                        </element-citation>
                    </ref>
                </ref-list>
            </back>
        </article>
        "#;

        let result = parse_pmc_xml(xml_content, "PMC123456");
        assert!(result.is_ok());

        let article = result.unwrap();
        assert_eq!(article.pmcid, "PMC123456");
        assert_eq!(article.title, "Test Article Title");
        assert_eq!(article.pub_date, "2023-12-25");
        assert!(!article.authors.is_empty());
        assert!(!article.sections.is_empty());
        assert!(!article.references.is_empty());
    }

    #[test]
    fn test_parse_minimal_xml() {
        // Test parsing with minimal XML structure
        let xml_content = r#"
        <article>
            <front>
                <article-meta>
                    <title-group>
                        <article-title>Minimal Test</article-title>
                    </title-group>
                </article-meta>
            </front>
        </article>
        "#;

        let result = parse_pmc_xml(xml_content, "PMC000000");
        assert!(result.is_ok());

        let article = result.unwrap();
        assert_eq!(article.pmcid, "PMC000000");
        assert_eq!(article.title, "Minimal Test");
    }

    // Note: Most detailed tests have been moved to the individual parser modules:
    // - AuthorParser tests in author_parser.rs
    // - section module functions tests in section.rs
    // - reference module functions tests in reference.rs
    // - metadata module functions tests in metadata.rs
    // - XmlUtils tests in xml_utils.rs
}
