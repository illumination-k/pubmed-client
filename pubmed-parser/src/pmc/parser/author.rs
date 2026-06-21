use crate::common::xml_utils::strip_inline_html_tags;
use crate::common::{Affiliation, Author};
use crate::error::{ParseError, Result};
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
    #[serde(rename = "@name-style", default)]
    #[allow(dead_code)]
    name_style: Option<String>,

    #[serde(rename = "surname", default)]
    surname: Option<String>,

    #[serde(rename = "given-names", default)]
    given_names: Option<String>,

    #[serde(rename = "suffix", default)]
    suffix: Option<String>,
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

/// Extract authors from PMC XML content
pub(crate) fn extract_authors(content: &str) -> Result<Vec<Author>> {
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
            Err(ParseError::XmlError(
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

    let mut author = Author::new(name.surname.clone(), name.given_names.clone());
    author.suffix = name.suffix;

    // Extract ORCID from contrib-id tags
    for contrib_id in &contrib.contrib_ids {
        if let Some(id_type) = &contrib_id.contrib_id_type
            && id_type == "orcid"
            && let Some(value) = &contrib_id.value
        {
            let clean_orcid = value.trim();
            if clean_orcid.contains("orcid.org") || !clean_orcid.is_empty() {
                author.orcid = Some(clean_orcid.to_string());
                break;
            }
        }
    }

    // Set email
    author.email = contrib.email.map(|e| e.trim().to_string());

    // Set corresponding author flag (check both corresp="yes" attribute and <xref ref-type="corresp">)
    author.is_corresponding = contrib.corresp.map(|c| c == "yes").unwrap_or(false)
        || contrib
            .xrefs
            .iter()
            .any(|x| x.ref_type.as_deref() == Some("corresp"));

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
        if let Some(ref_type) = &xref.ref_type
            && ref_type == "aff"
            && let Some(rid) = &xref.rid
        {
            affiliations.push(Affiliation {
                id: Some(rid.clone()),
                institution: Some(rid.clone()), // Use rid as institution for now
                department: None,
                address: None,
                country: None,
            });
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
