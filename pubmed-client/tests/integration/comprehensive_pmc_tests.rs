use rstest::*;
use tracing::{debug, info, warn};

use pubmed_client::pmc::parse_pmc_xml;

mod common;
use common::{pmc_xml_test_cases, PmcXmlTestCase};

/// 全XMLファイルを返すフィクスチャ
#[fixture]
fn xml_test_cases() -> Vec<PmcXmlTestCase> {
    pmc_xml_test_cases()
}

#[rstest]
fn test_xml_parsing_basic_validity(#[from(xml_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    for test_case in &test_cases {
        info!(filename = test_case.filename(), "Testing basic parsing");

        let xml_content = test_case.read_xml_content_or_panic();

        // Basic validity checks
        assert!(!xml_content.is_empty(), "XML file should not be empty");
        assert!(
            xml_content.contains("<article"),
            "Should contain article tag"
        );
        assert!(xml_content.contains("PMC"), "Should contain PMC reference");

        info!(filename = test_case.filename(), "Basic validity passed");
    }
}

#[rstest]
fn test_comprehensive_pmc_parsing(#[from(xml_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let mut successful_parses = 0;
    let mut failed_parses = 0;
    let mut parse_errors = Vec::new();

    for test_case in &test_cases {
        info!(
            filename = test_case.filename(),
            "Testing comprehensive parsing"
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);

        match result {
            Ok(article) => {
                successful_parses += 1;

                // Basic validation
                assert!(!article.title.is_empty(), "Article should have a title");
                assert!(!article.pmcid.is_empty(), "Article should have a PMC ID");
                assert_eq!(article.pmcid, test_case.pmcid, "PMC ID should match");

                // Log some statistics
                info!(
                    filename = test_case.filename(),
                    title = article.title.chars().take(60).collect::<String>(),
                    authors_count = article.authors.len(),
                    sections_count = article.sections.len(),
                    references_count = article.references.len(),
                    doi = ?article.doi,
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
        "Comprehensive PMC Parsing Summary"
    );

    if !parse_errors.is_empty() {
        warn!(error_count = parse_errors.len(), "Parse errors occurred");
        for (filename, error) in parse_errors {
            warn!(filename = filename, error = error, "Parse error detail");
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
        info!(filename = test_case.filename(), "Analyzing statistics");

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);

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

            debug!(
                filename = test_case.filename(),
                authors = article.authors.len(),
                sections = article.sections.len(),
                references = article.references.len(),
                "Article statistics"
            );
        }
    }

    // Print statistics
    info!(files_analyzed = successful_parses, "PMC Content Statistics");
    if successful_parses > 0 {
        info!(
            avg_authors = (total_authors as f64 / successful_parses as f64),
            avg_sections = (total_sections as f64 / successful_parses as f64),
            avg_references = (total_references as f64 / successful_parses as f64),
            articles_with_doi = articles_with_doi,
            doi_percentage = ((articles_with_doi as f64 / successful_parses as f64) * 100.0),
            articles_with_pmid = articles_with_pmid,
            pmid_percentage = ((articles_with_pmid as f64 / successful_parses as f64) * 100.0),
            articles_with_keywords = articles_with_keywords,
            keywords_percentage =
                ((articles_with_keywords as f64 / successful_parses as f64) * 100.0),
            articles_with_funding = articles_with_funding,
            funding_percentage =
                ((articles_with_funding as f64 / successful_parses as f64) * 100.0),
            "PMC content analysis summary"
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
        info!(filename = test_case.filename(), "Analyzing author details");

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);

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

            debug!(
                filename = test_case.filename(),
                total_authors = article.authors.len(),
                corresponding_count = corresponding_count,
                affiliation_count = affiliation_count,
                orcid_count = orcid_count,
                "Author details for article"
            );
        }
    }

    // Author statistics summary
    info!(
        total_authors_analyzed = total_authors_analyzed,
        "Author Details Statistics"
    );
    if total_authors_analyzed > 0 {
        info!(
            corresponding_authors = total_corresponding_authors,
            corresponding_percentage =
                ((total_corresponding_authors as f64 / total_authors_analyzed as f64) * 100.0),
            authors_with_affiliations = authors_with_affiliations,
            affiliations_percentage =
                ((authors_with_affiliations as f64 / total_authors_analyzed as f64) * 100.0),
            authors_with_orcid = authors_with_orcid,
            orcid_percentage =
                ((authors_with_orcid as f64 / total_authors_analyzed as f64) * 100.0),
            "Author details summary"
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
        info!(
            filename = test_case.filename(),
            "Analyzing content structure"
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);

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

            debug!(
                filename = test_case.filename(),
                sections_count = article.sections.len(),
                figures_count = figure_count,
                tables_count = table_count,
                has_subsections = has_subsections,
                "Content structure for article"
            );
        }
    }

    // Content structure statistics
    let analyzed_count = test_cases.len().min(5);
    info!(
        articles_analyzed = analyzed_count,
        "Content Structure Statistics"
    );
    if analyzed_count > 0 {
        info!(
            articles_with_figures = articles_with_figures,
            figures_percentage = ((articles_with_figures as f64 / analyzed_count as f64) * 100.0),
            articles_with_tables = articles_with_tables,
            tables_percentage = ((articles_with_tables as f64 / analyzed_count as f64) * 100.0),
            articles_with_subsections = articles_with_subsections,
            subsections_percentage =
                ((articles_with_subsections as f64 / analyzed_count as f64) * 100.0),
            "Content structure distribution"
        );

        if articles_with_figures > 0 {
            info!(
                avg_figures_per_article = (total_figures as f64 / articles_with_figures as f64),
                "Average figures per article (with figures)"
            );
        }
        if articles_with_tables > 0 {
            info!(
                avg_tables_per_article = (total_tables as f64 / articles_with_tables as f64),
                "Average tables per article (with tables)"
            );
        }
    }
}
