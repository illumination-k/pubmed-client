use rstest::*;

use pubmed_client_rs::pmc::PmcXmlParser;

mod common;
use common::{PmcXmlTestCase, pmc_xml_test_cases};

/// 全XMLファイルを返すフィクスチャ
#[fixture]
fn xml_test_cases() -> Vec<PmcXmlTestCase> {
    pmc_xml_test_cases()
}

#[rstest]
fn test_xml_parsing_basic_validity(#[from(xml_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    for test_case in &test_cases {
        println!("Testing basic parsing for: {}", test_case.filename());

        let xml_content = test_case.read_xml_content_or_panic();

        // Basic validity checks
        assert!(!xml_content.is_empty(), "XML file should not be empty");
        assert!(
            xml_content.contains("<article"),
            "Should contain article tag"
        );
        assert!(xml_content.contains("PMC"), "Should contain PMC reference");

        println!("✓ {}: Basic validity passed", test_case.filename());
    }
}

#[rstest]
fn test_comprehensive_pmc_parsing(#[from(xml_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let mut successful_parses = 0;
    let mut failed_parses = 0;
    let mut parse_errors = Vec::new();

    for test_case in &test_cases {
        println!(
            "Testing comprehensive parsing for: {}",
            test_case.filename()
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);

        match result {
            Ok(article) => {
                successful_parses += 1;

                // Basic validation
                assert!(!article.title.is_empty(), "Article should have a title");
                assert!(!article.pmcid.is_empty(), "Article should have a PMC ID");
                assert_eq!(article.pmcid, test_case.pmcid, "PMC ID should match");

                // Log some statistics
                println!(
                    "  Title: {}",
                    article.title.chars().take(60).collect::<String>()
                );
                println!("  Authors: {}", article.authors.len());
                println!("  Sections: {}", article.sections.len());
                println!("  References: {}", article.references.len());

                if article.doi.is_some() {
                    println!("  DOI: {:?}", article.doi);
                }

                println!("✓ {}: Comprehensive parsing passed", test_case.filename());
            }
            Err(e) => {
                failed_parses += 1;
                parse_errors.push((test_case.filename().to_string(), e.to_string()));
                println!("✗ {}: Parsing failed - {}", test_case.filename(), e);
            }
        }
    }

    // Summary
    println!("\n=== Comprehensive PMC Parsing Summary ===");
    println!("Total files tested: {}", test_cases.len());
    println!(
        "Successful parses: {} ({:.1}%)",
        successful_parses,
        (successful_parses as f64 / test_cases.len() as f64) * 100.0
    );
    println!(
        "Failed parses: {} ({:.1}%)",
        failed_parses,
        (failed_parses as f64 / test_cases.len() as f64) * 100.0
    );

    if !parse_errors.is_empty() {
        println!("\nParse Errors:");
        for (filename, error) in parse_errors {
            println!("  {}: {}", filename, error);
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
fn test_pmc_parsing_statistics(#[from(xml_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let mut total_authors = 0;
    let mut total_sections = 0;
    let mut total_references = 0;
    let mut articles_with_doi = 0;
    let mut articles_with_pmid = 0;
    let mut articles_with_keywords = 0;
    let mut articles_with_funding = 0;

    let mut successful_parses = 0;

    for test_case in test_cases.iter().take(10) {
        // Limit to first 10 for performance
        println!("Analyzing statistics for: {}", test_case.filename());

        let xml_content = test_case.read_xml_content_or_panic();

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);

        if let Ok(article) = result {
            successful_parses += 1;
            total_authors += article.authors.len();
            total_sections += article.sections.len();
            total_references += article.references.len();

            if article.doi.is_some() {
                articles_with_doi += 1;
            }
            if article.pmid.is_some() {
                articles_with_pmid += 1;
            }
            if !article.keywords.is_empty() {
                articles_with_keywords += 1;
            }
            if !article.funding.is_empty() {
                articles_with_funding += 1;
            }

            println!(
                "  Authors: {}, Sections: {}, References: {}",
                article.authors.len(),
                article.sections.len(),
                article.references.len()
            );
        }
    }

    // Print statistics
    println!("\n=== PMC Content Statistics ===");
    println!("Files analyzed: {}", successful_parses);
    if successful_parses > 0 {
        println!(
            "Average authors per article: {:.1}",
            total_authors as f64 / successful_parses as f64
        );
        println!(
            "Average sections per article: {:.1}",
            total_sections as f64 / successful_parses as f64
        );
        println!(
            "Average references per article: {:.1}",
            total_references as f64 / successful_parses as f64
        );
        println!(
            "Articles with DOI: {} ({:.1}%)",
            articles_with_doi,
            (articles_with_doi as f64 / successful_parses as f64) * 100.0
        );
        println!(
            "Articles with PMID: {} ({:.1}%)",
            articles_with_pmid,
            (articles_with_pmid as f64 / successful_parses as f64) * 100.0
        );
        println!(
            "Articles with keywords: {} ({:.1}%)",
            articles_with_keywords,
            (articles_with_keywords as f64 / successful_parses as f64) * 100.0
        );
        println!(
            "Articles with funding: {} ({:.1}%)",
            articles_with_funding,
            (articles_with_funding as f64 / successful_parses as f64) * 100.0
        );
    }
}

#[rstest]
fn test_pmc_parsing_author_details(#[from(xml_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let mut total_corresponding_authors = 0;
    let mut authors_with_affiliations = 0;
    let mut authors_with_orcid = 0;
    let mut total_authors_analyzed = 0;

    for test_case in test_cases.iter().take(5) {
        // Limit for performance
        println!("Analyzing author details for: {}", test_case.filename());

        let xml_content = test_case.read_xml_content_or_panic();

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);

        if let Ok(article) = result {
            for author in &article.authors {
                total_authors_analyzed += 1;

                if author.is_corresponding {
                    total_corresponding_authors += 1;
                }
                if !author.affiliations.is_empty() {
                    authors_with_affiliations += 1;
                }
                if author.orcid.is_some() {
                    authors_with_orcid += 1;
                }
            }

            println!("  Total authors: {}", article.authors.len());
            let corresponding_count = article
                .authors
                .iter()
                .filter(|a| a.is_corresponding)
                .count();
            let affiliation_count = article
                .authors
                .iter()
                .filter(|a| !a.affiliations.is_empty())
                .count();
            let orcid_count = article.authors.iter().filter(|a| a.orcid.is_some()).count();

            println!(
                "    Corresponding: {}, With affiliations: {}, With ORCID: {}",
                corresponding_count, affiliation_count, orcid_count
            );
        }
    }

    // Author statistics summary
    println!("\n=== Author Details Statistics ===");
    println!("Total authors analyzed: {}", total_authors_analyzed);
    if total_authors_analyzed > 0 {
        println!(
            "Corresponding authors: {} ({:.1}%)",
            total_corresponding_authors,
            (total_corresponding_authors as f64 / total_authors_analyzed as f64) * 100.0
        );
        println!(
            "Authors with affiliations: {} ({:.1}%)",
            authors_with_affiliations,
            (authors_with_affiliations as f64 / total_authors_analyzed as f64) * 100.0
        );
        println!(
            "Authors with ORCID: {} ({:.1}%)",
            authors_with_orcid,
            (authors_with_orcid as f64 / total_authors_analyzed as f64) * 100.0
        );
    }
}

#[rstest]
fn test_pmc_parsing_content_structure(#[from(xml_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let mut articles_with_figures = 0;
    let mut articles_with_tables = 0;
    let mut articles_with_subsections = 0;
    let mut total_figures = 0;
    let mut total_tables = 0;

    for test_case in test_cases.iter().take(5) {
        // Limit for performance
        println!("Analyzing content structure for: {}", test_case.filename());

        let xml_content = test_case.read_xml_content_or_panic();

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);

        if let Ok(article) = result {
            let mut has_figures = false;
            let mut has_tables = false;
            let mut has_subsections = false;
            let mut figure_count = 0;
            let mut table_count = 0;

            for section in &article.sections {
                if !section.figures.is_empty() {
                    has_figures = true;
                    figure_count += section.figures.len();
                }
                if !section.tables.is_empty() {
                    has_tables = true;
                    table_count += section.tables.len();
                }
                if !section.subsections.is_empty() {
                    has_subsections = true;
                }
            }

            if has_figures {
                articles_with_figures += 1;
                total_figures += figure_count;
            }
            if has_tables {
                articles_with_tables += 1;
                total_tables += table_count;
            }
            if has_subsections {
                articles_with_subsections += 1;
            }

            println!(
                "  Sections: {}, Figures: {}, Tables: {}, Has subsections: {}",
                article.sections.len(),
                figure_count,
                table_count,
                has_subsections
            );
        }
    }

    // Content structure statistics
    println!("\n=== Content Structure Statistics ===");
    let analyzed_count = test_cases.len().min(5);
    println!("Articles analyzed: {}", analyzed_count);
    if analyzed_count > 0 {
        println!(
            "Articles with figures: {} ({:.1}%)",
            articles_with_figures,
            (articles_with_figures as f64 / analyzed_count as f64) * 100.0
        );
        println!(
            "Articles with tables: {} ({:.1}%)",
            articles_with_tables,
            (articles_with_tables as f64 / analyzed_count as f64) * 100.0
        );
        println!(
            "Articles with subsections: {} ({:.1}%)",
            articles_with_subsections,
            (articles_with_subsections as f64 / analyzed_count as f64) * 100.0
        );

        if articles_with_figures > 0 {
            println!(
                "Average figures per article (with figures): {:.1}",
                total_figures as f64 / articles_with_figures as f64
            );
        }
        if articles_with_tables > 0 {
            println!(
                "Average tables per article (with tables): {:.1}",
                total_tables as f64 / articles_with_tables as f64
            );
        }
    }
}
