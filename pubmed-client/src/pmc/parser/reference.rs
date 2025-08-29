use super::author;
use super::xml_utils;
use crate::pmc::models::Reference;

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

                        let reference = parse_detailed_reference(ref_section, id);
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
        if let Some(lpage) = xml_utils::extract_text_between(ref_content, "<lpage>", "</lpage>") {
            reference.pages = Some(format!("{fpage}-{lpage}"));
        } else {
            reference.pages = Some(fpage);
        }
    }

    // Extract DOI
    reference.doi =
        xml_utils::extract_text_between(ref_content, "<pub-id pub-id-type=\"doi\">", "</pub-id>");

    // Extract PMID
    reference.pmid =
        xml_utils::extract_text_between(ref_content, "<pub-id pub-id-type=\"pmid\">", "</pub-id>");

    // Extract authors
    reference.authors = author::extract_reference_authors(ref_content).unwrap_or_default();

    // Determine reference type
    if ref_content.contains("<element-citation publication-type") {
        reference.ref_type = xml_utils::extract_attribute_value(ref_content, "publication-type");
    }

    reference
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

        let references = extract_references_detailed(content);
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
}
