use crate::error::Result;
use crate::pmc::models::{Author, Reference};
use crate::pubmed::parser::strip_inline_html_tags;
use quick_xml::de::from_str;
use serde::Deserialize;
use tracing;

/// XML structure for ref-list element
#[derive(Debug, Deserialize)]
#[serde(rename = "ref-list")]
struct RefList {
    #[serde(rename = "ref", default)]
    refs: Vec<Ref>,
}

/// XML structure for ref element
#[derive(Debug, Deserialize)]
struct Ref {
    #[serde(rename = "@id")]
    id: Option<String>,

    #[serde(rename = "element-citation", default)]
    element_citation: Option<ElementCitation>,

    #[serde(rename = "mixed-citation", default)]
    mixed_citation: Option<MixedCitation>,
}

/// XML structure for element-citation
#[derive(Debug, Deserialize)]
#[serde(rename = "element-citation")]
struct ElementCitation {
    #[serde(rename = "@publication-type")]
    publication_type: Option<String>,

    #[serde(rename = "article-title", default)]
    article_title: Option<String>,

    #[serde(rename = "source", default)]
    source: Option<String>,

    #[serde(rename = "year", default)]
    year: Option<String>,

    #[serde(rename = "volume", default)]
    volume: Option<String>,

    #[serde(rename = "issue", default)]
    issue: Option<String>,

    #[serde(rename = "fpage", default)]
    fpage: Option<String>,

    #[serde(rename = "lpage", default)]
    lpage: Option<String>,

    #[serde(rename = "pub-id", default)]
    pub_ids: Vec<PubId>,

    #[serde(rename = "person-group", default)]
    person_groups: Vec<PersonGroup>,
}

/// XML structure for mixed-citation (alternative citation format)
#[derive(Debug, Deserialize)]
#[serde(rename = "mixed-citation")]
struct MixedCitation {
    #[serde(rename = "@publication-type")]
    publication_type: Option<String>,

    #[serde(rename = "article-title", default)]
    article_title: Option<String>,

    #[serde(rename = "source", default)]
    source: Option<String>,

    #[serde(rename = "year", default)]
    year: Option<String>,

    #[serde(rename = "volume", default)]
    volume: Option<String>,

    #[serde(rename = "issue", default)]
    issue: Option<String>,

    #[serde(rename = "fpage", default)]
    fpage: Option<String>,

    #[serde(rename = "lpage", default)]
    lpage: Option<String>,

    #[serde(rename = "pub-id", default)]
    pub_ids: Vec<PubId>,

    #[serde(rename = "person-group", default)]
    person_groups: Vec<PersonGroup>,
}

/// XML structure for pub-id element
#[derive(Debug, Deserialize)]
struct PubId {
    #[serde(rename = "@pub-id-type")]
    pub_id_type: Option<String>,

    #[serde(rename = "$text")]
    value: Option<String>,
}

/// XML structure for person-group element
#[derive(Debug, Deserialize)]
#[serde(rename = "person-group")]
struct PersonGroup {
    #[serde(rename = "@person-group-type")]
    person_group_type: Option<String>,

    #[serde(rename = "name", default)]
    names: Vec<Name>,
}

/// XML structure for name element
#[derive(Debug, Deserialize)]
struct Name {
    #[serde(rename = "surname", default)]
    surname: Option<String>,

    #[serde(rename = "given-names", default)]
    given_names: Option<String>,
}

/// Extract detailed references from ref-list or alternative reference structures
pub fn extract_references_detailed(content: &str) -> Result<Vec<Reference>> {
    // Try multiple reference extraction strategies to handle different PMC XML formats

    // Strategy 1: Standard <ref-list> structure
    if let Some(references) = try_extract_from_ref_list(content)? {
        tracing::debug!(
            count = references.len(),
            "Extracted references from ref-list"
        );
        return Ok(references);
    }

    // Strategy 2: Alternative <references> structure
    if let Some(references) = try_extract_from_references_tag(content)? {
        tracing::debug!(
            count = references.len(),
            "Extracted references from references tag"
        );
        return Ok(references);
    }

    // Strategy 3: Direct <ref> tags in <back> section
    if let Some(references) = try_extract_from_back_section(content)? {
        tracing::debug!(
            count = references.len(),
            "Extracted references from back section"
        );
        return Ok(references);
    }

    // No references found with any strategy
    Ok(Vec::new())
}

/// Try to extract references from standard <ref-list> structure
fn try_extract_from_ref_list(content: &str) -> Result<Option<Vec<Reference>>> {
    let ref_list_content = if let Some(start) = content.find("<ref-list") {
        if let Some(end) = content[start..].find("</ref-list>") {
            &content[start..start + end + 11] // +11 for "</ref-list>"
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    // Parse the ref-list (strip inline HTML tags first)
    let cleaned_content = strip_inline_html_tags(ref_list_content);
    match from_str::<RefList>(&cleaned_content) {
        Ok(ref_list) => {
            let references = ref_list
                .refs
                .into_iter()
                .filter_map(parse_ref_to_reference)
                .collect();
            Ok(Some(references))
        }
        Err(_) => Ok(None),
    }
}

/// Try to extract references from alternative <references> structure
fn try_extract_from_references_tag(content: &str) -> Result<Option<Vec<Reference>>> {
    // Some PMC articles use <references> instead of <ref-list>
    let references_content = if let Some(start) = content.find("<references") {
        if let Some(end) = content[start..].find("</references>") {
            &content[start..start + end + 13] // +13 for "</references>"
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    // Try to adapt the content to ref-list format for parsing
    let adapted_content = references_content
        .replace("<references", "<ref-list")
        .replace("</references>", "</ref-list>");

    let cleaned_adapted = strip_inline_html_tags(&adapted_content);
    match from_str::<RefList>(&cleaned_adapted) {
        Ok(ref_list) => {
            let references = ref_list
                .refs
                .into_iter()
                .filter_map(parse_ref_to_reference)
                .collect();
            Ok(Some(references))
        }
        Err(_) => Ok(None),
    }
}

/// Try to extract references from direct <ref> tags in <back> section
fn try_extract_from_back_section(content: &str) -> Result<Option<Vec<Reference>>> {
    // Extract the back section
    let back_content = if let Some(start) = content.find("<back>") {
        if let Some(end) = content[start..].find("</back>") {
            &content[start..start + end + 7] // +7 for "</back>"
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    // Look for <ref> tags directly in the back section
    let mut references = Vec::new();
    let mut pos = 0;

    while let Some(ref_start) = back_content[pos..].find("<ref ") {
        let ref_start = pos + ref_start;
        if let Some(ref_end) = back_content[ref_start..].find("</ref>") {
            let ref_end = ref_start + ref_end + 6; // +6 for "</ref>"
            let ref_content = &back_content[ref_start..ref_end];

            // Wrap the ref in a temporary ref-list structure to reuse existing parsing
            let wrapped_content = format!("<ref-list>{}</ref-list>", ref_content);
            let cleaned_wrapped = strip_inline_html_tags(&wrapped_content);

            if let Ok(ref_list) = from_str::<RefList>(&cleaned_wrapped) {
                for ref_item in ref_list.refs {
                    if let Some(reference) = parse_ref_to_reference(ref_item) {
                        references.push(reference);
                    }
                }
            }

            pos = ref_end;
        } else {
            break;
        }
    }

    if references.is_empty() {
        Ok(None)
    } else {
        Ok(Some(references))
    }
}

/// Convert a Ref struct to a Reference model
fn parse_ref_to_reference(ref_elem: Ref) -> Option<Reference> {
    let id = ref_elem.id.unwrap_or_else(|| String::from("unknown"));
    let mut reference = Reference::new(id);

    // Try element-citation first, then mixed-citation
    let citation = ref_elem
        .element_citation
        .map(Citation::Element)
        .or_else(|| ref_elem.mixed_citation.map(Citation::Mixed));

    if let Some(citation) = citation {
        match citation {
            Citation::Element(elem) => {
                reference.ref_type = elem.publication_type;
                reference.title = elem.article_title;
                reference.journal = elem.source;
                reference.year = elem.year;
                reference.volume = elem.volume;
                reference.issue = elem.issue;
                reference.pages = format_pages(elem.fpage, elem.lpage);

                // Extract pub-ids
                for pub_id in elem.pub_ids {
                    if let (Some(id_type), Some(value)) = (pub_id.pub_id_type, pub_id.value) {
                        match id_type.as_str() {
                            "doi" => reference.doi = Some(value),
                            "pmid" => reference.pmid = Some(value),
                            _ => {}
                        }
                    }
                }

                // Extract authors
                reference.authors = extract_authors_from_person_groups(elem.person_groups);
            }
            Citation::Mixed(mixed) => {
                reference.ref_type = mixed.publication_type;
                reference.title = mixed.article_title;
                reference.journal = mixed.source;
                reference.year = mixed.year;
                reference.volume = mixed.volume;
                reference.issue = mixed.issue;
                reference.pages = format_pages(mixed.fpage, mixed.lpage);

                // Extract pub-ids
                for pub_id in mixed.pub_ids {
                    if let (Some(id_type), Some(value)) = (pub_id.pub_id_type, pub_id.value) {
                        match id_type.as_str() {
                            "doi" => reference.doi = Some(value),
                            "pmid" => reference.pmid = Some(value),
                            _ => {}
                        }
                    }
                }

                // Extract authors
                reference.authors = extract_authors_from_person_groups(mixed.person_groups);
            }
        }

        Some(reference)
    } else {
        None
    }
}

/// Helper enum to handle both citation types uniformly
enum Citation {
    Element(ElementCitation),
    Mixed(MixedCitation),
}

/// Format page range from first and last page
fn format_pages(fpage: Option<String>, lpage: Option<String>) -> Option<String> {
    match (fpage, lpage) {
        (Some(f), Some(l)) => Some(format!("{}-{}", f, l)),
        (Some(f), None) => Some(f),
        _ => None,
    }
}

/// Extract authors from person groups
fn extract_authors_from_person_groups(person_groups: Vec<PersonGroup>) -> Vec<Author> {
    let mut authors = Vec::new();

    for group in person_groups {
        // Only process author groups (not editor, etc.)
        if group.person_group_type.as_deref() == Some("author") || group.person_group_type.is_none()
        {
            for name in group.names {
                let author = Author::with_names(name.given_names.clone(), name.surname.clone());
                authors.push(author);
            }
        }
    }

    authors
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

        let references = extract_references_detailed(content).unwrap();
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
    fn test_extract_references_no_ref_list() {
        let content = "<article>No references here</article>";
        let references = extract_references_detailed(content).unwrap();
        assert_eq!(references.len(), 0);
    }

    #[test]
    fn test_extract_references_invalid_xml() {
        // The function is designed to be robust and handle malformed XML gracefully
        // by returning an empty vector instead of erroring. This test verifies that behavior.
        let content = "<ref-list><ref>Invalid XML</ref-list>";
        let result = extract_references_detailed(content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
