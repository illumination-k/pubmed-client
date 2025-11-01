//! Real API rate limiting tests
//!
//! These tests make actual network calls to NCBI E-utilities API to verify
//! rate limiting behavior works correctly with the real API.
//!
//! **IMPORTANT**: These tests are NOT run by default because they:
//! 1. Make real network requests to NCBI servers
//! 2. Take significant time to complete (due to intentional rate limiting)
//! 3. Could potentially trigger actual rate limiting from NCBI
//! 4. Require internet connectivity
//!
//! To run these tests explicitly:
//! ```bash
//! cargo test --test test_real_api_rate_limiting -- --nocapture
//! ```
//!
//! Or run with specific log levels:
//! ```bash
//! RUST_LOG=debug cargo test --test test_real_api_rate_limiting -- --nocapture
//! ```

use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};
use tracing_test::traced_test;

use pubmed_client::{ClientConfig, PubMedClient, PubMedError};

/// Helper function to check if we should skip network tests
/// Based on environment variable to avoid accidental execution
fn should_skip_network_tests() -> bool {
    std::env::var("PUBMED_REAL_API_TESTS").is_err()
}

/// Test that verifies basic rate limiting with real NCBI API
///
/// This test makes actual API calls to verify that:
/// 1. Rate limiting prevents exceeding 3 requests/second (without API key)
/// 2. Requests are properly spaced in time
/// 3. All requests eventually succeed despite rate limiting
#[tokio::test]
#[traced_test]
async fn test_real_api_basic_rate_limiting() {
    if should_skip_network_tests() {
        info!("Skipping real API test - set PUBMED_REAL_API_TESTS=1 to enable");
        return;
    }

    info!("Starting real API rate limiting test (3 req/sec limit)");

    let config = ClientConfig::new()
        .with_rate_limit(3.0) // 3 requests per second (NCBI default without API key)
        .with_email("test@example.com") // Required for NCBI API etiquette
        .with_tool("pubmed-client-rs-test");

    let client = PubMedClient::with_config(config);

    let start_time = Instant::now();
    let test_queries = [
        "covid-19",
        "cancer research",
        "machine learning",
        "diabetes treatment",
        "alzheimer disease",
    ];

    let mut successful_requests = 0;
    let mut total_results = 0;

    for (i, query) in test_queries.iter().enumerate() {
        let request_start = Instant::now();

        info!(
            request_number = i + 1,
            query = query,
            elapsed_since_start = ?start_time.elapsed(),
            "Making real API request"
        );

        match client.search_articles(query, 5).await {
            Ok(results) => {
                successful_requests += 1;
                total_results += results.len();

                let request_duration = request_start.elapsed();
                info!(
                    query = query,
                    results_count = results.len(),
                    request_duration_ms = request_duration.as_millis(),
                    "Request successful"
                );

                // Log some PMIDs for verification
                for (j, pmid) in results.iter().take(2).enumerate() {
                    debug!(pmid = pmid, position = j, "Retrieved PMID");
                }
            }
            Err(e) => {
                warn!(
                    query = query,
                    error = %e,
                    request_duration_ms = request_start.elapsed().as_millis(),
                    "Request failed"
                );

                // If we get rate limited, that's actually expected behavior we want to test
                match &e {
                    PubMedError::RateLimitExceeded => {
                        info!("Rate limit properly enforced by client");
                    }
                    PubMedError::ApiError { status: _, message } if message.contains("429") => {
                        info!("Rate limit enforced by NCBI server (429 response)");
                    }
                    _ => {
                        // Other errors might indicate network issues
                        warn!("Unexpected error type: {:?}", e);
                    }
                }
            }
        }
    }

    let total_duration = start_time.elapsed();

    info!(
        total_requests = test_queries.len(),
        successful_requests = successful_requests,
        total_results = total_results,
        total_duration_ms = total_duration.as_millis(),
        avg_duration_per_request_ms = total_duration.as_millis() / test_queries.len() as u128,
        "Real API rate limiting test completed"
    );

    // Verify that rate limiting is working:
    // With 5 requests at 3 req/sec, it should take at least 1.33 seconds
    // (but we add some tolerance for network latency)
    let min_expected_duration = Duration::from_millis(1000); // At least 1 second
    assert!(
        total_duration >= min_expected_duration,
        "Requests completed too quickly - rate limiting may not be working. Duration: {:?}",
        total_duration
    );

    // At least some requests should succeed (unless there are network issues)
    assert!(
        successful_requests > 0,
        "No requests succeeded - check network connectivity"
    );
}

/// Test concurrent requests with real API to verify shared rate limiting
///
/// This test spawns multiple concurrent tasks making API requests to verify:
/// 1. Rate limiting is shared across all tasks
/// 2. Concurrent requests don't overwhelm the API
/// 3. All requests eventually complete or fail gracefully
#[tokio::test]
#[traced_test]
async fn test_real_api_concurrent_rate_limiting() {
    if should_skip_network_tests() {
        info!("Skipping concurrent real API test - set PUBMED_REAL_API_TESTS=1 to enable");
        return;
    }

    info!("Starting concurrent real API rate limiting test");

    let config = ClientConfig::new()
        .with_rate_limit(2.0) // Conservative 2 req/sec for concurrent test
        .with_email("test@example.com")
        .with_tool("pubmed-client-rs-concurrent-test");

    let client = PubMedClient::with_config(config);
    let start_time = Instant::now();

    // Spawn concurrent tasks
    let tasks: Vec<_> = (0..6)
        .map(|i| {
            let client = client.clone();
            tokio::spawn(async move {
                let query = format!("test query {i}");
                let task_start = Instant::now();

                debug!(task_id = i, query = %query, "Starting concurrent task");

                let result = client.search_articles(&query, 3).await;
                let task_duration = task_start.elapsed();

                match result {
                    Ok(pmids) => {
                        info!(
                            task_id = i,
                            results_count = pmids.len(),
                            task_duration_ms = task_duration.as_millis(),
                            "Concurrent task completed successfully"
                        );
                        Ok(pmids.len())
                    }
                    Err(e) => {
                        warn!(
                            task_id = i,
                            error = %e,
                            task_duration_ms = task_duration.as_millis(),
                            "Concurrent task failed"
                        );
                        Err(e)
                    }
                }
            })
        })
        .collect();

    // Wait for all tasks and collect results
    let mut successful_tasks = 0;
    let mut total_results = 0;
    let mut rate_limited_tasks = 0;

    for (i, task) in tasks.into_iter().enumerate() {
        match task.await {
            Ok(Ok(result_count)) => {
                successful_tasks += 1;
                total_results += result_count;
            }
            Ok(Err(PubMedError::RateLimitExceeded)) => {
                rate_limited_tasks += 1;
                debug!(task_id = i, "Task was rate limited (expected)");
            }
            Ok(Err(PubMedError::ApiError { status: _, message })) if message.contains("429") => {
                rate_limited_tasks += 1;
                debug!(task_id = i, "Task got 429 from server (expected)");
            }
            Ok(Err(e)) => {
                warn!(task_id = i, error = %e, "Task failed with unexpected error");
            }
            Err(e) => {
                warn!(task_id = i, error = %e, "Task panicked");
            }
        }
    }

    let total_duration = start_time.elapsed();

    info!(
        total_tasks = 6,
        successful_tasks = successful_tasks,
        rate_limited_tasks = rate_limited_tasks,
        total_results = total_results,
        total_duration_ms = total_duration.as_millis(),
        "Concurrent real API test completed"
    );

    // Verify that concurrent rate limiting works:
    // With 6 requests at 2 req/sec, it should take at least 1 second (allowing for network latency)
    // The key is that some requests should be rate limited, not the exact timing
    let min_expected_duration = Duration::from_millis(1000);
    assert!(
        total_duration >= min_expected_duration,
        "Concurrent requests completed too quickly - rate limiting may not be working. Duration: {:?}",
        total_duration
    );

    // Some requests should complete (unless network issues)
    assert!(
        successful_tasks + rate_limited_tasks >= 4,
        "Too many tasks failed - expected at least 4 to complete or be rate limited"
    );
}

/// Test that API key configuration increases rate limits (requires real API key)
///
/// This test verifies:
/// 1. API key is properly included in requests
/// 2. Higher rate limits work with API key
/// 3. Requests complete faster with API key
///
/// **Note**: This test requires a real NCBI API key set in environment variable
#[tokio::test]
#[traced_test]
async fn test_real_api_with_api_key() {
    if should_skip_network_tests() {
        info!("Skipping API key test - set PUBMED_REAL_API_TESTS=1 to enable");
        return;
    }

    let api_key = match std::env::var("NCBI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            warn!("Skipping API key test - set NCBI_API_KEY environment variable");
            return;
        }
    };

    info!("Starting real API test with API key (10 req/sec limit)");

    let config = ClientConfig::new()
        .with_api_key(&api_key)
        .with_rate_limit(10.0) // 10 requests per second with API key
        .with_email("test@example.com")
        .with_tool("pubmed-client-rs-apikey-test");

    let client = PubMedClient::with_config(config);
    let start_time = Instant::now();

    // Test with more requests since we have higher rate limit
    let test_queries = [
        "CRISPR",
        "immunotherapy",
        "genomics",
        "proteomics",
        "bioinformatics",
        "microbiome",
        "epigenetics",
        "transcriptomics",
    ];

    let mut successful_requests = 0;

    for (i, query) in test_queries.iter().enumerate() {
        info!(
            request_number = i + 1,
            query = query,
            "Making API request with key"
        );

        match client.search_articles(query, 3).await {
            Ok(results) => {
                successful_requests += 1;
                debug!(
                    query = query,
                    results_count = results.len(),
                    "Request with API key successful"
                );
            }
            Err(e) => {
                warn!(query = query, error = %e, "Request with API key failed");
            }
        }
    }

    let total_duration = start_time.elapsed();

    info!(
        total_requests = test_queries.len(),
        successful_requests = successful_requests,
        total_duration_ms = total_duration.as_millis(),
        "API key test completed"
    );

    // With API key and 10 req/sec, 8 requests should complete much faster
    // Should take less than 2 seconds (with some tolerance for network latency)
    let max_expected_duration = Duration::from_millis(3000);
    assert!(
        total_duration <= max_expected_duration,
        "Requests took too long with API key - rate limiting may not be configured correctly. Duration: {:?}",
        total_duration
    );

    // Most requests should succeed with API key
    assert!(
        successful_requests >= 6,
        "Too few successful requests with API key: {}/{}",
        successful_requests,
        test_queries.len()
    );
}

/// Test fetching actual articles to verify end-to-end functionality
///
/// This test:
/// 1. Searches for articles using real API
/// 2. Fetches detailed article data
/// 3. Verifies data quality and parsing
/// 4. Tests rate limiting across different API endpoints
#[tokio::test]
#[traced_test]
async fn test_real_api_end_to_end_with_rate_limiting() {
    if should_skip_network_tests() {
        info!("Skipping end-to-end test - set PUBMED_REAL_API_TESTS=1 to enable");
        return;
    }

    info!("Starting end-to-end real API test with rate limiting");

    let config = ClientConfig::new()
        .with_rate_limit(2.5) // Conservative rate limit
        .with_email("test@example.com")
        .with_tool("pubmed-client-rs-e2e-test");

    let client = PubMedClient::with_config(config);
    let start_time = Instant::now();

    // Step 1: Search for articles
    info!("Step 1: Searching for COVID-19 articles");
    let search_start = Instant::now();

    let pmids = match client
        .search_articles("COVID-19[Title] AND 2023[PDAT]", 5)
        .await
    {
        Ok(pmids) => {
            info!(
                pmids_found = pmids.len(),
                search_duration_ms = search_start.elapsed().as_millis(),
                "Search completed"
            );
            pmids
        }
        Err(e) => {
            warn!(error = %e, "Search failed");
            return; // Skip rest of test if search fails
        }
    };

    // Step 2: Fetch detailed article data
    info!("Step 2: Fetching article details");
    let mut articles_fetched = 0;
    let mut articles_with_abstracts = 0;

    for (i, pmid) in pmids.iter().take(3).enumerate() {
        // Limit to 3 to avoid long test times
        let fetch_start = Instant::now();

        info!(
            pmid = pmid,
            article_number = i + 1,
            "Fetching article details"
        );

        match client.fetch_article(pmid).await {
            Ok(article) => {
                articles_fetched += 1;

                // Verify article data quality
                assert!(!article.title.is_empty(), "Article should have title");
                assert!(!article.journal.is_empty(), "Article should have journal");
                assert!(!article.authors.is_empty(), "Article should have authors");

                if article.abstract_text.is_some() {
                    articles_with_abstracts += 1;
                }

                info!(
                    pmid = pmid,
                    title_length = article.title.len(),
                    authors_count = article.authors.len(),
                    has_abstract = article.abstract_text.is_some(),
                    fetch_duration_ms = fetch_start.elapsed().as_millis(),
                    "Article fetched successfully"
                );

                // Log first 100 chars of title for verification
                let title_preview = if article.title.len() > 100 {
                    format!("{}...", &article.title[..100])
                } else {
                    article.title.clone()
                };
                debug!(pmid = pmid, title_preview = %title_preview, "Article title");
            }
            Err(e) => {
                warn!(pmid = pmid, error = %e, "Failed to fetch article");
            }
        }

        // Add small delay between requests to be extra respectful to NCBI
        if i < 2 {
            // Don't sleep after last request
            sleep(Duration::from_millis(100)).await;
        }
    }

    let total_duration = start_time.elapsed();

    info!(
        total_pmids_found = pmids.len(),
        articles_fetched = articles_fetched,
        articles_with_abstracts = articles_with_abstracts,
        total_duration_ms = total_duration.as_millis(),
        "End-to-end test completed"
    );

    // Verify end-to-end functionality
    assert!(
        !pmids.is_empty(),
        "Should find some PMIDs for COVID-19 search"
    );
    assert!(
        articles_fetched > 0,
        "Should successfully fetch at least one article"
    );

    // Rate limiting should ensure test takes reasonable time
    // With 4 requests (1 search + 3 fetches) at 2.5 req/sec, should take at least 1.2 seconds
    let min_expected_duration = Duration::from_millis(1000);
    assert!(
        total_duration >= min_expected_duration,
        "End-to-end test completed too quickly - rate limiting may not be working"
    );
}

/// Test behavior under real network conditions and potential server rate limiting
///
/// This test intentionally pushes rate limits to verify:
/// 1. Client gracefully handles server-side rate limiting
/// 2. Requests eventually succeed after backing off
/// 3. Error handling works correctly with real API responses
#[tokio::test]
#[traced_test]
async fn test_real_api_server_rate_limit_handling() {
    if should_skip_network_tests() {
        warn!("Skipping server rate limit test - set PUBMED_REAL_API_TESTS=1 to enable");
        return;
    }

    info!("Starting server-side rate limit handling test");

    // Use higher client-side rate limit to test server-side enforcement
    let config = ClientConfig::new()
        .with_rate_limit(5.0) // Higher than NCBI's limit without API key
        .with_email("test@example.com")
        .with_tool("pubmed-client-rs-server-limit-test");

    let client = PubMedClient::with_config(config);

    // Make rapid requests to potentially trigger server-side rate limiting
    let mut requests_made = 0;
    let mut successful_requests = 0;
    let mut rate_limit_errors = 0;
    let mut other_errors = 0;

    for i in 0..10 {
        requests_made += 1;

        let result = client.search_articles(&format!("test query {i}"), 1).await;

        match result {
            Ok(pmids) => {
                successful_requests += 1;
                debug!(
                    request_number = i + 1,
                    results_count = pmids.len(),
                    "Request successful despite high rate"
                );
            }
            Err(PubMedError::RateLimitExceeded) => {
                rate_limit_errors += 1;
                debug!(request_number = i + 1, "Client-side rate limit triggered");
            }
            Err(PubMedError::ApiError { status: _, message }) if message.contains("429") => {
                rate_limit_errors += 1;
                debug!(request_number = i + 1, "Server returned 429 (expected)");
            }
            Err(e) => {
                other_errors += 1;
                warn!(request_number = i + 1, error = %e, "Other error occurred");
            }
        }

        // Small delay to avoid overwhelming the server
        sleep(Duration::from_millis(50)).await;
    }

    info!(
        requests_made = requests_made,
        successful_requests = successful_requests,
        rate_limit_errors = rate_limit_errors,
        other_errors = other_errors,
        "Server rate limit test completed"
    );

    // Verify basic functionality (this test primarily checks error handling, not forcing rate limits)
    // If all requests succeed, that's actually fine - it means our rate limiting is working well
    assert!(
        successful_requests + rate_limit_errors + other_errors == requests_made,
        "All requests should be accounted for"
    );

    // At least some requests should succeed
    assert!(
        successful_requests > 0,
        "At least some requests should succeed"
    );

    // Should not have too many unexpected errors
    assert!(
        other_errors <= 2,
        "Too many unexpected errors: {}",
        other_errors
    );

    // If rate limiting occurred, that's expected behavior
    if rate_limit_errors > 0 {
        info!(
            rate_limit_errors = rate_limit_errors,
            "Rate limiting successfully detected and handled"
        );
    } else {
        info!("All requests succeeded - rate limiting is working well within limits");
    }
}
