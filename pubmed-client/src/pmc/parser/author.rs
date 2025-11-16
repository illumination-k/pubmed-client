use crate::error::{PubMedError, Result};
use crate::pmc::models::{Affiliation, Author};
use crate::pubmed::parser::strip_inline_html_tags;
use quick_xml::de::from_str;
use serde::Deserialize;

/// XML structure for contrib-group element
#[derive(Debug, Deserialize)]
#[serde(rename = "contrib-group")]
struct ContribGroup {
    #[serde(rename = "contrib", default)]
    contribs: Vec<Contrib>,
}

/// XML structure for contrib element
#[derive(Debug, Deserialize)]
struct Contrib {
    #[serde(rename = "@corresp", default)]
    corresp: Option<String>,

    #[serde(rename = "contrib-id", default)]
    contrib_ids: Vec<ContribId>,

    #[serde(rename = "name", default)]
    name: Option<Name>,

    #[serde(rename = "email", default)]
    email: Option<String>,

    #[serde(rename = "role", default)]
    roles: Vec<String>,

    #[serde(rename = "xref", default)]
    xrefs: Vec<Xref>,

    #[serde(rename = "aff", default)]
    affs: Vec<Aff>,
}

/// XML structure for contrib-id element
#[derive(Debug, Deserialize)]
struct ContribId {
    #[serde(rename = "@contrib-id-type")]
    contrib_id_type: Option<String>,

    #[serde(rename = "$text")]
    value: Option<String>,
}

/// XML structure for name element
#[derive(Debug, Deserialize)]
struct Name {
    #[serde(rename = "surname", default)]
    surname: Option<String>,

    #[serde(rename = "given-names", default)]
    given_names: Option<String>,
}

/// XML structure for xref element
#[derive(Debug, Deserialize)]
struct Xref {
    #[serde(rename = "@ref-type")]
    ref_type: Option<String>,

    #[serde(rename = "@rid")]
    rid: Option<String>,
}

/// XML structure for aff element
#[derive(Debug, Deserialize)]
struct Aff {
    #[serde(rename = "@id")]
    id: Option<String>,

    #[serde(rename = "$text", default)]
    text: Option<String>,

    #[serde(rename = "institution", default)]
    #[allow(dead_code)]
    institutions: Vec<String>,

    #[serde(rename = "addr-line", default)]
    #[allow(dead_code)]
    addr_lines: Vec<String>,

    #[serde(rename = "country", default)]
    #[allow(dead_code)]
    countries: Vec<String>,
}

/// XML structure for element-citation or mixed-citation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Citation {
    #[serde(rename = "person-group", default)]
    person_groups: Vec<PersonGroup>,
    #[serde(rename = "name", default)]
    names: Vec<Name>,
}

/// XML structure for person-group element
#[derive(Debug, Deserialize)]
#[serde(rename = "person-group")]
struct PersonGroup {
    #[serde(rename = "@person-group-type")]
    _group_type: Option<String>,
    #[serde(rename = "name", default)]
    names: Vec<Name>,
}

/// Extract authors from PMC XML content
pub fn extract_authors(content: &str) -> Result<Vec<Author>> {
    // Find and extract the contrib-group section
    if let Some(contrib_start) = content.find("<contrib-group>") {
        if let Some(contrib_end) = content[contrib_start..].find("</contrib-group>") {
            let contrib_section =
                &content[contrib_start..contrib_start + contrib_end + "</contrib-group>".len()];

            // Try to deserialize the contrib-group (strip inline HTML tags first)
            let cleaned_section = strip_inline_html_tags(contrib_section);
            match from_str::<ContribGroup>(&cleaned_section) {
                Ok(contrib_group) => {
                    let authors = contrib_group
                        .contribs
                        .into_iter()
                        .filter_map(parse_contrib_to_author)
                        .collect();
                    Ok(authors)
                }
                Err(e) => {
                    // Log the error but continue with empty authors rather than failing completely
                    tracing::warn!(
                        "Failed to parse contrib-group XML ({}), continuing with empty authors",
                        e
                    );
                    Ok(Vec::new())
                }
            }
        } else {
            Err(PubMedError::XmlError(
                "Found contrib-group start tag but no matching end tag".to_string(),
            ))
        }
    } else {
        // No contrib-group found - return empty vector as success
        Ok(Vec::new())
    }
}

/// Convert a Contrib to an Author
fn parse_contrib_to_author(contrib: Contrib) -> Option<Author> {
    let name = contrib.name?;

    let mut author = Author::with_names(name.surname.clone(), name.given_names.clone());

    // Extract ORCID from contrib-id tags
    for contrib_id in &contrib.contrib_ids {
        if let Some(id_type) = &contrib_id.contrib_id_type {
            if id_type == "orcid" {
                if let Some(value) = &contrib_id.value {
                    let clean_orcid = value.trim();
                    if clean_orcid.contains("orcid.org") || !clean_orcid.is_empty() {
                        author.orcid = Some(clean_orcid.to_string());
                        break;
                    }
                }
            }
        }
    }

    // Set email
    author.email = contrib.email.map(|e| e.trim().to_string());

    // Set corresponding author flag
    author.is_corresponding = contrib.corresp.map(|c| c == "yes").unwrap_or(false);

    // Set roles
    author.roles = contrib
        .roles
        .into_iter()
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .collect();

    // Extract affiliations from xrefs
    let mut affiliations = Vec::new();

    // Process xref affiliations
    for xref in &contrib.xrefs {
        if let Some(ref_type) = &xref.ref_type {
            if ref_type == "aff" {
                if let Some(rid) = &xref.rid {
                    affiliations.push(Affiliation {
                        id: Some(rid.clone()),
                        institution: Some(rid.clone()), // Use rid as institution for now
                        department: None,
                        address: None,
                        country: None,
                    });
                }
            }
        }
    }

    // Process direct affiliations
    for aff in &contrib.affs {
        if let Some(text) = &aff.text {
            let clean_text = text.trim();
            if !clean_text.is_empty() {
                affiliations.push(Affiliation {
                    id: aff.id.clone(),
                    institution: Some(clean_text.to_string()),
                    department: None,
                    address: None,
                    country: None,
                });
            }
        }
    }

    author.affiliations = affiliations;

    Some(author)
}

/// Extract authors from reference sections
pub fn extract_reference_authors(ref_content: &str) -> Result<Vec<Author>> {
    let mut authors = Vec::new();

    // Try to parse as element-citation
    if ref_content.contains("<element-citation") {
        if let Some(start) = ref_content.find("<element-citation") {
            if let Some(end) = ref_content[start..].find("</element-citation>") {
                let citation_content =
                    &ref_content[start..start + end + "</element-citation>".len()];
                let cleaned_citation = strip_inline_html_tags(citation_content);
                match from_str::<Citation>(&cleaned_citation) {
                    Ok(citation) => {
                        // Extract names from person-groups first
                        for person_group in citation.person_groups {
                            for name in person_group.names {
                                authors.push(Author::with_names(name.surname, name.given_names));
                            }
                        }
                        // Also check for direct names (without person-group wrapper)
                        for name in citation.names {
                            authors.push(Author::with_names(name.surname, name.given_names));
                        }
                        if !authors.is_empty() {
                            return Ok(authors);
                        }
                    }
                    Err(e) => {
                        return Err(PubMedError::XmlError(format!(
                            "Failed to parse element-citation XML: {}",
                            e
                        )));
                    }
                }
            } else {
                return Err(PubMedError::XmlError(
                    "Found element-citation start tag but no matching end tag".to_string(),
                ));
            }
        }
    }

    // Try to parse as mixed-citation
    if ref_content.contains("<mixed-citation") {
        if let Some(start) = ref_content.find("<mixed-citation") {
            if let Some(end) = ref_content[start..].find("</mixed-citation>") {
                let citation_content = &ref_content[start..start + end + "</mixed-citation>".len()];
                let cleaned_citation = strip_inline_html_tags(citation_content);
                match from_str::<Citation>(&cleaned_citation) {
                    Ok(citation) => {
                        // Extract names from person-groups first
                        for person_group in citation.person_groups {
                            for name in person_group.names {
                                authors.push(Author::with_names(name.surname, name.given_names));
                            }
                        }
                        // Also check for direct names (without person-group wrapper)
                        for name in citation.names {
                            authors.push(Author::with_names(name.surname, name.given_names));
                        }
                        if !authors.is_empty() {
                            return Ok(authors);
                        }
                    }
                    Err(e) => {
                        return Err(PubMedError::XmlError(format!(
                            "Failed to parse mixed-citation XML: {}",
                            e
                        )));
                    }
                }
            } else {
                return Err(PubMedError::XmlError(
                    "Found mixed-citation start tag but no matching end tag".to_string(),
                ));
            }
        }
    }

    // No citations found or no authors in citations - return empty vector as success
    Ok(authors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_authors_detailed() {
        let content = r#"
        <contrib-group>
            <contrib corresp="yes">
                <name>
                    <surname>Doe</surname>
                    <given-names>John</given-names>
                </name>
                <email>john.doe@example.com</email>
                <role>Principal Investigator</role>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].surname, Some("Doe".to_string()));
        assert_eq!(authors[0].given_names, Some("John".to_string()));
        assert!(authors[0].is_corresponding);
        assert_eq!(authors[0].email, Some("john.doe@example.com".to_string()));
        assert_eq!(authors[0].roles, vec!["Principal Investigator"]);
    }

    #[test]
    fn test_extract_reference_authors() {
        let content = r#"
        <element-citation>
            <name>
                <surname>Johnson</surname>
                <given-names>Alice</given-names>
            </name>
            <name>
                <surname>Williams</surname>
                <given-names>Bob</given-names>
            </name>
        </element-citation>
        "#;

        let authors = extract_reference_authors(content).unwrap();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].surname, Some("Johnson".to_string()));
        assert_eq!(authors[0].given_names, Some("Alice".to_string()));
        assert_eq!(authors[1].surname, Some("Williams".to_string()));
        assert_eq!(authors[1].given_names, Some("Bob".to_string()));
    }

    #[test]
    fn test_extract_orcid_from_contrib_id() {
        let content = r#"
        <contrib-group>
            <contrib corresp="yes">
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0002-3066-2940</contrib-id>
                <name name-style="western">
                    <surname>Doe</surname>
                    <given-names>John</given-names>
                </name>
                <email>john.doe@example.com</email>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].surname, Some("Doe".to_string()));
        assert_eq!(authors[0].given_names, Some("John".to_string()));
        assert_eq!(
            authors[0].orcid,
            Some("https://orcid.org/0000-0002-3066-2940".to_string())
        );
        assert!(authors[0].is_corresponding);
    }

    #[test]
    fn test_extract_orcid_with_xml_tags() {
        let content = r#"
        <contrib-group>
            <contrib>
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0001-2345-6789</contrib-id><name name-style="western">
                    <surname>Smith</surname>
                    <given-names>Jane</given-names>
                </name>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].surname, Some("Smith".to_string()));
        assert_eq!(authors[0].given_names, Some("Jane".to_string()));
        assert_eq!(
            authors[0].orcid,
            Some("https://orcid.org/0000-0001-2345-6789".to_string())
        );
        assert!(!authors[0].is_corresponding);
    }

    #[test]
    fn test_extract_multiple_authors_with_orcid() {
        let content = r#"
        <contrib-group>
            <contrib>
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0001-1111-1111</contrib-id>
                <name>
                    <surname>First</surname>
                    <given-names>Author</given-names>
                </name>
            </contrib>
            <contrib corresp="yes">
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0002-2222-2222</contrib-id>
                <name>
                    <surname>Second</surname>
                    <given-names>Author</given-names>
                </name>
            </contrib>
            <contrib>
                <name>
                    <surname>Third</surname>
                    <given-names>Author</given-names>
                </name>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 3);

        // First author with ORCID
        assert_eq!(authors[0].surname, Some("First".to_string()));
        assert_eq!(
            authors[0].orcid,
            Some("https://orcid.org/0000-0001-1111-1111".to_string())
        );
        assert!(!authors[0].is_corresponding);

        // Second author with ORCID and corresponding
        assert_eq!(authors[1].surname, Some("Second".to_string()));
        assert_eq!(
            authors[1].orcid,
            Some("https://orcid.org/0000-0002-2222-2222".to_string())
        );
        assert!(authors[1].is_corresponding);

        // Third author without ORCID
        assert_eq!(authors[2].surname, Some("Third".to_string()));
        assert_eq!(authors[2].orcid, None);
        assert!(!authors[2].is_corresponding);
    }
}
