use super::author::AuthorParser;
use super::xml_utils;
use crate::pmc::models::Reference;

/// Parser for extracting reference information from PMC XML content
pub struct ReferenceParser;

impl ReferenceParser {
    /// Extract detailed references from ref-list
    pub fn extract_references_detailed(content: &str) -> Vec<Reference> {
        let mut references = Vec::new();

        if let Some(ref_start) = content.find("<ref-list") {
            if let Some(ref_end) = content[ref_start..].find("</ref-list>") {
                let ref_content = &content[ref_start..ref_start + ref_end];

                let mut pos = 0;
                while let Some(ref_start) = ref_content[pos..].find("<ref id=\"") {
                    let ref_start = pos + ref_start;
                    if let Some(id_end) = ref_content[ref_start + 9..].find('"') {
                        let id = ref_content[ref_start + 9..ref_start + 9 + id_end].to_string();

                        if let Some(ref_end) = ref_content[ref_start..].find("</ref>") {
                            let ref_section = &ref_content[ref_start..ref_start + ref_end];

                            let reference = Self::parse_detailed_reference(ref_section, id);
                            references.push(reference);
                            pos = ref_start + ref_end;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        references
    }

    /// Parse detailed reference information
    fn parse_detailed_reference(ref_content: &str, id: String) -> Reference {
        let mut reference = Reference::new(id);

        // Extract title
        reference.title =
            xml_utils::extract_text_between(ref_content, "<article-title>", "</article-title>");

        // Extract journal
        reference.journal = xml_utils::extract_text_between(ref_content, "<source>", "</source>");

        // Extract year
        reference.year = xml_utils::extract_text_between(ref_content, "<year>", "</year>");

        // Extract volume
        reference.volume = xml_utils::extract_text_between(ref_content, "<volume>", "</volume>");

        // Extract issue
        reference.issue = xml_utils::extract_text_between(ref_content, "<issue>", "</issue>");

        // Extract pages
        if let Some(fpage) = xml_utils::extract_text_between(ref_content, "<fpage>", "</fpage>") {
            if let Some(lpage) = xml_utils::extract_text_between(ref_content, "<lpage>", "</lpage>")
            {
                reference.pages = Some(format!("{fpage}-{lpage}"));
            } else {
                reference.pages = Some(fpage);
            }
        }

        // Extract DOI
        reference.doi = xml_utils::extract_text_between(
            ref_content,
            "<pub-id pub-id-type=\"doi\">",
            "</pub-id>",
        );

        // Extract PMID
        reference.pmid = xml_utils::extract_text_between(
            ref_content,
            "<pub-id pub-id-type=\"pmid\">",
            "</pub-id>",
        );

        // Extract authors
        reference.authors = AuthorParser::extract_reference_authors(ref_content);

        // Determine reference type
        if ref_content.contains("<element-citation publication-type") {
            reference.ref_type =
                xml_utils::extract_attribute_value(ref_content, "publication-type");
        }

        reference
    }

    /// Extract simple reference text (fallback method)
    pub fn extract_simple_references(content: &str) -> Vec<String> {
        let mut references = Vec::new();

        if let Some(ref_start) = content.find("<ref-list") {
            if let Some(ref_end) = content[ref_start..].find("</ref-list>") {
                let ref_content = &content[ref_start..ref_start + ref_end];

                let ref_tags = xml_utils::find_all_tags(ref_content, "ref");
                for ref_tag in ref_tags {
                    if let Some(ref_text) = xml_utils::extract_element_content(&ref_tag, "ref") {
                        let clean_ref = xml_utils::strip_xml_tags(&ref_text);
                        if !clean_ref.trim().is_empty() {
                            references.push(clean_ref.trim().to_string());
                        }
                    }
                }
            }
        }

        references
    }

    /// Extract reference by ID
    pub fn extract_reference_by_id(content: &str, target_id: &str) -> Option<Reference> {
        let pattern = format!("<ref id=\"{}\">", target_id);

        if let Some(ref_start) = content.find(&pattern) {
            if let Some(ref_end) = content[ref_start..].find("</ref>") {
                let ref_content = &content[ref_start..ref_start + ref_end];
                return Some(Self::parse_detailed_reference(
                    ref_content,
                    target_id.to_string(),
                ));
            }
        }

        None
    }

    /// Extract all reference IDs from ref-list
    pub fn extract_reference_ids(content: &str) -> Vec<String> {
        let mut ids = Vec::new();

        if let Some(ref_start) = content.find("<ref-list") {
            if let Some(ref_end) = content[ref_start..].find("</ref-list>") {
                let ref_content = &content[ref_start..ref_start + ref_end];

                let mut pos = 0;
                while let Some(ref_start) = ref_content[pos..].find("<ref id=\"") {
                    let ref_start = pos + ref_start;
                    if let Some(id_end) = ref_content[ref_start + 9..].find('"') {
                        let id = ref_content[ref_start + 9..ref_start + 9 + id_end].to_string();
                        ids.push(id);
                        pos = ref_start + 9 + id_end;
                    } else {
                        break;
                    }
                }
            }
        }

        ids
    }

    /// Extract reference title
    pub fn extract_reference_title(ref_content: &str) -> Option<String> {
        xml_utils::extract_text_between(ref_content, "<article-title>", "</article-title>")
            .or_else(|| {
                xml_utils::extract_text_between(ref_content, "<chapter-title>", "</chapter-title>")
            })
            .or_else(|| xml_utils::extract_text_between(ref_content, "<source>", "</source>"))
    }

    /// Extract reference journal/source
    pub fn extract_reference_journal(ref_content: &str) -> Option<String> {
        xml_utils::extract_text_between(ref_content, "<source>", "</source>")
    }

    /// Extract reference publication year
    pub fn extract_reference_year(ref_content: &str) -> Option<String> {
        xml_utils::extract_text_between(ref_content, "<year>", "</year>")
    }

    /// Extract reference DOI
    pub fn extract_reference_doi(ref_content: &str) -> Option<String> {
        xml_utils::extract_text_between(ref_content, "<pub-id pub-id-type=\"doi\">", "</pub-id>")
    }

    /// Extract reference PMID
    pub fn extract_reference_pmid(ref_content: &str) -> Option<String> {
        xml_utils::extract_text_between(ref_content, "<pub-id pub-id-type=\"pmid\">", "</pub-id>")
    }

    /// Extract publication type from reference
    pub fn extract_publication_type(ref_content: &str) -> Option<String> {
        if ref_content.contains("<element-citation publication-type")
            || ref_content.contains("<mixed-citation publication-type")
        {
            xml_utils::extract_attribute_value(ref_content, "publication-type")
        } else {
            None
        }
    }

    /// Extract page range from reference
    pub fn extract_page_range(ref_content: &str) -> Option<String> {
        if let Some(fpage) = xml_utils::extract_text_between(ref_content, "<fpage>", "</fpage>") {
            if let Some(lpage) = xml_utils::extract_text_between(ref_content, "<lpage>", "</lpage>")
            {
                Some(format!("{fpage}-{lpage}"))
            } else {
                Some(fpage)
            }
        } else {
            // Try to extract page from a single page element
            xml_utils::extract_text_between(ref_content, "<page>", "</page>")
        }
    }

    /// Count total references in ref-list
    pub fn count_references(content: &str) -> usize {
        if let Some(ref_start) = content.find("<ref-list") {
            if let Some(ref_end) = content[ref_start..].find("</ref-list>") {
                let ref_content = &content[ref_start..ref_start + ref_end];
                return ref_content.matches("<ref id=\"").count();
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_references_detailed() {
        let content = r#"
        <ref-list>
            <ref id="ref1">
                <element-citation publication-type="journal">
                    <person-group person-group-type="author">
                        <name>
                            <surname>Smith</surname>
                            <given-names>J</given-names>
                        </name>
                    </person-group>
                    <article-title>Test Article</article-title>
                    <source>Test Journal</source>
                    <year>2023</year>
                    <volume>10</volume>
                    <issue>2</issue>
                    <fpage>123</fpage>
                    <lpage>130</lpage>
                    <pub-id pub-id-type="doi">10.1234/test</pub-id>
                </element-citation>
            </ref>
        </ref-list>
        "#;

        let references = ReferenceParser::extract_references_detailed(content);
        assert_eq!(references.len(), 1);

        let ref1 = &references[0];
        assert_eq!(ref1.id, "ref1");
        assert_eq!(ref1.title, Some("Test Article".to_string()));
        assert_eq!(ref1.journal, Some("Test Journal".to_string()));
        assert_eq!(ref1.year, Some("2023".to_string()));
        assert_eq!(ref1.volume, Some("10".to_string()));
        assert_eq!(ref1.issue, Some("2".to_string()));
        assert_eq!(ref1.pages, Some("123-130".to_string()));
        assert_eq!(ref1.doi, Some("10.1234/test".to_string()));
        assert_eq!(ref1.authors.len(), 1);
    }

    #[test]
    fn test_extract_reference_ids() {
        let content = r#"
        <ref-list>
            <ref id="ref1"><p>Reference 1</p></ref>
            <ref id="ref2"><p>Reference 2</p></ref>
            <ref id="ref3"><p>Reference 3</p></ref>
        </ref-list>
        "#;

        let ids = ReferenceParser::extract_reference_ids(content);
        assert_eq!(ids, vec!["ref1", "ref2", "ref3"]);
    }

    #[test]
    fn test_extract_reference_by_id() {
        let content = r#"
        <ref-list>
            <ref id="ref1">
                <element-citation>
                    <article-title>Specific Article</article-title>
                    <source>Specific Journal</source>
                    <year>2024</year>
                </element-citation>
            </ref>
            <ref id="ref2">
                <element-citation>
                    <article-title>Another Article</article-title>
                </element-citation>
            </ref>
        </ref-list>
        "#;

        let reference = ReferenceParser::extract_reference_by_id(content, "ref1");
        assert!(reference.is_some());

        let ref1 = reference.unwrap();
        assert_eq!(ref1.id, "ref1");
        assert_eq!(ref1.title, Some("Specific Article".to_string()));
        assert_eq!(ref1.journal, Some("Specific Journal".to_string()));
        assert_eq!(ref1.year, Some("2024".to_string()));

        // Test non-existent ID
        let no_ref = ReferenceParser::extract_reference_by_id(content, "ref999");
        assert!(no_ref.is_none());
    }

    #[test]
    fn test_count_references() {
        let content = r#"
        <ref-list>
            <ref id="ref1"><p>Reference 1</p></ref>
            <ref id="ref2"><p>Reference 2</p></ref>
        </ref-list>
        "#;

        assert_eq!(ReferenceParser::count_references(content), 2);

        let empty_content = "<body>No references here</body>";
        assert_eq!(ReferenceParser::count_references(empty_content), 0);
    }

    #[test]
    fn test_extract_page_range() {
        let content_with_range = r#"<fpage>123</fpage><lpage>130</lpage>"#;
        assert_eq!(
            ReferenceParser::extract_page_range(content_with_range),
            Some("123-130".to_string())
        );

        let content_single_page = r#"<fpage>123</fpage>"#;
        assert_eq!(
            ReferenceParser::extract_page_range(content_single_page),
            Some("123".to_string())
        );

        let content_no_pages = r#"<title>No pages here</title>"#;
        assert_eq!(ReferenceParser::extract_page_range(content_no_pages), None);
    }

    #[test]
    fn test_extract_publication_type() {
        let journal_ref =
            r#"<element-citation publication-type="journal">Content</element-citation>"#;
        assert_eq!(
            ReferenceParser::extract_publication_type(journal_ref),
            Some("journal".to_string())
        );

        let book_ref = r#"<mixed-citation publication-type="book">Content</mixed-citation>"#;
        assert_eq!(
            ReferenceParser::extract_publication_type(book_ref),
            Some("book".to_string())
        );

        let no_type = r#"<citation>Content</citation>"#;
        assert_eq!(ReferenceParser::extract_publication_type(no_type), None);
    }
}
