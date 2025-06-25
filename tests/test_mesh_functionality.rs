use pubmed_client_rs::PubMedClient;
use pubmed_client_rs::pubmed::{
    ChemicalConcept, MeshHeading, MeshQualifier, MeshTerm, PubMedArticle, SearchQuery,
};

#[cfg(test)]
mod mesh_parsing_tests {
    use pubmed_client_rs::pubmed::parser::PubMedXmlParser;

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

#[cfg(test)]
mod mesh_utility_tests {
    use super::*;

    fn create_test_article_with_mesh() -> PubMedArticle {
        PubMedArticle {
            pmid: "12345".to_string(),
            title: "Test Article".to_string(),
            authors: vec!["John Doe".to_string()],
            journal: "Test Journal".to_string(),
            pub_date: "2023".to_string(),
            doi: None,
            abstract_text: None,
            article_types: vec![],
            mesh_headings: Some(vec![
                MeshHeading {
                    mesh_terms: vec![MeshTerm {
                        descriptor_name: "Diabetes Mellitus, Type 2".to_string(),
                        descriptor_ui: "D003924".to_string(),
                        major_topic: true,
                        qualifiers: vec![
                            MeshQualifier {
                                qualifier_name: "drug therapy".to_string(),
                                qualifier_ui: "Q000188".to_string(),
                                major_topic: false,
                            },
                            MeshQualifier {
                                qualifier_name: "genetics".to_string(),
                                qualifier_ui: "Q000235".to_string(),
                                major_topic: true,
                            },
                        ],
                    }],
                    supplemental_concepts: vec![],
                },
                MeshHeading {
                    mesh_terms: vec![MeshTerm {
                        descriptor_name: "Hypertension".to_string(),
                        descriptor_ui: "D006973".to_string(),
                        major_topic: false,
                        qualifiers: vec![],
                    }],
                    supplemental_concepts: vec![],
                },
            ]),
            keywords: Some(vec!["diabetes".to_string(), "treatment".to_string()]),
            chemical_list: Some(vec![ChemicalConcept {
                name: "Metformin".to_string(),
                registry_number: Some("657-24-9".to_string()),
                ui: Some("D008687".to_string()),
            }]),
        }
    }

    #[test]
    fn test_get_major_mesh_terms() {
        let article = create_test_article_with_mesh();
        let major_terms = article.get_major_mesh_terms();

        assert_eq!(major_terms.len(), 1);
        assert_eq!(major_terms[0], "Diabetes Mellitus, Type 2");
    }

    #[test]
    fn test_has_mesh_term() {
        let article = create_test_article_with_mesh();

        assert!(article.has_mesh_term("Diabetes Mellitus, Type 2"));
        assert!(article.has_mesh_term("DIABETES MELLITUS, TYPE 2")); // Case insensitive
        assert!(article.has_mesh_term("Hypertension"));
        assert!(!article.has_mesh_term("Cancer"));
    }

    #[test]
    fn test_get_all_mesh_terms() {
        let article = create_test_article_with_mesh();
        let all_terms = article.get_all_mesh_terms();

        assert_eq!(all_terms.len(), 2);
        assert!(all_terms.contains(&"Diabetes Mellitus, Type 2".to_string()));
        assert!(all_terms.contains(&"Hypertension".to_string()));
    }

    #[test]
    fn test_mesh_term_similarity() {
        let article1 = create_test_article_with_mesh();
        let mut article2 = create_test_article_with_mesh();

        // Same article should have similarity of 1.0
        let similarity = article1.mesh_term_similarity(&article2);
        assert_eq!(similarity, 1.0);

        // Different MeSH terms
        article2.mesh_headings = Some(vec![MeshHeading {
            mesh_terms: vec![
                MeshTerm {
                    descriptor_name: "Diabetes Mellitus, Type 2".to_string(),
                    descriptor_ui: "D003924".to_string(),
                    major_topic: true,
                    qualifiers: vec![],
                },
                MeshTerm {
                    descriptor_name: "Obesity".to_string(),
                    descriptor_ui: "D009765".to_string(),
                    major_topic: false,
                    qualifiers: vec![],
                },
            ],
            supplemental_concepts: vec![],
        }]);

        let similarity = article1.mesh_term_similarity(&article2);
        // Should have partial similarity (1 common term out of 3 unique terms)
        assert!(similarity > 0.0 && similarity < 1.0);
        assert_eq!(similarity, 1.0 / 3.0); // Jaccard similarity

        // No MeSH terms
        let article3 = PubMedArticle {
            pmid: "54321".to_string(),
            title: "Test".to_string(),
            authors: vec![],
            journal: "Test".to_string(),
            pub_date: "2023".to_string(),
            doi: None,
            abstract_text: None,
            article_types: vec![],
            mesh_headings: None,
            keywords: None,
            chemical_list: None,
        };

        assert_eq!(article1.mesh_term_similarity(&article3), 0.0);
    }

    #[test]
    fn test_get_mesh_qualifiers() {
        let article = create_test_article_with_mesh();

        let qualifiers = article.get_mesh_qualifiers("Diabetes Mellitus, Type 2");
        assert_eq!(qualifiers.len(), 2);
        assert!(qualifiers.contains(&"drug therapy".to_string()));
        assert!(qualifiers.contains(&"genetics".to_string()));

        let qualifiers = article.get_mesh_qualifiers("Hypertension");
        assert_eq!(qualifiers.len(), 0);

        let qualifiers = article.get_mesh_qualifiers("Nonexistent Term");
        assert_eq!(qualifiers.len(), 0);
    }

    #[test]
    fn test_has_mesh_terms() {
        let article = create_test_article_with_mesh();
        assert!(article.has_mesh_terms());

        let mut article_no_mesh = article.clone();
        article_no_mesh.mesh_headings = None;
        assert!(!article_no_mesh.has_mesh_terms());

        let mut article_empty_mesh = article.clone();
        article_empty_mesh.mesh_headings = Some(vec![]);
        assert!(!article_empty_mesh.has_mesh_terms());
    }

    #[test]
    fn test_get_chemical_names() {
        let article = create_test_article_with_mesh();
        let chemicals = article.get_chemical_names();

        assert_eq!(chemicals.len(), 1);
        assert_eq!(chemicals[0], "Metformin");

        let mut article_no_chemicals = article.clone();
        article_no_chemicals.chemical_list = None;
        let chemicals = article_no_chemicals.get_chemical_names();
        assert_eq!(chemicals.len(), 0);
    }
}

#[cfg(test)]
mod mesh_search_tests {
    use super::*;

    #[test]
    fn test_mesh_major_topic_query() {
        let query = SearchQuery::new()
            .mesh_major_topic("Diabetes Mellitus, Type 2")
            .build();

        assert_eq!(query, "Diabetes Mellitus, Type 2[MeSH Major Topic]");
    }

    #[test]
    fn test_mesh_term_query() {
        let query = SearchQuery::new().mesh_term("Neoplasms").build();

        assert_eq!(query, "Neoplasms[MeSH Terms]");
    }

    #[test]
    fn test_multiple_mesh_terms_query() {
        let query = SearchQuery::new()
            .mesh_terms(&["Neoplasms", "Antineoplastic Agents"])
            .build();

        assert_eq!(
            query,
            "Neoplasms[MeSH Terms] AND Antineoplastic Agents[MeSH Terms]"
        );
    }

    #[test]
    fn test_mesh_subheading_query() {
        let query = SearchQuery::new()
            .mesh_term("Diabetes Mellitus")
            .mesh_subheading("drug therapy")
            .build();

        assert_eq!(
            query,
            "Diabetes Mellitus[MeSH Terms] AND drug therapy[MeSH Subheading]"
        );
    }

    #[test]
    fn test_complex_mesh_query() {
        let query = SearchQuery::new()
            .query("clinical outcomes")
            .mesh_major_topic("Diabetes Mellitus, Type 2")
            .mesh_subheading("drug therapy")
            .published_after(2020)
            .clinical_trials_only()
            .build();

        let expected = "clinical outcomes AND Diabetes Mellitus, Type 2[MeSH Major Topic] AND drug therapy[MeSH Subheading] AND 2020:3000[pdat] AND Clinical Trial[pt]";
        assert_eq!(query, expected);
    }

    #[test]
    fn test_mesh_query_with_free_text() {
        let query = SearchQuery::new()
            .query("machine learning")
            .mesh_terms(&["Artificial Intelligence", "Machine Learning"])
            .free_full_text()
            .build();

        let expected = "machine learning AND Artificial Intelligence[MeSH Terms] AND Machine Learning[MeSH Terms] AND free full text[sb]";
        assert_eq!(query, expected);
    }
}

#[tokio::test]
#[ignore] // This is an integration test that requires network access
async fn test_mesh_search_integration() {
    use pubmed_client_rs::ClientConfig;

    // Create client with rate limiting for testing
    let config = ClientConfig::new().with_rate_limit(1.0);
    let client = PubMedClient::with_config(config);

    // Search for articles with specific MeSH terms
    let articles = SearchQuery::new()
        .mesh_major_topic("COVID-19")
        .mesh_subheading("prevention & control")
        .published_after(2023)
        .limit(5)
        .search_and_fetch(&client)
        .await
        .unwrap();

    assert!(!articles.is_empty());

    // Verify that fetched articles have MeSH terms
    for article in &articles {
        println!("Article: {} - {}", article.pmid, article.title);
        if let Some(_mesh_headings) = &article.mesh_headings {
            println!("  MeSH terms: {}", article.get_all_mesh_terms().join(", "));

            // Check if COVID-19 is a major topic
            let major_terms = article.get_major_mesh_terms();
            println!("  Major topics: {}", major_terms.join(", "));
        }
    }
}

#[tokio::test]
#[ignore] // This is an integration test that requires network access
async fn test_chemical_search_integration() {
    use pubmed_client_rs::ClientConfig;

    let config = ClientConfig::new().with_rate_limit(1.0);
    let client = PubMedClient::with_config(config);

    // Search for articles about metformin
    let articles = SearchQuery::new()
        .mesh_term("Metformin")
        .mesh_major_topic("Diabetes Mellitus, Type 2")
        .published_after(2022)
        .limit(3)
        .search_and_fetch(&client)
        .await
        .unwrap();

    assert!(!articles.is_empty());

    for article in &articles {
        println!("Article: {} - {}", article.pmid, article.title);

        // Check chemicals
        let chemicals = article.get_chemical_names();
        if !chemicals.is_empty() {
            println!("  Chemicals: {}", chemicals.join(", "));
        }

        // Check MeSH qualifiers for diabetes
        let qualifiers = article.get_mesh_qualifiers("Diabetes Mellitus, Type 2");
        if !qualifiers.is_empty() {
            println!("  Diabetes qualifiers: {}", qualifiers.join(", "));
        }
    }
}
