use crate::error::{PubMedError, Result};
use crate::pmc::models::{Author, Reference};
use quick_xml::de::from_str;
use serde::Deserialize;

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

/// Extract detailed references from ref-list
pub fn extract_references_detailed(content: &str) -> Result<Vec<Reference>> {
    // Find ref-list content
    let ref_list_content = if let Some(start) = content.find("<ref-list") {
        if let Some(end) = content[start..].find("</ref-list>") {
            &content[start..start + end + 11] // +11 for "</ref-list>"
        } else {
            return Ok(Vec::new()); // No closing tag found, but not an error - just no references
        }
    } else {
        return Ok(Vec::new()); // No ref-list found, but not an error - just no references
    };

    // Parse the ref-list
    let ref_list =
        from_str::<RefList>(ref_list_content).map_err(|e| PubMedError::XmlParseError {
            message: format!("Failed to parse ref-list: {}", e),
        })?;

    let references = ref_list
        .refs
        .into_iter()
        .filter_map(parse_ref_to_reference)
        .collect();

    Ok(references)
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
        let content = "<ref-list><ref>Invalid XML</ref-list>";
        let result = extract_references_detailed(content);
        assert!(result.is_err());
    }
}
