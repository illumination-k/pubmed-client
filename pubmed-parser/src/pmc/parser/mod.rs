use crate::error::Result;
use crate::pmc::models::PmcFullText;

pub mod author;
pub mod metadata;
pub(crate) mod reader_utils;
pub mod reference;
pub mod section;
pub mod xml_utils;

/// Extract a section slice from XML content without allocating.
///
/// Returns a `&str` slice covering `start_tag..end_tag` (inclusive),
/// or `None` if the tags are not found.
fn extract_section_slice<'a>(content: &'a str, start_tag: &str, end_tag: &str) -> Option<&'a str> {
    let start = content.find(start_tag)?;
    let end_offset = content[start..].find(end_tag)?;
    Some(&content[start..start + end_offset + end_tag.len()])
}

/// Parse PMC XML content into structured data
///
/// This function acts as a coordinator that delegates parsing tasks
/// to specialized parser modules for better maintainability and separation of concerns.
pub fn parse_pmc_xml(xml_content: &str, pmcid: &str) -> Result<PmcFullText> {
    // Pre-extract major XML sections once to avoid scanning the full document repeatedly.
    // PMC JATS XML structure: <article> <front>...</front> <body>...</body> <back>...</back> </article>
    let front = extract_section_slice(xml_content, "<front>", "</front>").unwrap_or(xml_content);
    let back = extract_section_slice(xml_content, "<back>", "</back>").unwrap_or("");

    // Metadata from <front> (title, journal, dates, IDs, keywords, funding are all in <front>)
    let title = metadata::extract_title(front);
    let journal = metadata::extract_journal_info(front);
    let pub_date = metadata::extract_pub_date(front);
    let doi = metadata::extract_doi(front);
    let pmid = metadata::extract_pmid(front);
    let keywords = metadata::extract_keywords(front);
    let funding = metadata::extract_funding(front);

    // Additional metadata from <front>
    let abstract_text = metadata::extract_abstract(front);
    let copyright = metadata::extract_copyright(front);
    let license = metadata::extract_license(front);
    let license_url = metadata::extract_license_url(front);
    let history_dates = metadata::extract_history_dates(front);
    let categories = metadata::extract_categories(front);
    let fpage = metadata::extract_fpage(front);
    let lpage = metadata::extract_lpage(front);
    let elocation_id = metadata::extract_elocation_id(front);

    // Article type is an attribute on the <article> tag itself (before <front>)
    let article_type = metadata::extract_article_type(xml_content);

    // Back matter
    let conflict_of_interest = metadata::extract_conflict_of_interest(back);
    let acknowledgments = metadata::extract_acknowledgments(back);

    // These can appear in body or back, so search full content
    let data_availability = metadata::extract_data_availability(xml_content);
    let supplementary_materials = metadata::extract_supplementary_materials(xml_content);

    // Authors from <front> (contrib-group is in article-meta)
    let authors = author::extract_authors(front)?;

    // Sections from <body> (extract_sections_enhanced finds <body> internally)
    let sections = section::extract_sections_enhanced(xml_content);

    // References from <back> (extract_references_detailed finds <ref-list>/<back> internally)
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
        abstract_text,
        copyright,
        license,
        license_url,
        history_dates,
        categories,
        fpage,
        lpage,
        elocation_id,
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
