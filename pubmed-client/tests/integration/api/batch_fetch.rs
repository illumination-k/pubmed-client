//! Real API integration tests for batch fetch_articles
//!
//! These tests make actual network calls to NCBI E-utilities API.
//!
//! **IMPORTANT**: These tests are only run when:
//! 1. The `integration-tests` feature is enabled
//! 2. The `PUBMED_REAL_API_TESTS` environment variable is set
//!
//! To run:
//! ```bash
//! PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test test_batch_fetch_api
//! ```

#[path = "../common/mod.rs"]
mod common;

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use std::collections::HashSet;
    use std::time::Instant;

    use tracing::{debug, info};
    use tracing_test::traced_test;

    use crate::common::integration_test_utils::{
        TEST_PMIDS_STR, create_test_pubmed_client, should_run_real_api_tests,
    };

    /// Test batch fetch with known PMIDs returns correct articles
    #[tokio::test]
    #[traced_test]
    async fn test_batch_fetch_known_articles() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let pmids = &TEST_PMIDS_STR[..3]; // First 3 known PMIDs

        info!(?pmids, "Testing batch fetch with known PMIDs");

        let start = Instant::now();
        let articles = client
            .fetch_articles(pmids)
            .await
            .expect("Batch fetch of known PMIDs should succeed");
        let duration = start.elapsed();

        info!(
            requested = pmids.len(),
            returned = articles.len(),
            duration_ms = duration.as_millis(),
            "Batch fetch completed"
        );

        assert_eq!(articles.len(), 3, "Should return all 3 articles");

        // Verify all requested PMIDs are in the result
        let returned_pmids: HashSet<&str> = articles.iter().map(|a| a.pmid.as_str()).collect();
        for pmid in pmids {
            assert!(
                returned_pmids.contains(*pmid),
                "Result should contain PMID {pmid}"
            );
        }

        // Verify article quality
        for article in &articles {
            assert!(!article.title.is_empty(), "Article should have title");
            assert!(!article.journal.is_empty(), "Article should have journal");
            assert!(!article.authors.is_empty(), "Article should have authors");
            assert!(!article.pub_date.is_empty(), "Article should have pub date");

            debug!(
                pmid = %article.pmid,
                title = %article.title,
                authors = article.authors.len(),
                "Article verified"
            );
        }
    }

    /// Test batch fetch is faster than sequential fetch
    #[tokio::test]
    #[traced_test]
    async fn test_batch_fetch_faster_than_sequential() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let pmids = &TEST_PMIDS_STR[..3];

        // Measure batch fetch time
        let batch_start = Instant::now();
        let batch_articles = client
            .fetch_articles(pmids)
            .await
            .expect("Batch fetch should succeed");
        let batch_duration = batch_start.elapsed();

        assert_eq!(batch_articles.len(), 3);

        // Measure sequential fetch time
        let seq_start = Instant::now();
        for pmid in pmids {
            client
                .fetch_article(pmid)
                .await
                .expect("Sequential fetch should succeed");
        }
        let seq_duration = seq_start.elapsed();

        info!(
            batch_ms = batch_duration.as_millis(),
            sequential_ms = seq_duration.as_millis(),
            speedup = format!(
                "{:.1}x",
                seq_duration.as_secs_f64() / batch_duration.as_secs_f64()
            ),
            "Batch vs sequential performance"
        );

        // Batch should be faster (1 request vs 3 requests + rate limiting delays)
        assert!(
            batch_duration < seq_duration,
            "Batch fetch ({batch_duration:?}) should be faster than sequential ({seq_duration:?})"
        );
    }

    /// Test batch fetch with all 5 known test PMIDs
    #[tokio::test]
    #[traced_test]
    async fn test_batch_fetch_all_test_pmids() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        info!(
            count = TEST_PMIDS_STR.len(),
            "Fetching all test PMIDs in batch"
        );

        let articles = client
            .fetch_articles(TEST_PMIDS_STR)
            .await
            .expect("Batch fetch of all test PMIDs should succeed");

        assert_eq!(
            articles.len(),
            TEST_PMIDS_STR.len(),
            "Should return all {} articles",
            TEST_PMIDS_STR.len()
        );

        // Verify all articles have basic required fields
        for article in &articles {
            assert!(!article.pmid.is_empty());
            assert!(!article.title.is_empty());
            assert!(!article.journal.is_empty());
        }

        // Verify specific known articles
        let covid = articles.iter().find(|a| a.pmid == "31978945");
        assert!(covid.is_some(), "Should contain COVID-19 article");
        assert!(covid.unwrap().abstract_text.is_some());

        let crispr = articles.iter().find(|a| a.pmid == "25760099");
        assert!(crispr.is_some(), "Should contain CRISPR article");
    }

    /// Test batch fetch with single PMID
    #[tokio::test]
    #[traced_test]
    async fn test_batch_fetch_single_pmid() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        let articles = client
            .fetch_articles(&["31978945"])
            .await
            .expect("Single PMID batch should succeed");

        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].pmid, "31978945");
        assert!(!articles[0].title.is_empty());
    }

    /// Test batch fetch results match individual fetch results
    #[tokio::test]
    #[traced_test]
    async fn test_batch_fetch_matches_individual() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let pmids = &["31978945", "25760099"];

        // Fetch individually
        let individual_1 = client
            .fetch_article("31978945")
            .await
            .expect("Individual fetch should succeed");
        let individual_2 = client
            .fetch_article("25760099")
            .await
            .expect("Individual fetch should succeed");

        // Fetch as batch
        let batch = client
            .fetch_articles(pmids)
            .await
            .expect("Batch fetch should succeed");

        // Compare results
        let batch_1 = batch.iter().find(|a| a.pmid == "31978945").unwrap();
        let batch_2 = batch.iter().find(|a| a.pmid == "25760099").unwrap();

        assert_eq!(batch_1.title, individual_1.title);
        assert_eq!(batch_1.journal, individual_1.journal);
        assert_eq!(batch_1.authors.len(), individual_1.authors.len());
        assert_eq!(batch_1.abstract_text, individual_1.abstract_text);

        assert_eq!(batch_2.title, individual_2.title);
        assert_eq!(batch_2.journal, individual_2.journal);
        assert_eq!(batch_2.authors.len(), individual_2.authors.len());

        info!("Batch results match individual fetch results");
    }

    /// Test search_and_fetch uses batch internally (end-to-end)
    #[tokio::test]
    #[traced_test]
    async fn test_search_and_fetch_batch_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        let start = Instant::now();
        let articles = client
            .search_and_fetch("COVID-19[Title] AND 2023[PDAT]", 5, None)
            .await
            .expect("search_and_fetch should succeed");
        let duration = start.elapsed();

        info!(
            results = articles.len(),
            duration_ms = duration.as_millis(),
            "search_and_fetch with batch completed"
        );

        assert!(!articles.is_empty(), "Should find articles");
        assert!(articles.len() <= 5, "Should respect limit");

        for article in &articles {
            assert!(!article.title.is_empty());
            assert!(!article.pmid.is_empty());
        }
    }
}

#[cfg(not(feature = "integration-tests"))]
mod placeholder {
    //! Integration tests are only available with the `integration-tests` feature.
    //!
    //! To run:
    //! ```bash
    //! PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test test_batch_fetch_api
    //! ```
}
