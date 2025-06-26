use crate::error::{PubMedError, Result};
use crate::pubmed::models::{
    Affiliation, Author, ChemicalConcept, MeshHeading, MeshQualifier, MeshTerm, PubMedArticle,
};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::io::BufReader;
use tracing::{debug, instrument};

pub struct PubMedXmlParser;

impl PubMedXmlParser {
    /// Parse article from EFetch XML response
    #[instrument(skip(xml), fields(pmid = %pmid, xml_size = xml.len()))]
    pub fn parse_article_from_xml(xml: &str, pmid: &str) -> Result<PubMedArticle> {
        let mut reader = Reader::from_reader(BufReader::new(xml.as_bytes()));
        reader.config_mut().trim_text(true);

        let mut title = String::new();
        let mut authors: Vec<Author> = Vec::new();
        let mut journal = String::new();
        let mut pub_date = String::new();
        let doi = None;
        let mut abstract_text: Option<String> = None;
        let mut article_types = Vec::new();
        let mut mesh_headings: Vec<MeshHeading> = Vec::new();
        let mut keywords: Vec<String> = Vec::new();
        let mut chemical_list: Vec<ChemicalConcept> = Vec::new();

        let mut buf = Vec::new();
        let mut in_article_title = false;
        let mut in_abstract = false;
        let mut in_abstract_text = false;
        let mut in_journal_title = false;
        let mut in_pub_date = false;
        let mut in_author_list = false;
        let mut in_author = false;
        let mut in_last_name = false;
        let mut in_fore_name = false;
        let mut in_initials = false;
        let mut in_suffix = false;
        let mut in_publication_type = false;
        let mut in_affiliation_info = false;
        let mut in_affiliation = false;
        let mut in_identifier = false;
        let mut current_author_last = String::new();
        let mut current_author_fore = String::new();
        let mut current_author_initials = String::new();
        let mut current_author_suffix = String::new();
        let mut current_author_affiliations: Vec<Affiliation> = Vec::new();
        let mut current_affiliation_text = String::new();
        let mut current_orcid = Option::<String>::None;
        let mut current_identifier_source = String::new();

        // MeSH parsing state
        let mut in_mesh_heading_list = false;
        let mut in_mesh_heading = false;
        let mut in_descriptor_name = false;
        let mut in_qualifier_name = false;
        let mut current_descriptor_name = String::new();
        let mut current_descriptor_ui = String::new();
        let mut current_descriptor_major = false;
        let mut current_qualifiers: Vec<MeshQualifier> = Vec::new();
        let mut current_qualifier_name = String::new();
        let mut current_qualifier_ui = String::new();
        let mut current_qualifier_major = false;

        // Chemical parsing state
        let mut in_chemical_list = false;
        let mut in_chemical = false;
        let mut in_name_of_substance = false;
        let mut current_chemical_name = String::new();
        let mut current_registry_number = String::new();
        let mut current_chemical_ui = String::new();

        // Keyword parsing state
        let mut in_keyword_list = false;
        let mut in_keyword = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"ArticleTitle" => in_article_title = true,
                        b"Abstract" => in_abstract = true,
                        b"AbstractText" => in_abstract_text = true,
                        b"Title" if !in_article_title => in_journal_title = true,
                        b"PubDate" => in_pub_date = true,
                        b"AuthorList" => in_author_list = true,
                        b"Author" if in_author_list => {
                            in_author = true;
                            current_author_last.clear();
                            current_author_fore.clear();
                            current_author_initials.clear();
                            current_author_suffix.clear();
                            current_author_affiliations.clear();
                            current_orcid = None;
                        }
                        b"LastName" if in_author => in_last_name = true,
                        b"ForeName" if in_author => in_fore_name = true,
                        b"Initials" if in_author => in_initials = true,
                        b"Suffix" if in_author => in_suffix = true,
                        b"AffiliationInfo" if in_author => {
                            in_affiliation_info = true;
                            current_affiliation_text.clear();
                        }
                        b"Affiliation" if in_affiliation_info => in_affiliation = true,
                        b"Identifier" if in_author => {
                            in_identifier = true;
                            current_identifier_source.clear();
                            // Check for Source attribute
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"Source" {
                                    current_identifier_source =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                        b"PublicationType" => in_publication_type = true,
                        b"ELocationID" => {
                            // Check if this is a DOI
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"EIdType" && attr.value.as_ref() == b"doi"
                                {
                                    // We'll capture the DOI text in the next text event
                                }
                            }
                        }
                        // MeSH parsing
                        b"MeshHeadingList" => in_mesh_heading_list = true,
                        b"MeshHeading" if in_mesh_heading_list => {
                            in_mesh_heading = true;
                            current_qualifiers.clear();
                        }
                        b"DescriptorName" if in_mesh_heading => {
                            in_descriptor_name = true;
                            // Check for MajorTopicYN attribute
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"MajorTopicYN" {
                                    current_descriptor_major = attr.value.as_ref() == b"Y";
                                }
                                if attr.key.as_ref() == b"UI" {
                                    current_descriptor_ui =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                        b"QualifierName" if in_mesh_heading => {
                            in_qualifier_name = true;
                            // Check for MajorTopicYN attribute
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"MajorTopicYN" {
                                    current_qualifier_major = attr.value.as_ref() == b"Y";
                                }
                                if attr.key.as_ref() == b"UI" {
                                    current_qualifier_ui =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                        // Chemical parsing
                        b"ChemicalList" => in_chemical_list = true,
                        b"Chemical" if in_chemical_list => {
                            in_chemical = true;
                            current_chemical_name.clear();
                            current_registry_number.clear();
                            current_chemical_ui.clear();
                        }
                        b"NameOfSubstance" if in_chemical => {
                            in_name_of_substance = true;
                            // Get UI attribute if available
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"UI" {
                                    current_chemical_ui =
                                        String::from_utf8_lossy(&attr.value).to_string();
                                }
                            }
                        }
                        // Keyword parsing
                        b"KeywordList" => in_keyword_list = true,
                        b"Keyword" if in_keyword_list => in_keyword = true,
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => match e.name().as_ref() {
                    b"ArticleTitle" => in_article_title = false,
                    b"Abstract" => in_abstract = false,
                    b"AbstractText" => in_abstract_text = false,
                    b"Title" => in_journal_title = false,
                    b"PubDate" => in_pub_date = false,
                    b"AuthorList" => in_author_list = false,
                    b"Author" => {
                        if in_author {
                            let full_name = format_author_name(
                                &if current_author_last.is_empty() {
                                    None
                                } else {
                                    Some(current_author_last.clone())
                                },
                                &if current_author_fore.is_empty() {
                                    None
                                } else {
                                    Some(current_author_fore.clone())
                                },
                                &if current_author_initials.is_empty() {
                                    None
                                } else {
                                    Some(current_author_initials.clone())
                                },
                            );

                            if !full_name.trim().is_empty() && full_name != "Unknown Author" {
                                let author = Author {
                                    last_name: if current_author_last.is_empty() {
                                        None
                                    } else {
                                        Some(current_author_last.clone())
                                    },
                                    fore_name: if current_author_fore.is_empty() {
                                        None
                                    } else {
                                        Some(current_author_fore.clone())
                                    },
                                    first_name: None, // Could be extracted from fore_name if needed
                                    middle_name: None, // Could be extracted from fore_name if needed
                                    initials: if current_author_initials.is_empty() {
                                        None
                                    } else {
                                        Some(current_author_initials.clone())
                                    },
                                    suffix: if current_author_suffix.is_empty() {
                                        None
                                    } else {
                                        Some(current_author_suffix.clone())
                                    },
                                    full_name,
                                    affiliations: current_author_affiliations.clone(),
                                    orcid: current_orcid.clone(),
                                    is_corresponding: false, // TODO: Detect corresponding authors from XML
                                    author_roles: Vec::new(), // TODO: Parse author contributions if available
                                };
                                authors.push(author);
                            }
                            in_author = false;
                        }
                    }
                    b"LastName" => in_last_name = false,
                    b"ForeName" => in_fore_name = false,
                    b"Initials" => in_initials = false,
                    b"Suffix" => in_suffix = false,
                    b"AffiliationInfo" => {
                        if in_affiliation_info && !current_affiliation_text.is_empty() {
                            let affiliation = parse_affiliation_text(&current_affiliation_text);
                            current_author_affiliations.push(affiliation);
                        }
                        in_affiliation_info = false;
                    }
                    b"Affiliation" => in_affiliation = false,
                    b"Identifier" => {
                        in_identifier = false;
                        current_identifier_source.clear();
                    }
                    b"PublicationType" => in_publication_type = false,
                    // MeSH parsing
                    b"MeshHeadingList" => in_mesh_heading_list = false,
                    b"MeshHeading" => {
                        if in_mesh_heading {
                            // Create MeshTerm and add to headings
                            if !current_descriptor_name.is_empty() {
                                let mesh_term = MeshTerm {
                                    descriptor_name: current_descriptor_name.clone(),
                                    descriptor_ui: current_descriptor_ui.clone(),
                                    major_topic: current_descriptor_major,
                                    qualifiers: current_qualifiers.clone(),
                                };
                                mesh_headings.push(MeshHeading {
                                    mesh_terms: vec![mesh_term],
                                    supplemental_concepts: Vec::new(), // TODO: Parse supplemental concepts if needed
                                });
                            }
                            current_descriptor_name.clear();
                            current_descriptor_ui.clear();
                            current_descriptor_major = false;
                            in_mesh_heading = false;
                        }
                    }
                    b"DescriptorName" => in_descriptor_name = false,
                    b"QualifierName" => {
                        if in_qualifier_name && !current_qualifier_name.is_empty() {
                            current_qualifiers.push(MeshQualifier {
                                qualifier_name: current_qualifier_name.clone(),
                                qualifier_ui: current_qualifier_ui.clone(),
                                major_topic: current_qualifier_major,
                            });
                            current_qualifier_name.clear();
                            current_qualifier_ui.clear();
                            current_qualifier_major = false;
                        }
                        in_qualifier_name = false;
                    }
                    // Chemical parsing
                    b"ChemicalList" => in_chemical_list = false,
                    b"Chemical" => {
                        if in_chemical && !current_chemical_name.is_empty() {
                            chemical_list.push(ChemicalConcept {
                                name: current_chemical_name.clone(),
                                registry_number: if current_registry_number.is_empty() {
                                    None
                                } else {
                                    Some(current_registry_number.clone())
                                },
                                ui: if current_chemical_ui.is_empty() {
                                    None
                                } else {
                                    Some(current_chemical_ui.clone())
                                },
                            });
                        }
                        in_chemical = false;
                    }
                    b"NameOfSubstance" => in_name_of_substance = false,
                    // Keyword parsing
                    b"KeywordList" => in_keyword_list = false,
                    b"Keyword" => in_keyword = false,
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    let text = e
                        .unescape()
                        .map_err(|_| PubMedError::XmlParseError {
                            message: "Failed to decode XML text".to_string(),
                        })?
                        .into_owned();

                    if in_article_title {
                        title = text;
                    } else if in_abstract_text && in_abstract {
                        // Handle structured abstracts with multiple AbstractText sections
                        if let Some(existing) = abstract_text.as_mut() {
                            if !existing.is_empty() {
                                existing.push(' '); // Add space between sections
                            }
                            existing.push_str(&text);
                        } else {
                            abstract_text = Some(text);
                        }
                    } else if in_journal_title && !in_article_title {
                        journal = text;
                    } else if in_pub_date {
                        if pub_date.is_empty() {
                            pub_date = text;
                        } else {
                            pub_date.push(' ');
                            pub_date.push_str(&text);
                        }
                    } else if in_last_name && in_author {
                        current_author_last = text;
                    } else if in_fore_name && in_author {
                        current_author_fore = text;
                    } else if in_initials && in_author {
                        current_author_initials = text;
                    } else if in_suffix && in_author {
                        current_author_suffix = text;
                    } else if in_affiliation && in_affiliation_info {
                        current_affiliation_text = text;
                    } else if in_identifier && in_author {
                        if current_identifier_source == "ORCID" {
                            current_orcid = Some(text);
                        }
                    } else if in_publication_type {
                        article_types.push(text);
                    } else if in_descriptor_name && in_mesh_heading {
                        current_descriptor_name = text;
                    } else if in_qualifier_name && in_mesh_heading {
                        current_qualifier_name = text;
                    } else if in_name_of_substance && in_chemical {
                        current_chemical_name = text;
                    } else if in_chemical && text.trim() != current_chemical_name {
                        // This might be RegistryNumber
                        current_registry_number = text;
                    } else if in_keyword && in_keyword_list {
                        keywords.push(text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(PubMedError::XmlParseError {
                        message: format!("XML parsing error: {}", e),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        // If no article found, return error
        if title.is_empty() {
            debug!("No article title found in XML, article not found");
            return Err(PubMedError::ArticleNotFound {
                pmid: pmid.to_string(),
            });
        }

        debug!(
            authors_parsed = authors.len(),
            has_abstract = abstract_text.is_some(),
            journal = %journal,
            mesh_terms_count = mesh_headings.len(),
            keywords_count = keywords.len(),
            chemicals_count = chemical_list.len(),
            "Completed XML parsing"
        );

        let author_count = authors.len() as u32;

        Ok(PubMedArticle {
            pmid: pmid.to_string(),
            title,
            authors,
            author_count,
            journal,
            pub_date,
            doi,
            abstract_text,
            article_types,
            mesh_headings: if mesh_headings.is_empty() {
                None
            } else {
                Some(mesh_headings)
            },
            keywords: if keywords.is_empty() {
                None
            } else {
                Some(keywords)
            },
            chemical_list: if chemical_list.is_empty() {
                None
            } else {
                Some(chemical_list)
            },
        })
    }
}

/// Parse affiliation text into structured components
fn parse_affiliation_text(text: &str) -> Affiliation {
    // This is a simplified parser for affiliation text
    // In practice, this would be more sophisticated
    let text = text.trim();

    // Extract email if present
    let email = extract_email_from_text(text);

    // Extract country (usually at the end)
    let country = extract_country_from_text(text);

    // For now, treat the whole text as institution
    // More sophisticated parsing could identify departments, addresses, etc.
    Affiliation {
        institution: if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        },
        department: None, // TODO: Parse department from affiliation text
        address: None,    // TODO: Parse address from affiliation text
        country,
        email,
    }
}

/// Extract email address from affiliation text
fn extract_email_from_text(text: &str) -> Option<String> {
    // Simple regex-like pattern matching for email
    let parts: Vec<&str> = text.split_whitespace().collect();
    for part in parts {
        if part.contains('@') && part.contains('.') {
            // Remove punctuation that might be at the end
            let cleaned = part.trim_end_matches(&['.', ',', ';', ')'][..]);
            if cleaned.len() > 5 {
                // Basic validation
                return Some(cleaned.to_string());
            }
        }
    }
    None
}

/// Extract country from affiliation text (basic implementation)
fn extract_country_from_text(text: &str) -> Option<String> {
    // Common country patterns that might appear at the end of affiliations
    let common_countries = [
        "USA",
        "United States",
        "US",
        "UK",
        "United Kingdom",
        "England",
        "Scotland",
        "Wales",
        "Canada",
        "Australia",
        "Germany",
        "France",
        "Italy",
        "Spain",
        "Japan",
        "China",
        "India",
        "Brazil",
        "Netherlands",
        "Sweden",
        "Switzerland",
        "Denmark",
        "Norway",
        "Finland",
        "Belgium",
        "Austria",
        "Portugal",
        "Ireland",
        "Israel",
        "South Korea",
        "Singapore",
        "Hong Kong",
        "Taiwan",
        "New Zealand",
        "Mexico",
    ];

    let text_lower = text.to_lowercase();
    for country in &common_countries {
        if text_lower.ends_with(&country.to_lowercase())
            || text_lower.contains(&format!(", {}", country.to_lowercase()))
        {
            return Some(country.to_string());
        }
    }

    None
}

/// Format an author name from components
fn format_author_name(
    last_name: &Option<String>,
    fore_name: &Option<String>,
    initials: &Option<String>,
) -> String {
    match (fore_name, last_name) {
        (Some(fore), Some(last)) => format!("{} {}", fore, last),
        (None, Some(last)) => {
            if let Some(init) = initials {
                format!("{} {}", init, last)
            } else {
                last.clone()
            }
        }
        (Some(fore), None) => fore.clone(),
        (None, None) => "Unknown Author".to_string(),
    }
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
        <MedlineJournalInfo>
            <MedlineTA>Test J</MedlineTA>
        </MedlineJournalInfo>
        <PubmedData>
            <ArticleIdList>
                <ArticleId IdType="pubmed">12345678</ArticleId>
            </ArticleIdList>
            <PublicationStatus>ppublish</PublicationStatus>
        </PubmedData>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = PubMedXmlParser::parse_article_from_xml(xml, "12345678").unwrap();

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
        assert_eq!(author.last_name, Some("Doe".to_string()));
        assert_eq!(author.fore_name, Some("John".to_string()));
        assert_eq!(author.initials, Some("JA".to_string()));
        assert_eq!(author.full_name, "John Doe");
        assert_eq!(author.orcid, Some("0000-0001-2345-6789".to_string()));
        assert_eq!(author.affiliations.len(), 1);
        assert!(
            author.affiliations[0]
                .institution
                .as_ref()
                .unwrap()
                .contains("Harvard Medical School")
        );

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
                        </Journal>
                        <PubDate>
                            <Year>2020</Year>
                            <Month>Sep</Month>
                        </PubDate>
                    </Article>
                </MedlineCitation>
            </PubmedArticle>
        </PubmedArticleSet>"#;

        let result = PubMedXmlParser::parse_article_from_xml(xml, "32887691");
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

        debug!(
            abstract_length = abstract_text.len(),
            "Parsed abstract successfully"
        );
        debug!(abstract = %abstract_text, "Abstract content parsed");
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
        <MedlineJournalInfo>
            <MedlineTA>Another J</MedlineTA>
        </MedlineJournalInfo>
        <PubmedData>
            <ArticleIdList>
                <ArticleId IdType="pubmed">87654321</ArticleId>
            </ArticleIdList>
            <PublicationStatus>ppublish</PublicationStatus>
        </PubmedData>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = PubMedXmlParser::parse_article_from_xml(xml, "87654321").unwrap();

        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.author_count, 1);
        assert_eq!(article.authors[0].full_name, "Jane Smith");
        assert!(article.mesh_headings.is_none());
        assert!(article.chemical_list.is_none());
        assert!(article.keywords.is_none());
    }

    #[test]
    fn test_parse_affiliation_text() {
        let affiliation_text = "Department of Medicine, Harvard Medical School, Boston, MA, USA. john.doe@hms.harvard.edu";
        let affiliation = parse_affiliation_text(affiliation_text);

        assert!(affiliation.institution.is_some());
        assert!(
            affiliation
                .institution
                .as_ref()
                .unwrap()
                .contains("Harvard Medical School")
        );
        assert_eq!(
            affiliation.email,
            Some("john.doe@hms.harvard.edu".to_string())
        );
        assert_eq!(affiliation.country, Some("USA".to_string()));
    }

    #[test]
    fn test_extract_email_from_text() {
        assert_eq!(
            extract_email_from_text("Contact john.doe@example.com for details"),
            Some("john.doe@example.com".to_string())
        );

        assert_eq!(
            extract_email_from_text("Email: jane.smith@university.edu."),
            Some("jane.smith@university.edu".to_string())
        );

        assert_eq!(extract_email_from_text("No email here"), None);
    }

    #[test]
    fn test_extract_country_from_text() {
        assert_eq!(
            extract_country_from_text("Harvard Medical School, Boston, MA, USA"),
            Some("USA".to_string())
        );

        assert_eq!(
            extract_country_from_text("University of Oxford, Oxford, UK"),
            Some("UK".to_string())
        );

        assert_eq!(extract_country_from_text("Local Institution"), None);
    }

    #[test]
    fn test_format_author_name() {
        assert_eq!(
            format_author_name(&Some("Smith".to_string()), &Some("John".to_string()), &None),
            "John Smith"
        );

        assert_eq!(
            format_author_name(&Some("Doe".to_string()), &None, &Some("J".to_string())),
            "J Doe"
        );

        assert_eq!(
            format_author_name(&Some("Johnson".to_string()), &None, &None),
            "Johnson"
        );
    }
}
