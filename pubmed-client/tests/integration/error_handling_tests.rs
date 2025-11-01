//! Comprehensive error handling integration tests
//!
//! These tests verify that the library properly handles real-world error scenarios
//! from NCBI APIs, including network issues, invalid requests, and server errors.
//!
//! **IMPORTANT**: These tests are only run when:
//! 1. The `integration-tests` feature is enabled
//! 2. The `PUBMED_REAL_API_TESTS` environment variable is set
//!
//! To run these tests:
//! ```bash
//! PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test error_handling_tests
//! ```

mod common;

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use pubmed_client::time::{sleep, Duration};
    use tracing::{debug, info, warn};
    use tracing_test::traced_test;

    use pubmed_client::{ClientConfig, PubMedClient, PubMedError};

    // Import test utilities
    use crate::common::integration_test_utils::{
        create_test_pmc_client, create_test_pubmed_client, should_run_real_api_tests,
    };

    /// Test handling of invalid search queries
    #[tokio::test]
    #[traced_test]
    async fn test_invalid_search_queries_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing invalid search queries error handling");

        let client = create_test_pubmed_client();

        let invalid_queries = [
            "",                                                        // Empty query
            "   ",                                                     // Whitespace only
            "[[[invalid]]brackets",                                    // Malformed brackets
            "AND OR NOT",                                              // Invalid boolean logic
            "field[NonExistentField]",                                 // Invalid field
            "title[Abstract]",                                         // Wrong field mapping
            &String::from_utf8(vec![b'\xFF'; 10]).unwrap_or_default(), // Invalid UTF-8 chars
        ];

        for query in &invalid_queries {
            if query.is_empty() {
                continue; // Skip empty query as it might be handled specially
            }

            info!(query = query, "Testing invalid query");

            match client.search_articles(query, 10).await {
                Ok(results) => {
                    // Some "invalid" queries might actually return results
                    // This is not necessarily an error - NCBI might be forgiving
                    info!(
                        query = query,
                        results_count = results.len(),
                        "Query returned results (NCBI was forgiving)"
                    );
                }
                Err(e) => {
                    info!(
                        query = query,
                        error = %e,
                        "Query properly rejected with error"
                    );

                    // Verify error handling
                    match &e {
                        PubMedError::ApiError { status, message } => {
                            assert!(!message.is_empty(), "Error message should not be empty");
                            debug!(status = status, message = message, "API error details");
                        }
                        PubMedError::InvalidQuery(_) => {
                            info!("Query validation caught invalid query");
                        }
                        _ => {
                            debug!(error_type = ?e, "Other error type for invalid query");
                        }
                    }
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Test handling of non-existent or invalid PMIDs
    #[tokio::test]
    #[traced_test]
    async fn test_invalid_pmids_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing invalid PMIDs error handling");

        let client = create_test_pubmed_client();

        let invalid_pmids = [
            "0",                   // Invalid PMID (PMIDs start from 1)
            "999999999",           // Very high PMID, likely doesn't exist
            "abc123",              // Non-numeric PMID
            "-123",                // Negative PMID
            "12.34",               // Decimal PMID
            "1234567890123456789", // Extremely long number
        ];

        for pmid in &invalid_pmids {
            info!(pmid = pmid, "Testing invalid PMID");

            match client.fetch_article(pmid).await {
                Ok(article) => {
                    warn!(
                        pmid = pmid,
                        article_title = %article.title,
                        "Unexpected success with invalid PMID"
                    );
                    // This might happen if the "invalid" PMID actually exists
                }
                Err(e) => {
                    info!(
                        pmid = pmid,
                        error = %e,
                        "PMID properly rejected with error"
                    );

                    // Verify error handling
                    match &e {
                        PubMedError::ArticleNotFound { pmid: _ } => {
                            info!("Article not found error (expected)");
                        }
                        PubMedError::ApiError { status, message } => {
                            assert!(!message.is_empty(), "Error message should not be empty");
                            debug!(status = status, message = message, "API error details");
                        }
                        PubMedError::InvalidPmid { pmid: _ } => {
                            info!("Invalid PMID error (expected)");
                        }
                        _ => {
                            debug!(error_type = ?e, "Other error type for invalid PMID");
                        }
                    }
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Test handling of invalid PMCIDs
    #[tokio::test]
    #[traced_test]
    async fn test_invalid_pmcids_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing invalid PMCIDs error handling");

        let client = create_test_pmc_client();

        let invalid_pmcids = [
            "PMC0",         // Invalid PMCID (should be positive)
            "PMC999999999", // Very high PMCID, likely doesn't exist
            "PMC",          // Incomplete PMCID
            "123456",       // Missing PMC prefix
            "PMCabc123",    // Non-numeric part
            "pmc123456",    // Lowercase (might be handled)
            "PMC-123456",   // Invalid format with dash
        ];

        for pmcid in &invalid_pmcids {
            info!(pmcid = pmcid, "Testing invalid PMCID");

            match client.fetch_full_text(pmcid).await {
                Ok(fulltext) => {
                    warn!(
                        pmcid = pmcid,
                        article_title = %fulltext.title,
                        "Unexpected success with invalid PMCID"
                    );
                    // This might happen if the "invalid" PMCID actually exists
                    // or if NCBI normalizes the format
                }
                Err(e) => {
                    info!(
                        pmcid = pmcid,
                        error = %e,
                        "PMCID properly rejected with error"
                    );

                    // Verify error is descriptive
                    let error_str = e.to_string();
                    assert!(!error_str.is_empty(), "Error message should not be empty");
                }
            }

            sleep(Duration::from_millis(150)).await;
        }
    }

    /// Test network timeout and connectivity error handling
    #[tokio::test]
    #[traced_test]
    async fn test_network_timeout_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing network timeout error handling");

        // Create client with very short timeout to trigger timeout errors
        let config = ClientConfig::new()
            .with_email("test@example.com")
            .with_tool("pubmed-client-rs-timeout-test")
            .with_timeout(Duration::from_millis(1)) // Extremely short timeout
            .with_rate_limit(1.0);

        let client = PubMedClient::with_config(config);

        info!("Testing search with short timeout");

        match client.search_articles("COVID-19", 5).await {
            Ok(results) => {
                // This might succeed if the request is very fast
                info!(
                    results_count = results.len(),
                    "Request succeeded despite short timeout (network was very fast)"
                );
            }
            Err(e) => {
                info!(error = %e, "Timeout error properly handled");

                // Verify it's a timeout-related error
                let error_str = e.to_string().to_lowercase();
                let is_timeout_error = error_str.contains("timeout")
                    || error_str.contains("connection")
                    || error_str.contains("network");

                if is_timeout_error {
                    info!("Confirmed timeout error");
                } else {
                    debug!(error_string = %error_str, "Non-timeout error (may be expected)");
                }
            }
        }
    }

    /// Test concurrent request error handling and rate limiting
    #[tokio::test]
    #[traced_test]
    async fn test_concurrent_error_handling_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing concurrent request error handling");

        let client = create_test_pubmed_client();

        // Spawn multiple concurrent requests with mix of valid and invalid queries
        let queries = [
            ("valid_query_1", "COVID-19[Title]"),
            ("invalid_query_1", "[[[malformed"),
            ("valid_query_2", "cancer[Title]"),
            ("invalid_query_2", ""),
            ("valid_query_3", "diabetes[Title]"),
            ("invalid_pmid", "999999999"), // This will be used as PMID fetch
        ];

        let mut tasks = Vec::new();

        for (label, query) in queries.iter() {
            let client = client.clone();
            let label = label.to_string();
            let query = query.to_string();

            let task = tokio::spawn(async move {
                if label.contains("invalid_pmid") {
                    // Test PMID fetch for one task
                    info!(task_label = %label, "Testing concurrent PMID fetch");
                    client
                        .fetch_article(&query)
                        .await
                        .map(|_| 1)
                        .map_err(|e| (label, e))
                } else if query.is_empty() {
                    // Skip empty queries
                    info!(task_label = %label, "Skipping empty query");
                    Ok(0)
                } else {
                    // Test search for other tasks
                    info!(task_label = %label, query = %query, "Testing concurrent search");
                    client
                        .search_articles(&query, 3)
                        .await
                        .map(|r| r.len())
                        .map_err(|e| (label, e))
                }
            });

            tasks.push(task);

            // Slight delay between spawning tasks
            sleep(Duration::from_millis(50)).await;
        }

        // Collect results
        let mut successful_tasks = 0;
        let mut failed_tasks = 0;
        let mut error_types = std::collections::HashMap::new();

        for task in tasks {
            match task.await {
                Ok(Ok(result_count)) => {
                    successful_tasks += 1;
                    debug!(result_count = result_count, "Task completed successfully");
                }
                Ok(Err((label, e))) => {
                    failed_tasks += 1;
                    let error_type = match &e {
                        PubMedError::ApiError { .. } => "api_error",
                        PubMedError::InvalidQuery(_) => "invalid_query",
                        PubMedError::ArticleNotFound { pmid: _ } => "article_not_found",
                        PubMedError::RateLimitExceeded => "rate_limit",
                        _ => "other",
                    };

                    *error_types.entry(error_type).or_insert(0) += 1;

                    info!(
                        task_label = %label,
                        error = %e,
                        error_type = error_type,
                        "Task failed with expected error"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "Task panicked");
                }
            }
        }

        info!(
            total_tasks = queries.len(),
            successful_tasks = successful_tasks,
            failed_tasks = failed_tasks,
            error_types = ?error_types,
            "Concurrent error handling test completed"
        );

        // Verify that we handled both success and failure cases
        assert!(successful_tasks > 0, "Some tasks should succeed");
        assert!(
            failed_tasks > 0,
            "Some tasks should fail (expected for invalid queries)"
        );
    }

    /// Test ELink API error handling with invalid inputs
    #[tokio::test]
    #[traced_test]
    async fn test_elink_error_handling_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing ELink API error handling");

        let client = create_test_pubmed_client();

        // Test with empty PMID list
        info!("Testing ELink with empty PMID list");
        match client.get_related_articles(&[]).await {
            Ok(related) => {
                info!(
                    related_count = related.related_pmids.len(),
                    "ELink succeeded with empty input (returned empty results)"
                );
                assert!(
                    related.related_pmids.is_empty(),
                    "Should return empty results for empty input"
                );
            }
            Err(e) => {
                info!(error = %e, "ELink properly rejected empty input");
            }
        }

        // Test with invalid PMIDs - convert to u32 where possible
        let invalid_pmids_str = ["0", "abc123", "999999999"];
        let invalid_pmids: Vec<u32> = invalid_pmids_str
            .iter()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect();

        info!("Testing ELink with invalid PMIDs");
        match client.get_related_articles(&invalid_pmids).await {
            Ok(related) => {
                info!(
                    related_count = related.related_pmids.len(),
                    "ELink succeeded with invalid PMIDs (may have filtered them out)"
                );
                // NCBI might filter out invalid PMIDs and return results for valid ones
            }
            Err(e) => {
                info!(error = %e, "ELink properly handled invalid PMIDs");
            }
        }

        // Test PMC links with invalid PMIDs
        info!("Testing ELink PMC links with invalid PMIDs");
        match client.get_pmc_links(&invalid_pmids).await {
            Ok(pmc_links) => {
                info!(
                    pmc_links_count = pmc_links.pmc_ids.len(),
                    "ELink PMC links succeeded with invalid PMIDs"
                );
            }
            Err(e) => {
                info!(error = %e, "ELink PMC links properly handled invalid PMIDs");
            }
        }

        // Test citations with invalid PMIDs
        info!("Testing ELink citations with invalid PMIDs");
        match client.get_citations(&invalid_pmids).await {
            Ok(citations) => {
                info!(
                    citations_count = citations.citing_pmids.len(),
                    "ELink citations succeeded with invalid PMIDs"
                );
            }
            Err(e) => {
                info!(error = %e, "ELink citations properly handled invalid PMIDs");
            }
        }
    }

    /// Test API error responses and status codes
    #[tokio::test]
    #[traced_test]
    async fn test_api_error_responses_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing API error responses");

        let client = create_test_pubmed_client();

        // Test various scenarios that might trigger API errors
        let error_scenarios = [
            ("empty_search", ""),
            ("malformed_brackets", "[[[invalid"),
            ("very_long_query", &"x".repeat(10000)), // Extremely long query
        ];

        for (scenario, query) in &error_scenarios {
            if query.is_empty() {
                continue; // Skip empty queries
            }

            info!(scenario = scenario, "Testing error scenario");

            match client.search_articles(query, 10).await {
                Ok(results) => {
                    info!(
                        scenario = scenario,
                        results_count = results.len(),
                        "Scenario succeeded unexpectedly (API was forgiving)"
                    );
                }
                Err(e) => {
                    info!(
                        scenario = scenario,
                        error = %e,
                        "Scenario produced expected error"
                    );

                    // Verify error information is available
                    match &e {
                        PubMedError::ApiError { status, message } => {
                            assert!(
                                *status > 0 || !message.is_empty(),
                                "API error should have status or message"
                            );
                            debug!(
                                scenario = scenario,
                                status = ?status,
                                message = message,
                                "API error details"
                            );
                        }
                        _ => {
                            debug!(
                                scenario = scenario,
                                error_type = ?e,
                                "Non-API error type"
                            );
                        }
                    }
                }
            }

            sleep(Duration::from_millis(200)).await;
        }
    }

    /// Test recovery from transient errors
    #[tokio::test]
    #[traced_test]
    async fn test_error_recovery_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing error recovery and retry behavior");

        let client = create_test_pubmed_client();

        // Test that valid requests work after invalid ones
        let mixed_requests = [
            ("invalid_1", "999999999"),     // Invalid PMID
            ("valid_1", "31978945"),        // Valid PMID
            ("invalid_2", "[[[bad"),        // Invalid search
            ("valid_2", "COVID-19[Title]"), // Valid search
        ];

        let mut successful_requests = 0;
        let mut failed_requests = 0;

        for (label, input) in &mixed_requests {
            info!(label = label, input = input, "Testing mixed request");

            let result = if label.contains("invalid_1") || label.contains("valid_1") {
                // PMID fetch
                client.fetch_article(input).await.map(|_| 1)
            } else {
                // Search
                client.search_articles(input, 5).await.map(|r| r.len())
            };

            match result {
                Ok(count) => {
                    successful_requests += 1;
                    info!(label = label, result_count = count, "Request succeeded");
                }
                Err(e) => {
                    failed_requests += 1;
                    info!(
                        label = label,
                        error = %e,
                        "Request failed (expected for invalid inputs)"
                    );
                }
            }

            sleep(Duration::from_millis(200)).await;
        }

        info!(
            successful_requests = successful_requests,
            failed_requests = failed_requests,
            "Error recovery test completed"
        );

        // Verify that valid requests succeeded
        assert!(successful_requests >= 2, "Valid requests should succeed");
        assert!(failed_requests >= 2, "Invalid requests should fail");
    }
}

// Placeholder module for non-integration builds
#[cfg(not(feature = "integration-tests"))]
mod placeholder {
    //! Integration tests are only available with the `integration-tests` feature.
    //!
    //! To run these tests:
    //! ```bash
    //! cargo test --features integration-tests --test error_handling_tests
    //! ```
}
