use super::xml_utils;
use crate::pmc::models::{Affiliation, Author};

/// Parser for extracting author information from PMC XML content
pub struct AuthorParser;

impl AuthorParser {
    /// Extract simple author names from contributor group (legacy method for compatibility)
    pub fn extract_authors(content: &str) -> Vec<String> {
        let mut authors = Vec::new();

        if let Some(contrib_start) = content.find("<contrib-group>") {
            if let Some(contrib_end) = content[contrib_start..].find("</contrib-group>") {
                let contrib_section = &content[contrib_start..contrib_start + contrib_end];

                let mut pos = 0;
                while let Some(surname_start) = contrib_section[pos..].find("<surname>") {
                    let surname_start = pos + surname_start + 9;
                    if let Some(surname_end) = contrib_section[surname_start..].find("</surname>") {
                        let surname_end = surname_start + surname_end;
                        let surname = &contrib_section[surname_start..surname_end];

                        if let Some(given_start) =
                            contrib_section[surname_end..].find("<given-names")
                        {
                            let given_start = surname_end + given_start;
                            if let Some(given_content_start) =
                                contrib_section[given_start..].find(">")
                            {
                                let given_content_start = given_start + given_content_start + 1;
                                if let Some(given_end) =
                                    contrib_section[given_content_start..].find("</given-names>")
                                {
                                    let given_end = given_content_start + given_end;
                                    let given_names =
                                        &contrib_section[given_content_start..given_end];
                                    authors.push(format!("{given_names} {surname}"));
                                    pos = given_end;
                                    continue;
                                }
                            }
                        }

                        authors.push(surname.to_string());
                        pos = surname_end;
                    } else {
                        break;
                    }
                }
            }
        }

        authors
    }

    /// Extract detailed author information with affiliations and ORCID
    pub fn extract_authors_detailed(content: &str) -> Vec<Author> {
        let mut authors = Vec::new();

        if let Some(contrib_start) = content.find("<contrib-group>") {
            if let Some(contrib_end) = content[contrib_start..].find("</contrib-group>") {
                let contrib_section = &content[contrib_start..contrib_start + contrib_end];

                let mut pos = 0;
                while let Some(contrib_start) = contrib_section[pos..].find("<contrib") {
                    let contrib_start = pos + contrib_start;
                    if let Some(contrib_end) = contrib_section[contrib_start..].find("</contrib>") {
                        let contrib_end = contrib_start + contrib_end;
                        let contrib_content = &contrib_section[contrib_start..contrib_end];

                        if let Some(author) = Self::parse_single_author(contrib_content) {
                            authors.push(author);
                        }

                        pos = contrib_end;
                    } else {
                        break;
                    }
                }
            }
        }

        // Fallback to simple author extraction if detailed extraction fails
        if authors.is_empty() {
            let simple_authors = Self::extract_authors(content);
            authors = simple_authors.into_iter().map(Author::new).collect();
        }

        authors
    }

    /// Parse a single author from contrib XML
    fn parse_single_author(contrib_content: &str) -> Option<Author> {
        let surname = xml_utils::extract_text_between(contrib_content, "<surname>", "</surname>");
        let given_names =
            xml_utils::extract_text_between(contrib_content, "<given-names>", "</given-names>")
                .or_else(|| {
                    // Handle self-closing given-names tag
                    if let Some(start) = contrib_content.find("<given-names") {
                        if let Some(end) = contrib_content[start..].find(">") {
                            let tag_content = &contrib_content[start..start + end + 1];
                            if tag_content.contains("/>") {
                                return None; // Self-closing tag with no content
                            }
                        }
                    }
                    None
                });

        let mut author = Author::with_names(given_names, surname);

        // Extract ORCID from contrib-id tags
        let contrib_id_tags = xml_utils::find_all_tags(contrib_content, "contrib-id");
        for tag in contrib_id_tags {
            if tag.contains("contrib-id-type=\"orcid\"") || tag.contains("https://orcid.org/") {
                if let Some(orcid_content) = xml_utils::extract_element_content(&tag, "contrib-id")
                {
                    let clean_orcid = xml_utils::strip_xml_tags(&orcid_content);
                    if clean_orcid.contains("https://orcid.org/") {
                        author.orcid = Some(clean_orcid.trim().to_string());
                        break;
                    }
                }
            }
        }

        // Extract email
        author.email = xml_utils::extract_text_between(contrib_content, "<email", "</email>")
            .and_then(|email_content| {
                // Extract actual email from the tag content
                email_content
                    .find(">")
                    .map(|start| email_content[start + 1..].to_string())
            });

        // Check if corresponding author
        author.is_corresponding = contrib_content.contains("corresp=\"yes\"");

        // Extract roles
        let mut roles = Vec::new();
        let mut pos = 0;
        while let Some(role_start) = contrib_content[pos..].find("<role") {
            let role_start = pos + role_start;
            if let Some(role_end) = contrib_content[role_start..].find("</role>") {
                let role_end = role_start + role_end;
                let role_section = &contrib_content[role_start..role_end];

                if let Some(content_start) = role_section.find(">") {
                    let role_content = &role_section[content_start + 1..];
                    if !role_content.trim().is_empty() {
                        roles.push(role_content.trim().to_string());
                    }
                }

                pos = role_end;
            } else {
                break;
            }
        }

        author.roles = roles;

        // Extract affiliations
        author.affiliations = Self::extract_affiliations(contrib_content);

        Some(author)
    }

    /// Extract affiliations from author contribution content
    fn extract_affiliations(contrib_content: &str) -> Vec<Affiliation> {
        let mut affiliations = Vec::new();

        // Look for xref elements that reference affiliations
        let xref_tags = xml_utils::find_all_tags(contrib_content, "xref");
        for xref_tag in xref_tags {
            if xref_tag.contains("ref-type=\"aff\"") {
                if let Some(rid) = xml_utils::extract_attribute_value(&xref_tag, "rid") {
                    let affiliation = Affiliation {
                        id: Some(rid.clone()),
                        institution: rid, // Use rid as institution for now
                        department: None,
                        address: None,
                        country: None,
                    };
                    affiliations.push(affiliation);
                }
            }
        }

        // Also look for direct affiliation content
        let aff_tags = xml_utils::find_all_tags(contrib_content, "aff");
        for aff_tag in aff_tags {
            if let Some(aff_content) = xml_utils::extract_element_content(&aff_tag, "aff") {
                let clean_aff = xml_utils::strip_xml_tags(&aff_content);
                if !clean_aff.trim().is_empty() {
                    let affiliation = Affiliation {
                        id: xml_utils::extract_attribute_value(&aff_tag, "id"),
                        institution: clean_aff.trim().to_string(),
                        department: None,
                        address: None,
                        country: None,
                    };
                    affiliations.push(affiliation);
                }
            }
        }

        affiliations
    }

    /// Extract authors from reference sections
    pub fn extract_reference_authors(ref_content: &str) -> Vec<Author> {
        let mut authors = Vec::new();

        let mut pos = 0;
        while let Some(name_start) = ref_content[pos..].find("<name>") {
            let name_start = pos + name_start;
            if let Some(name_end) = ref_content[name_start..].find("</name>") {
                let name_end = name_start + name_end;
                let name_content = &ref_content[name_start..name_end];

                let surname =
                    xml_utils::extract_text_between(name_content, "<surname>", "</surname>");
                let given_names = xml_utils::extract_text_between(
                    name_content,
                    "<given-names>",
                    "</given-names>",
                );

                let author = Author::with_names(given_names, surname);
                authors.push(author);

                pos = name_end;
            } else {
                break;
            }
        }

        authors
    }

    /// Extract author names from a string-name element (alternative author format)
    pub fn extract_string_name_authors(content: &str) -> Vec<Author> {
        let mut authors = Vec::new();

        let string_name_tags = xml_utils::find_all_tags(content, "string-name");
        for tag in string_name_tags {
            if let Some(name_content) = xml_utils::extract_element_content(&tag, "string-name") {
                let clean_name = xml_utils::strip_xml_tags(&name_content);
                if !clean_name.trim().is_empty() {
                    authors.push(Author::new(clean_name.trim().to_string()));
                }
            }
        }

        authors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_authors() {
        let content = r#"
        <contrib-group>
            <contrib>
                <name>
                    <surname>Doe</surname>
                    <given-names>John</given-names>
                </name>
            </contrib>
            <contrib>
                <name>
                    <surname>Smith</surname>
                    <given-names>Jane</given-names>
                </name>
            </contrib>
        </contrib-group>
        "#;

        let authors = AuthorParser::extract_authors(content);
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0], "John Doe");
        assert_eq!(authors[1], "Jane Smith");
    }

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

        let authors = AuthorParser::extract_authors_detailed(content);
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

        let authors = AuthorParser::extract_reference_authors(content);
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].surname, Some("Johnson".to_string()));
        assert_eq!(authors[0].given_names, Some("Alice".to_string()));
        assert_eq!(authors[1].surname, Some("Williams".to_string()));
        assert_eq!(authors[1].given_names, Some("Bob".to_string()));
    }

    #[test]
    fn test_extract_string_name_authors() {
        let content = r#"
        <string-name>John Doe</string-name>
        <string-name>Jane Smith</string-name>
        "#;

        let authors = AuthorParser::extract_string_name_authors(content);
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].full_name, "John Doe");
        assert_eq!(authors[1].full_name, "Jane Smith");
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

        let authors = AuthorParser::extract_authors_detailed(content);
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

        let authors = AuthorParser::extract_authors_detailed(content);
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

        let authors = AuthorParser::extract_authors_detailed(content);
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
