use crate::error::{PubMedError, Result};
use crate::pubmed::models::{ChemicalConcept, MeshHeading, MeshQualifier, MeshTerm, PubMedArticle};
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
        let mut authors = Vec::new();
        let mut journal = String::new();
        let mut pub_date = String::new();
        let doi = None;
        let mut abstract_text = None;
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
        let mut in_publication_type = false;
        let mut current_author_last = String::new();
        let mut current_author_fore = String::new();

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
                        }
                        b"LastName" if in_author => in_last_name = true,
                        b"ForeName" if in_author => in_fore_name = true,
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
                            let full_name = if !current_author_fore.is_empty() {
                                format!("{} {}", current_author_fore, current_author_last)
                            } else {
                                current_author_last.clone()
                            };
                            if !full_name.trim().is_empty() {
                                authors.push(full_name);
                            }
                            in_author = false;
                        }
                    }
                    b"LastName" => in_last_name = false,
                    b"ForeName" => in_fore_name = false,
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
                        abstract_text = Some(text);
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

        Ok(PubMedArticle {
            pmid: pmid.to_string(),
            title,
            authors,
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

        // Test keywords
        assert!(article.keywords.is_some());
        let keywords = article.keywords.as_ref().unwrap();
        assert_eq!(keywords.len(), 2);
        assert_eq!(keywords[0], "diabetes treatment");
        assert_eq!(keywords[1], "insulin therapy");
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

        assert!(article.mesh_headings.is_none());
        assert!(article.chemical_list.is_none());
        assert!(article.keywords.is_none());
    }
}
