//! Comprehensive PMC (PubMed Central) API integration tests
//!
//! These tests make actual network calls to PMC APIs to verify
//! real-world functionality and catch API changes.
//!
//! **IMPORTANT**: These tests are only run when:
//! 1. The `integration-tests` feature is enabled
//! 2. The `PUBMED_REAL_API_TESTS` environment variable is set
//!
//! To run these tests:
//! ```bash
//! PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pmc_api_tests
//! ```
//!
//! With API key for higher rate limits:
//! ```bash
//! PUBMED_REAL_API_TESTS=1 NCBI_API_KEY=your_key cargo test --features integration-tests --test pmc_api_tests
//! ```

mod common;

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use std::time::{Duration, Instant};
    use tokio::time::sleep;
    use tracing::{debug, info, warn};
    use tracing_test::traced_test;

    use pubmed_client::pmc::markdown::PmcMarkdownConverter;

    // Import test utilities
    use crate::common::integration_test_utils::{
        create_test_pmc_client, create_test_pubmed_client, should_run_real_api_tests, TEST_PMCIDS,
    };

    /// Test fetching PMC full-text articles
    #[tokio::test]
    #[traced_test]
    async fn test_fetch_pmc_fulltext_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PMC full-text fetching integration");

        let client = create_test_pmc_client();

        for pmcid in TEST_PMCIDS.iter().take(3) {
            info!(pmcid = pmcid, "Testing PMC full-text fetch");

            let start_time = Instant::now();
            match client.fetch_full_text(pmcid).await {
                Ok(fulltext) => {
                    let duration = start_time.elapsed();
                    // Count figures and tables across all sections
                    let mut total_figures = 0;
                    let mut total_tables = 0;
                    let mut has_abstract = false;

                    for section in &fulltext.sections {
                        total_figures += section.figures.len();
                        total_tables += section.tables.len();
                        if section.section_type == "abstract" {
                            has_abstract = true;
                        }
                    }

                    info!(
                        pmcid = pmcid,
                        title_length = fulltext.title.len(),
                        authors_count = fulltext.authors.len(),
                        sections_count = fulltext.sections.len(),
                        figures_count = total_figures,
                        tables_count = total_tables,
                        has_abstract = has_abstract,
                        duration_ms = duration.as_millis(),
                        "PMC full-text fetch successful"
                    );

                    // Verify required fields
                    assert!(!fulltext.title.is_empty(), "PMC article should have title");
                    assert!(
                        !fulltext.authors.is_empty(),
                        "PMC article should have authors"
                    );
                    assert!(
                        !fulltext.sections.is_empty(),
                        "PMC article should have sections"
                    );

                    // Verify PMCID format
                    assert!(pmcid.starts_with("PMC"), "PMCID should start with PMC");

                    // Verify section structure
                    for (i, section) in fulltext.sections.iter().take(3).enumerate() {
                        if let Some(ref title) = section.title {
                            assert!(!title.is_empty(), "Section title should not be empty");
                        }
                        assert!(!section.content.is_empty(), "Section should have content");
                        debug!(
                            section_index = i,
                            section_title = ?section.title,
                            section_type = %section.section_type,
                            content_length = section.content.len(),
                            "Section details"
                        );
                    }

                    // Log title preview for verification
                    let title_preview = if fulltext.title.len() > 100 {
                        let preview = &fulltext.title[..100];
                        format!("{preview}...")
                    } else {
                        fulltext.title.clone()
                    };
                    debug!(pmcid = pmcid, title_preview = %title_preview, "PMC article title");
                }
                Err(e) => {
                    warn!(pmcid = pmcid, error = %e, "PMC full-text fetch failed");
                    // Don't panic here as some PMCIDs might not be available
                    info!("PMC article not available (expected for some PMCIDs)");
                }
            }

            sleep(Duration::from_millis(200)).await;
        }
    }

    /// Test PMC article conversion to Markdown
    #[tokio::test]
    #[traced_test]
    async fn test_pmc_to_markdown_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PMC to Markdown conversion integration");

        let client = create_test_pmc_client();

        // Try to find an available PMC article
        for pmcid in TEST_PMCIDS.iter().take(3) {
            info!(pmcid = pmcid, "Testing PMC to Markdown conversion");

            match client.fetch_full_text(pmcid).await {
                Ok(fulltext) => {
                    info!(
                        pmcid = pmcid,
                        "Successfully fetched PMC article for Markdown test"
                    );

                    // Convert to Markdown
                    let converter = PmcMarkdownConverter::new();
                    let markdown = converter.convert(&fulltext);

                    info!(
                        pmcid = pmcid,
                        markdown_length = markdown.len(),
                        "Markdown conversion successful"
                    );

                    // Verify Markdown structure
                    assert!(!markdown.is_empty(), "Markdown should not be empty");
                    assert!(markdown.contains("#"), "Markdown should contain headers");

                    // Should contain title
                    assert!(
                        markdown.contains(&fulltext.title)
                            || markdown.contains(&fulltext.title.replace(" ", "")),
                        "Markdown should contain article title"
                    );

                    // Should contain author information
                    if !fulltext.authors.is_empty() {
                        let first_author = &fulltext.authors[0];
                        let author_name = format!(
                            "{} {}",
                            first_author.given_names.as_deref().unwrap_or(""),
                            first_author.surname.as_deref().unwrap_or("")
                        );
                        assert!(
                            markdown.contains(author_name.trim())
                                || markdown.contains(first_author.surname.as_deref().unwrap_or(""))
                                || markdown.contains(&first_author.full_name),
                            "Markdown should contain author information"
                        );
                    }

                    // Should contain section content
                    if !fulltext.sections.is_empty() {
                        let first_section = &fulltext.sections[0];
                        let title_check = if let Some(ref title) = first_section.title {
                            markdown.contains(title)
                        } else {
                            true // Skip title check if no title
                        };

                        let content_preview =
                            &first_section.content[..50.min(first_section.content.len())];
                        assert!(
                            title_check || markdown.contains(content_preview),
                            "Markdown should contain section content"
                        );
                    }

                    debug!(
                        pmcid = pmcid,
                        markdown_preview = &markdown[..200.min(markdown.len())],
                        "Markdown preview"
                    );

                    // Only test one successful conversion
                    return;
                }
                Err(e) => {
                    debug!(pmcid = pmcid, error = %e, "PMC article not available, trying next");
                }
            }

            sleep(Duration::from_millis(200)).await;
        }

        warn!("No PMC articles were available for Markdown conversion test");
    }

    /// Test PMC article structure and content validation
    #[tokio::test]
    #[traced_test]
    async fn test_pmc_content_structure_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PMC content structure integration");

        let client = create_test_pmc_client();

        // Find an available PMC article with rich content
        for pmcid in TEST_PMCIDS.iter().take(5) {
            match client.fetch_full_text(pmcid).await {
                Ok(fulltext) => {
                    info!(
                        pmcid = pmcid,
                        "Found available PMC article for structure validation"
                    );

                    // Test author structure
                    if !fulltext.authors.is_empty() {
                        let author = &fulltext.authors[0];
                        assert!(!author.full_name.is_empty(), "Author should have name");

                        debug!(
                            first_author_given_names = ?author.given_names,
                            first_author_surname = ?author.surname,
                            first_author_full_name = %author.full_name,
                            affiliations = ?author.affiliations,
                            "First author details"
                        );
                    }

                    // Test section structure
                    let mut introduction_found = false;
                    let mut methods_found = false;
                    let mut results_found = false;
                    let mut discussion_found = false;

                    for section in &fulltext.sections {
                        let section_type_lower = section.section_type.to_lowercase();
                        let title_lower = section
                            .title
                            .as_ref()
                            .map(|t| t.to_lowercase())
                            .unwrap_or_default();

                        if section_type_lower.contains("introduction")
                            || title_lower.contains("introduction")
                            || section_type_lower.contains("background")
                            || title_lower.contains("background")
                        {
                            introduction_found = true;
                        } else if section_type_lower.contains("method")
                            || title_lower.contains("method")
                            || section_type_lower.contains("material")
                            || title_lower.contains("material")
                        {
                            methods_found = true;
                        } else if section_type_lower.contains("result")
                            || title_lower.contains("result")
                            || section_type_lower.contains("finding")
                            || title_lower.contains("finding")
                        {
                            results_found = true;
                        } else if section_type_lower.contains("discussion")
                            || title_lower.contains("discussion")
                            || section_type_lower.contains("conclusion")
                            || title_lower.contains("conclusion")
                        {
                            discussion_found = true;
                        }

                        // Verify section content is not empty
                        assert!(
                            !section.content.is_empty(),
                            "Section content should not be empty"
                        );
                        assert!(
                            section.content.len() > 10,
                            "Section content should be substantial"
                        );
                    }

                    info!(
                        pmcid = pmcid,
                        sections_count = fulltext.sections.len(),
                        introduction_found = introduction_found,
                        methods_found = methods_found,
                        results_found = results_found,
                        discussion_found = discussion_found,
                        "PMC article structure analysis"
                    );

                    // Test figures and tables from sections
                    let mut total_figures = 0;
                    let mut total_tables = 0;

                    for section in &fulltext.sections {
                        total_figures += section.figures.len();
                        total_tables += section.tables.len();

                        if !section.figures.is_empty() {
                            let figure = &section.figures[0];
                            assert!(!figure.caption.is_empty(), "Figure should have caption");

                            debug!(
                                figure_id = %figure.id,
                                caption_length = figure.caption.len(),
                                "Figure details"
                            );
                        }

                        if !section.tables.is_empty() {
                            let table = &section.tables[0];
                            assert!(!table.caption.is_empty(), "Table should have caption");

                            debug!(
                                table_id = %table.id,
                                caption_length = table.caption.len(),
                                "Table details"
                            );
                        }
                    }

                    debug!(
                        total_figures = total_figures,
                        total_tables = total_tables,
                        "Total figures and tables across all sections"
                    );

                    // Only analyze one successful article
                    return;
                }
                Err(e) => {
                    debug!(pmcid = pmcid, error = %e, "PMC article not available, trying next");
                }
            }

            sleep(Duration::from_millis(200)).await;
        }

        warn!("No PMC articles were available for content structure test");
    }

    /// Test PMC API error handling
    #[tokio::test]
    #[traced_test]
    async fn test_pmc_error_handling_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PMC error handling integration");

        let client = create_test_pmc_client();

        // Test with invalid PMCID
        let invalid_pmcids = [
            "PMC999999999", // Very high number, likely doesn't exist
            "PMC0",         // Invalid format
            "PMC00000001",  // Invalid format with leading zeros
        ];

        for invalid_pmcid in &invalid_pmcids {
            info!(invalid_pmcid = invalid_pmcid, "Testing invalid PMCID");

            match client.fetch_full_text(invalid_pmcid).await {
                Ok(_) => {
                    warn!(
                        invalid_pmcid = invalid_pmcid,
                        "Unexpected success with invalid PMCID"
                    );
                    // This might happen if the "invalid" PMCID actually exists
                }
                Err(e) => {
                    info!(
                        invalid_pmcid = invalid_pmcid,
                        error = %e,
                        "Expected error for invalid PMCID"
                    );

                    // Verify error message is descriptive
                    let error_str = e.to_string();
                    assert!(!error_str.is_empty(), "Error message should not be empty");
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Test PMC rate limiting compliance
    #[tokio::test]
    #[traced_test]
    async fn test_pmc_rate_limiting_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PMC rate limiting integration");

        let client = create_test_pmc_client();
        let start_time = Instant::now();

        // Make several requests to test rate limiting
        let mut successful_requests = 0;
        let mut failed_requests = 0;

        for (i, pmcid) in TEST_PMCIDS.iter().enumerate().take(5) {
            info!(
                request_number = i + 1,
                pmcid = pmcid,
                "Making PMC request for rate limiting test"
            );

            let request_start = Instant::now();
            match client.fetch_full_text(pmcid).await {
                Ok(_) => {
                    successful_requests += 1;
                    let request_duration = request_start.elapsed();
                    debug!(
                        pmcid = pmcid,
                        request_duration_ms = request_duration.as_millis(),
                        "PMC request successful"
                    );
                }
                Err(e) => {
                    failed_requests += 1;
                    debug!(
                        pmcid = pmcid,
                        error = %e,
                        "PMC request failed (may be unavailable)"
                    );
                }
            }
        }

        let total_duration = start_time.elapsed();

        info!(
            total_requests = TEST_PMCIDS.len().min(5),
            successful_requests = successful_requests,
            failed_requests = failed_requests,
            total_duration_ms = total_duration.as_millis(),
            "PMC rate limiting test completed"
        );

        // Verify rate limiting is working (should take reasonable time)
        let min_expected_duration = Duration::from_millis(1500); // At least 1.5 seconds for 5 requests
        assert!(
            total_duration >= min_expected_duration,
            "PMC requests completed too quickly - rate limiting may not be working"
        );

        // At least some requests should work (even if articles are unavailable)
        // The important thing is that the rate limiting and API calls function
        info!("PMC rate limiting test completed - verified proper timing behavior");
    }

    /// Test PMC combined workflow with search and fetch
    #[tokio::test]
    #[traced_test]
    async fn test_pmc_combined_workflow_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PMC combined workflow integration");

        // Create both PubMed and PMC clients for the workflow
        let pubmed_client = create_test_pubmed_client();
        let pmc_client = create_test_pmc_client();

        // Step 1: Search for articles that might have PMC full text
        let search_query = "open access[Filter] AND COVID-19[Title] AND 2023[PDAT]";

        info!(
            query = search_query,
            "Step 1: Searching for open access articles"
        );

        let pmids = match pubmed_client.search_articles(search_query, 10).await {
            Ok(pmids) => {
                info!(results_count = pmids.len(), "Search completed");
                pmids
            }
            Err(e) => {
                warn!(error = %e, "Search failed");
                return; // Skip rest of test
            }
        };

        if pmids.is_empty() {
            info!("No articles found, ending workflow test");
            return;
        }

        // Convert PMIDs to u32 for ELink API
        let pmids_u32: Vec<u32> = pmids
            .iter()
            .filter_map(|pmid| pmid.parse::<u32>().ok())
            .collect();

        if pmids_u32.is_empty() {
            info!("No valid PMIDs found for ELink, ending workflow test");
            return;
        }

        // Step 2: Get PMC links for the found articles
        info!("Step 2: Getting PMC links for found articles");

        let pmc_links = match pubmed_client
            .get_pmc_links(&pmids_u32[..5.min(pmids_u32.len())])
            .await
        {
            Ok(links) => {
                info!(
                    pmc_articles_count = links.pmc_ids.len(),
                    "PMC links retrieved"
                );
                links
            }
            Err(e) => {
                warn!(error = %e, "Failed to get PMC links");
                return;
            }
        };

        if pmc_links.pmc_ids.is_empty() {
            info!("No PMC full-text available for searched articles");
            return;
        }

        // Step 3: Attempt to fetch PMC full text
        info!("Step 3: Fetching PMC full text");

        let mut pmc_articles_fetched = 0;

        for (i, _pmc_id) in pmc_links.pmc_ids.iter().take(2).enumerate() {
            // Convert PMID to PMCID format - this is a simplification
            // In reality, you'd need proper PMID->PMCID mapping
            info!(attempt = i + 1, "Attempting to find PMC article");

            // For this test, we'll try our known working PMCIDs
            if let Some(pmcid) = TEST_PMCIDS.get(i) {
                match pmc_client.fetch_full_text(pmcid).await {
                    Ok(fulltext) => {
                        pmc_articles_fetched += 1;
                        info!(
                            pmcid = pmcid,
                            title_length = fulltext.title.len(),
                            sections_count = fulltext.sections.len(),
                            "PMC full text retrieved successfully"
                        );

                        // Verify we can convert to markdown
                        let converter = PmcMarkdownConverter::new();
                        let markdown = converter.convert(&fulltext);
                        assert!(
                            !markdown.is_empty(),
                            "Should be able to convert to markdown"
                        );

                        debug!(
                            pmcid = pmcid,
                            markdown_length = markdown.len(),
                            "Markdown conversion successful"
                        );
                    }
                    Err(e) => {
                        debug!(pmcid = pmcid, error = %e, "PMC article not available");
                    }
                }
            }

            sleep(Duration::from_millis(200)).await;
        }

        info!(
            workflow_summary = "completed",
            pmids_found = pmids.len(),
            pmc_links_found = pmc_links.pmc_ids.len(),
            pmc_articles_fetched = pmc_articles_fetched,
            "Combined workflow test completed"
        );

        // The workflow is successful if we can execute all steps without errors
        // Even if no full text is available, the API integration is working
        assert!(!pmids.is_empty(), "Should find some articles in search");
    }
}

// Placeholder module for non-integration builds
#[cfg(not(feature = "integration-tests"))]
mod placeholder {
    //! Integration tests are only available with the `integration-tests` feature.
    //!
    //! To run these tests:
    //! ```bash
    //! cargo test --features integration-tests --test pmc_api_tests
    //! ```
}
