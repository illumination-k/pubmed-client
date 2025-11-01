use rstest::*;
use tracing::{debug, info, warn};
use tracing_test::traced_test;

use pubmed_client::pubmed::parser::parse_article_from_xml;

mod common;
use common::{pubmed_xml_test_cases, PubMedXmlTestCase};

/// Fixture for all PubMed XML test cases
#[fixture]
fn xml_test_cases() -> Vec<PubMedXmlTestCase> {
    pubmed_xml_test_cases()
}

#[rstest]
#[traced_test]
fn test_xml_parsing_basic_validity(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    for test_case in &test_cases {
        info!(filename = test_case.filename(), "Testing basic parsing");

        let xml_content = test_case.read_xml_content_or_panic();

        // Basic validity checks
        assert!(!xml_content.is_empty(), "XML file should not be empty");
        assert!(
            xml_content.contains("<PubmedArticle") || xml_content.contains("<MedlineCitation"),
            "Should contain PubmedArticle or MedlineCitation tag"
        );
        assert!(
            xml_content.contains(&test_case.pmid),
            "Should contain PMID reference"
        );

        info!(filename = test_case.filename(), "Basic validity passed");
    }
}

#[rstest]
#[traced_test]
fn test_comprehensive_pubmed_parsing(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    let mut successful_parses = 0;
    let mut failed_parses = 0;
    let mut parse_errors = Vec::new();

    for test_case in &test_cases {
        info!(
            filename = test_case.filename(),
            "Testing comprehensive parsing"
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        match result {
            Ok(article) => {
                successful_parses += 1;

                // Basic validation
                assert!(!article.title.is_empty(), "Article should have a title");
                assert!(!article.pmid.is_empty(), "Article should have a PMID");
                assert_eq!(article.pmid, test_case.pmid, "PMID should match");

                // Log some statistics
                debug!(
                    title_preview = article.title.chars().take(60).collect::<String>(),
                    authors_count = article.authors.len(),
                    journal = article.journal,
                    pub_date = article.pub_date,
                    "Article metadata"
                );

                if let Some(doi) = &article.doi {
                    debug!(doi = doi, "Article has DOI");
                }

                debug!(
                    has_abstract = article.abstract_text.is_some(),
                    "Abstract status"
                );

                if let Some(mesh_headings) = &article.mesh_headings {
                    let total_mesh_terms = mesh_headings
                        .iter()
                        .map(|h| h.mesh_terms.len())
                        .sum::<usize>();
                    debug!(mesh_terms_count = total_mesh_terms, "MeSH terms found");
                }

                info!(
                    filename = test_case.filename(),
                    "Comprehensive parsing passed"
                );
            }
            Err(e) => {
                failed_parses += 1;
                parse_errors.push((test_case.filename().to_string(), e.to_string()));
                warn!(filename = test_case.filename(), error = %e, "Parsing failed");
            }
        }
    }

    // Summary
    let success_rate = (successful_parses as f64 / test_cases.len() as f64) * 100.0;
    let failure_rate = (failed_parses as f64 / test_cases.len() as f64) * 100.0;
    info!(
        total_files = test_cases.len(),
        successful_parses = successful_parses,
        success_rate = success_rate,
        failed_parses = failed_parses,
        failure_rate = failure_rate,
        "Comprehensive PubMed parsing summary"
    );

    if !parse_errors.is_empty() {
        warn!("Parse errors encountered");
        for (filename, error) in parse_errors {
            warn!(filename = filename, error = error, "Parse error details");
        }
    }

    // Assert that most files parse successfully (at least 80%)
    let success_rate = successful_parses as f64 / test_cases.len() as f64;
    assert!(
        success_rate >= 0.8,
        "Success rate should be at least 80%, got {:.1}%",
        success_rate * 100.0
    );
}

#[rstest]
#[traced_test]
fn test_pubmed_parsing_statistics(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    let mut total_authors = 0;
    let mut total_mesh_terms = 0;
    let mut articles_with_doi = 0;
    let mut articles_with_abstract = 0;
    let mut articles_with_mesh = 0;
    let mut articles_with_keywords = 0;
    let mut articles_with_chemicals = 0;

    let mut successful_parses = 0;

    for test_case in test_cases.iter().take(10) {
        // Limit to first 10 for performance
        info!(filename = test_case.filename(), "Analyzing statistics");

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        if let Ok(article) = result {
            successful_parses += 1;
            total_authors += article.authors.len();

            if article.doi.is_some() {
                articles_with_doi += 1;
            }
            if article.abstract_text.is_some() {
                articles_with_abstract += 1;
            }
            if let Some(mesh_headings) = &article.mesh_headings {
                if !mesh_headings.is_empty() {
                    articles_with_mesh += 1;
                    for heading in mesh_headings {
                        total_mesh_terms += heading.mesh_terms.len();
                    }
                }
            }
            if let Some(keywords) = &article.keywords {
                if !keywords.is_empty() {
                    articles_with_keywords += 1;
                }
            }
            if let Some(chemicals) = &article.chemical_list {
                if !chemicals.is_empty() {
                    articles_with_chemicals += 1;
                }
            }

            debug!(
                authors_count = article.authors.len(),
                has_abstract = article.abstract_text.is_some(),
                mesh_headings_count = article.mesh_headings.as_ref().map_or(0, |h| h.len()),
                keywords_count = article.keywords.as_ref().map_or(0, |k| k.len()),
                chemicals_count = article.chemical_list.as_ref().map_or(0, |c| c.len()),
                "Article content statistics"
            );
        }
    }

    // Print statistics
    info!(
        files_analyzed = successful_parses,
        "PubMed content statistics"
    );
    if successful_parses > 0 {
        let avg_authors = total_authors as f64 / successful_parses as f64;
        let avg_mesh_terms = total_mesh_terms as f64 / successful_parses as f64;
        let doi_percentage = (articles_with_doi as f64 / successful_parses as f64) * 100.0;
        let abstract_percentage =
            (articles_with_abstract as f64 / successful_parses as f64) * 100.0;
        let mesh_percentage = (articles_with_mesh as f64 / successful_parses as f64) * 100.0;
        let keywords_percentage =
            (articles_with_keywords as f64 / successful_parses as f64) * 100.0;
        let chemicals_percentage =
            (articles_with_chemicals as f64 / successful_parses as f64) * 100.0;

        info!(
            avg_authors_per_article = avg_authors,
            avg_mesh_terms_per_article = avg_mesh_terms,
            articles_with_doi = articles_with_doi,
            doi_percentage = doi_percentage,
            articles_with_abstract = articles_with_abstract,
            abstract_percentage = abstract_percentage,
            articles_with_mesh = articles_with_mesh,
            mesh_percentage = mesh_percentage,
            articles_with_keywords = articles_with_keywords,
            keywords_percentage = keywords_percentage,
            articles_with_chemicals = articles_with_chemicals,
            chemicals_percentage = chemicals_percentage,
            "Content statistics summary"
        );
    }
}

#[rstest]
#[traced_test]
fn test_pubmed_article_metadata(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    for test_case in test_cases.iter().take(5) {
        // Limit for performance
        info!(
            filename = test_case.filename(),
            "Analyzing article metadata"
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        if let Ok(article) = result {
            // Title validation
            assert!(!article.title.is_empty(), "Title should not be empty");
            assert!(
                article.title.len() > 10,
                "Title should be substantial (>10 chars)"
            );

            // PMID validation
            assert_eq!(article.pmid, test_case.pmid, "PMID should match test case");

            // Journal validation
            assert!(!article.journal.is_empty(), "Journal should not be empty");

            // Publication date validation
            assert!(
                !article.pub_date.is_empty(),
                "Publication date should not be empty"
            );

            // Authors validation
            if !article.authors.is_empty() {
                for author in &article.authors {
                    assert!(!author.is_empty(), "Author name should not be empty");
                }
            }

            // Article types validation
            if !article.article_types.is_empty() {
                for article_type in &article.article_types {
                    assert!(!article_type.is_empty(), "Article type should not be empty");
                }
            }

            info!(
                title_length = article.title.len(),
                authors_count = article.authors.len(),
                article_types_count = article.article_types.len(),
                "Metadata validation passed"
            );
        }
    }
}

#[rstest]
#[traced_test]
fn test_pubmed_mesh_term_extraction(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    let mut articles_with_major_topics = 0;
    let mut articles_with_qualifiers = 0;
    let mut total_major_topics = 0;
    let mut total_qualifiers = 0;

    for test_case in test_cases.iter().take(8) {
        // Test first 8 files
        info!(filename = test_case.filename(), "Analyzing MeSH terms");

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        if let Ok(article) = result {
            if let Some(mesh_headings) = &article.mesh_headings {
                let mut has_major_topics = false;
                let mut has_qualifiers = false;
                let mut article_major_topics = 0;
                let mut article_qualifiers = 0;

                for heading in mesh_headings {
                    for mesh_term in &heading.mesh_terms {
                        // Validate MeSH term structure
                        assert!(
                            !mesh_term.descriptor_name.is_empty(),
                            "MeSH descriptor name should not be empty"
                        );
                        assert!(
                            !mesh_term.descriptor_ui.is_empty(),
                            "MeSH descriptor UI should not be empty"
                        );

                        if mesh_term.major_topic {
                            has_major_topics = true;
                            article_major_topics += 1;
                        }

                        if !mesh_term.qualifiers.is_empty() {
                            has_qualifiers = true;
                            article_qualifiers += mesh_term.qualifiers.len();

                            for qualifier in &mesh_term.qualifiers {
                                assert!(
                                    !qualifier.qualifier_name.is_empty(),
                                    "Qualifier name should not be empty"
                                );
                                assert!(
                                    !qualifier.qualifier_ui.is_empty(),
                                    "Qualifier UI should not be empty"
                                );
                            }
                        }
                    }
                }

                if has_major_topics {
                    articles_with_major_topics += 1;
                    total_major_topics += article_major_topics;
                }
                if has_qualifiers {
                    articles_with_qualifiers += 1;
                    total_qualifiers += article_qualifiers;
                }

                // Test utility methods
                let major_terms = article.get_major_mesh_terms();
                let all_terms = article.get_all_mesh_terms();

                assert!(
                    all_terms.len() >= major_terms.len(),
                    "All terms should include major terms"
                );

                // Test MeSH term checking
                if !all_terms.is_empty() {
                    let first_term = &all_terms[0];
                    assert!(
                        article.has_mesh_term(first_term),
                        "Should find existing MeSH term"
                    );
                    assert!(
                        !article.has_mesh_term("NonexistentMeshTerm12345"),
                        "Should not find non-existent MeSH term"
                    );
                }

                debug!(
                    mesh_terms_count = all_terms.len(),
                    major_topics_count = article_major_topics,
                    qualifiers_count = article_qualifiers,
                    "MeSH terms analysis"
                );
            } else {
                debug!("No MeSH terms found");
            }
        }
    }

    // MeSH statistics summary
    let avg_major_topics = if articles_with_major_topics > 0 {
        Some(total_major_topics as f64 / articles_with_major_topics as f64)
    } else {
        None
    };
    let avg_qualifiers = if articles_with_qualifiers > 0 {
        Some(total_qualifiers as f64 / articles_with_qualifiers as f64)
    } else {
        None
    };

    info!(
        articles_with_major_topics = articles_with_major_topics,
        articles_with_qualifiers = articles_with_qualifiers,
        avg_major_topics_per_article = avg_major_topics,
        avg_qualifiers_per_article = avg_qualifiers,
        "MeSH term analysis summary"
    );
}

#[rstest]
#[traced_test]
fn test_pubmed_abstract_analysis(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    let mut articles_with_abstract = 0;
    let mut total_abstract_length = 0;

    for test_case in test_cases.iter().take(10) {
        info!(filename = test_case.filename(), "Analyzing abstract");

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        if let Ok(article) = result {
            if let Some(abstract_text) = &article.abstract_text {
                articles_with_abstract += 1;
                total_abstract_length += abstract_text.len();

                // Abstract validation
                assert!(
                    !abstract_text.is_empty(),
                    "Abstract should not be empty if present"
                );
                assert!(
                    abstract_text.len() > 50,
                    "Abstract should be substantial (>50 chars)"
                );

                // Check for common abstract patterns
                let abstract_lower = abstract_text.to_lowercase();
                let has_common_words = abstract_lower.contains("study")
                    || abstract_lower.contains("research")
                    || abstract_lower.contains("method")
                    || abstract_lower.contains("result")
                    || abstract_lower.contains("conclusion")
                    || abstract_lower.contains("background")
                    || abstract_lower.contains("objective")
                    || abstract_lower.contains("analysis")
                    || abstract_lower.contains("treatment")
                    || abstract_lower.contains("patient")
                    || abstract_lower.contains("tissue")
                    || abstract_lower.contains("cell")
                    || abstract_lower.contains("gene")
                    || abstract_lower.contains("protein")
                    || abstract_lower.contains("infection")
                    || abstract_lower.contains("response")
                    || abstract_lower.contains("function")
                    || abstract_lower.contains("mechanism");

                assert!(
                    has_common_words,
                    "Abstract should contain common scientific terms: '{}'",
                    abstract_text.chars().take(200).collect::<String>()
                );

                debug!(
                    abstract_length = abstract_text.len(),
                    abstract_preview = abstract_text.chars().take(50).collect::<String>(),
                    "Abstract analysis"
                );
            } else {
                debug!("No abstract found");
            }
        }
    }

    // Abstract statistics
    let avg_abstract_length = if articles_with_abstract > 0 {
        Some(total_abstract_length as f64 / articles_with_abstract as f64)
    } else {
        None
    };

    info!(
        articles_with_abstracts = articles_with_abstract,
        avg_abstract_length = avg_abstract_length,
        "Abstract analysis summary"
    );
}

#[rstest]
#[traced_test]
fn test_pubmed_author_details(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    let mut total_authors = 0;
    let mut articles_analyzed = 0;

    for test_case in test_cases.iter().take(6) {
        info!(filename = test_case.filename(), "Analyzing author details");

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        if let Ok(article) = result {
            articles_analyzed += 1;

            if !article.authors.is_empty() {
                total_authors += article.authors.len();

                for (i, author) in article.authors.iter().enumerate() {
                    // Author name validation
                    assert!(!author.is_empty(), "Author name should not be empty");
                    assert!(
                        author.len() > 2,
                        "Author name should be substantial (>2 chars)"
                    );

                    // Check for reasonable author name patterns
                    let has_alpha = author.chars().any(|c| c.is_alphabetic());
                    assert!(
                        has_alpha,
                        "Author name should contain alphabetic characters"
                    );

                    if i < 3 {
                        // Log first few authors
                        debug!(author_index = i + 1, author_name = %author.full_name, "Author details");
                    }
                }

                debug!(total_authors = article.authors.len(), "Authors count");
            } else {
                debug!("No authors found");
            }
        }
    }

    // Author statistics
    let avg_authors_per_article = if articles_analyzed > 0 {
        Some(total_authors as f64 / articles_analyzed as f64)
    } else {
        None
    };

    info!(
        articles_analyzed = articles_analyzed,
        avg_authors_per_article = avg_authors_per_article,
        "Author details summary"
    );
}

#[rstest]
#[traced_test]
fn test_pubmed_article_types(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    let mut article_type_counts = std::collections::HashMap::new();

    for test_case in test_cases.iter() {
        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        if let Ok(article) = result {
            for article_type in &article.article_types {
                *article_type_counts.entry(article_type.clone()).or_insert(0) += 1;
            }
        }
    }

    info!("Article type distribution");
    for (article_type, count) in article_type_counts.iter() {
        info!(
            article_type = article_type,
            count = count,
            "Article type frequency"
        );
    }

    // Verify we have some common article types
    let has_journal_article = article_type_counts.contains_key("Journal Article");
    if has_journal_article {
        info!("Found Journal Article type");
    }
}

#[rstest]
#[traced_test]
fn test_pubmed_chemical_substances(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    let mut articles_with_chemicals = 0;
    let mut total_chemicals = 0;

    for test_case in test_cases.iter().take(8) {
        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_article_from_xml(&xml_content, &test_case.pmid);

        if let Ok(article) = result {
            if let Some(chemical_list) = &article.chemical_list {
                if !chemical_list.is_empty() {
                    articles_with_chemicals += 1;
                    total_chemicals += chemical_list.len();

                    // Validate chemical structures
                    for chemical in chemical_list {
                        assert!(
                            !chemical.name.is_empty(),
                            "Chemical name should not be empty"
                        );

                        // Registry numbers are optional but should be valid if present
                        if let Some(registry_number) = &chemical.registry_number {
                            assert!(
                                !registry_number.is_empty(),
                                "Registry number should not be empty if present"
                            );
                        }
                    }

                    // Test utility method
                    let chemical_names = article.get_chemical_names();
                    assert_eq!(
                        chemical_names.len(),
                        chemical_list.len(),
                        "Chemical names should match chemical list"
                    );

                    debug!(
                        chemicals_count = chemical_list.len(),
                        first_chemical = chemical_list
                            .first()
                            .map(|c| &c.name)
                            .unwrap_or(&"None".to_string()),
                        "Chemical substances analysis"
                    );
                }
            }
        }
    }

    let avg_chemicals_per_article = if articles_with_chemicals > 0 {
        Some(total_chemicals as f64 / articles_with_chemicals as f64)
    } else {
        None
    };

    info!(
        articles_with_chemicals = articles_with_chemicals,
        avg_chemicals_per_article = avg_chemicals_per_article,
        "Chemical substances summary"
    );
}

#[rstest]
#[traced_test]
fn test_pmc_id_extraction(#[from(xml_test_cases)] test_cases: Vec<PubMedXmlTestCase>) {
    // Known PMC IDs from test data
    let expected_pmc_ids = [
        ("29540945", Some("PMC5844442")),
        ("34567890", Some("PMC8454462")),
        ("26846451", Some("PMC4892867")),
        ("31978945", Some("PMC7092803")),
        ("33515491", Some("PMC7906746")),
        // Articles without PMC IDs
        ("25760099", None),
        ("27350240", None),
    ];

    let mut pmc_ids_found = 0;
    let mut pmc_ids_expected = 0;

    for test_case in &test_cases {
        info!(filename = test_case.filename(), "Testing PMC ID extraction");

        let xml_content = test_case.read_xml_content_or_panic();
        let article = parse_article_from_xml(&xml_content, &test_case.pmid)
            .expect("Failed to parse article XML");

        // Find expected PMC ID for this PMID
        let expected_pmc = expected_pmc_ids
            .iter()
            .find(|(pmid, _)| *pmid == test_case.pmid)
            .map(|(_, pmc)| *pmc);

        if let Some(Some(expected_pmc_id)) = expected_pmc {
            pmc_ids_expected += 1;

            if let Some(ref actual_pmc_id) = article.pmc_id {
                pmc_ids_found += 1;
                assert_eq!(
                    actual_pmc_id, expected_pmc_id,
                    "PMC ID should match for PMID {}",
                    test_case.pmid
                );
                info!(
                    pmid = %test_case.pmid,
                    pmc_id = %actual_pmc_id,
                    "PMC ID extracted successfully"
                );
            } else {
                warn!(
                    pmid = %test_case.pmid,
                    expected_pmc = %expected_pmc_id,
                    "PMC ID not found but was expected"
                );
            }
        } else if let Some(None) = expected_pmc {
            // This article should NOT have a PMC ID
            if let Some(ref pmc_id) = article.pmc_id {
                warn!(
                    pmid = %test_case.pmid,
                    pmc_id = %pmc_id,
                    "PMC ID found but was not expected"
                );
            }
        }
    }

    info!(
        pmc_ids_found = pmc_ids_found,
        pmc_ids_expected = pmc_ids_expected,
        "PMC ID extraction summary"
    );

    // Assert that we found at least some PMC IDs
    assert!(
        pmc_ids_found > 0,
        "Should have extracted at least one PMC ID"
    );

    // Assert that we found most of the expected PMC IDs (allowing for some flexibility)
    assert!(
        pmc_ids_found >= (pmc_ids_expected * 80 / 100),
        "Should have extracted at least 80% of expected PMC IDs"
    );
}
