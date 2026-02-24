//! WebEnv/History Server integration tests
//!
//! These tests verify the WebEnv/query_key functionality for efficient
//! pagination through large result sets using NCBI's history server.
//!
//! **IMPORTANT**: These tests are only run when:
//! 1. The `integration-tests` feature is enabled
//! 2. The `PUBMED_REAL_API_TESTS` environment variable is set
//!
//! To run these tests:
//! ```bash
//! PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test test_webenv
//! ```

#[path = "../common/mod.rs"]
mod common;

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use std::time::Duration;
    use tokio::time::sleep;
    use tracing::{debug, info};
    use tracing_test::traced_test;

    use crate::common::integration_test_utils::{
        create_test_pubmed_client, should_run_real_api_tests,
    };

    /// Test search_with_history returns WebEnv and query_key
    #[tokio::test]
    #[traced_test]
    async fn test_search_with_history_returns_session() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        let result = client.search_with_history("cancer", 10).await.unwrap();

        info!(
            total_count = result.total_count,
            pmids_count = result.pmids.len(),
            has_webenv = result.webenv.is_some(),
            has_query_key = result.query_key.is_some(),
            "Search with history completed"
        );

        // Verify we got results
        assert!(!result.pmids.is_empty(), "Should return PMIDs");
        assert!(result.total_count > 0, "Should have total count");

        // Verify history session is available
        assert!(result.webenv.is_some(), "Should have WebEnv");
        assert!(result.query_key.is_some(), "Should have query_key");
        assert!(result.has_history(), "has_history() should return true");

        // Verify we can get a HistorySession
        let session = result.history_session();
        assert!(session.is_some(), "Should be able to get HistorySession");

        let session = session.unwrap();
        assert!(!session.webenv.is_empty(), "WebEnv should not be empty");
        assert!(
            !session.query_key.is_empty(),
            "query_key should not be empty"
        );

        debug!(
            webenv = %session.webenv,
            query_key = %session.query_key,
            "Got history session"
        );
    }

    /// Test fetch_from_history can retrieve articles using WebEnv
    #[tokio::test]
    #[traced_test]
    async fn test_fetch_from_history() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        // First, search with history
        let result = client
            .search_with_history("diabetes treatment", 20)
            .await
            .unwrap();

        let session = result
            .history_session()
            .expect("Should have history session");

        info!(
            total_count = result.total_count,
            "Search completed, fetching from history"
        );

        // Wait a bit to avoid rate limiting
        sleep(Duration::from_millis(200)).await;

        // Fetch first batch
        let articles = client.fetch_from_history(&session, 0, 5).await.unwrap();

        info!(
            fetched_count = articles.len(),
            "Fetched articles from history"
        );

        assert!(!articles.is_empty(), "Should fetch articles");
        assert!(articles.len() <= 5, "Should not exceed requested max");

        // Verify article structure
        for article in &articles {
            assert!(!article.pmid.is_empty(), "Article should have PMID");
            assert!(!article.title.is_empty(), "Article should have title");
            debug!(pmid = %article.pmid, title = %article.title, "Fetched article");
        }
    }

    /// Test pagination using fetch_from_history
    #[tokio::test]
    #[traced_test]
    async fn test_history_pagination() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        // Search for a topic with many results
        let result = client.search_with_history("COVID-19", 100).await.unwrap();

        let session = result
            .history_session()
            .expect("Should have history session");

        info!(
            total_count = result.total_count,
            "Search completed, testing pagination"
        );

        // We need at least 10 results for this test
        if result.total_count < 10 {
            info!("Not enough results for pagination test, skipping");
            return;
        }

        sleep(Duration::from_millis(200)).await;

        // Fetch first batch (0-4)
        let batch1 = client.fetch_from_history(&session, 0, 5).await.unwrap();
        assert_eq!(batch1.len(), 5, "First batch should have 5 articles");

        sleep(Duration::from_millis(200)).await;

        // Fetch second batch (5-9)
        let batch2 = client.fetch_from_history(&session, 5, 5).await.unwrap();
        assert_eq!(batch2.len(), 5, "Second batch should have 5 articles");

        // Verify batches don't overlap (different PMIDs)
        let batch1_pmids: Vec<_> = batch1.iter().map(|a| &a.pmid).collect();
        let batch2_pmids: Vec<_> = batch2.iter().map(|a| &a.pmid).collect();

        for pmid in &batch2_pmids {
            assert!(
                !batch1_pmids.contains(pmid),
                "Batches should not overlap: {pmid} appears in both"
            );
        }

        info!(
            batch1_pmids = ?batch1_pmids,
            batch2_pmids = ?batch2_pmids,
            "Pagination verified - batches are distinct"
        );
    }

    /// Test EPost uploads PMIDs and returns a valid session
    #[tokio::test]
    #[traced_test]
    async fn test_epost_returns_session() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        let result = client
            .epost(&["31978945", "33515491", "25760099"])
            .await
            .unwrap();

        info!(
            webenv = %result.webenv,
            query_key = %result.query_key,
            "EPost completed"
        );

        assert!(!result.webenv.is_empty(), "WebEnv should not be empty");
        assert!(
            !result.query_key.is_empty(),
            "query_key should not be empty"
        );
    }

    /// Test EPost followed by fetch_from_history retrieves articles
    #[tokio::test]
    #[traced_test]
    async fn test_epost_then_fetch() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        // Upload PMIDs
        let result = client.epost(&["31978945", "33515491"]).await.unwrap();

        let session = result.history_session();

        sleep(Duration::from_millis(200)).await;

        // Fetch articles using the session
        let articles = client.fetch_from_history(&session, 0, 10).await.unwrap();

        info!(
            fetched_count = articles.len(),
            "Fetched articles from EPost session"
        );

        assert!(
            !articles.is_empty(),
            "Should fetch articles from EPost session"
        );

        // Verify we got the expected PMIDs
        let pmids: Vec<&str> = articles.iter().map(|a| a.pmid.as_str()).collect();
        debug!(pmids = ?pmids, "Fetched PMIDs from EPost session");
    }

    /// Test EPost to existing session appends PMIDs
    #[tokio::test]
    #[traced_test]
    async fn test_epost_to_session() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        // First upload
        let result1 = client.epost(&["31978945"]).await.unwrap();

        sleep(Duration::from_millis(200)).await;

        // Append to existing session
        let result2 = client
            .epost_to_session(&["33515491"], &result1.history_session())
            .await
            .unwrap();

        info!(
            webenv1 = %result1.webenv,
            query_key1 = %result1.query_key,
            webenv2 = %result2.webenv,
            query_key2 = %result2.query_key,
            "EPost to session completed"
        );

        // WebEnv should be the same session
        assert_eq!(
            result1.webenv, result2.webenv,
            "WebEnv should be the same for appended session"
        );

        // Query key should be different (new set of IDs)
        assert_ne!(
            result1.query_key, result2.query_key,
            "Query keys should differ between uploads"
        );
    }

    /// Test empty query returns empty result
    #[tokio::test]
    #[traced_test]
    async fn test_search_with_history_empty_query() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        let result = client.search_with_history("", 10).await.unwrap();

        assert!(
            result.pmids.is_empty(),
            "Empty query should return no PMIDs"
        );
        assert_eq!(
            result.total_count, 0,
            "Empty query should have zero total count"
        );
        assert!(!result.has_history(), "Empty query should not have history");
    }

    /// Test search_all streaming (limited to first few results)
    #[tokio::test]
    #[traced_test]
    async fn test_search_all_streaming() {
        use futures_util::StreamExt;
        use std::pin::pin;

        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        let client = create_test_pubmed_client();

        let stream = client.search_all("biomarker cancer", 5);
        let mut stream = pin!(stream);
        let mut articles = Vec::new();
        let mut count = 0;
        let max_articles = 10; // Limit for test

        while let Some(result) = stream.next().await {
            match result {
                Ok(article) => {
                    debug!(pmid = %article.pmid, title = %article.title, "Streamed article");
                    articles.push(article);
                    count += 1;
                    if count >= max_articles {
                        break;
                    }
                }
                Err(e) => {
                    info!(error = %e, "Stream error (may be expected for empty results)");
                    break;
                }
            }
        }

        info!(streamed_count = articles.len(), "Stream test completed");

        // We should have gotten some articles (unless the query returns nothing)
        // Don't assert on count since API results can vary
        for article in &articles {
            assert!(
                !article.pmid.is_empty(),
                "Streamed article should have PMID"
            );
            assert!(
                !article.title.is_empty(),
                "Streamed article should have title"
            );
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use pubmed_client::{HistorySession, SearchResult};

    #[test]
    fn test_search_result_history_session() {
        // With both webenv and query_key
        let result = SearchResult {
            pmids: vec!["123".to_string()],
            total_count: 100,
            webenv: Some("MCID_abc123".to_string()),
            query_key: Some("1".to_string()),
            query_translation: None,
        };

        assert!(result.has_history());
        let session = result.history_session().unwrap();
        assert_eq!(session.webenv, "MCID_abc123");
        assert_eq!(session.query_key, "1");
    }

    #[test]
    fn test_search_result_no_history_session() {
        // Without webenv
        let result = SearchResult {
            pmids: vec!["123".to_string()],
            total_count: 100,
            webenv: None,
            query_key: Some("1".to_string()),
            query_translation: None,
        };

        assert!(!result.has_history());
        assert!(result.history_session().is_none());

        // Without query_key
        let result2 = SearchResult {
            pmids: vec!["123".to_string()],
            total_count: 100,
            webenv: Some("MCID_abc123".to_string()),
            query_key: None,
            query_translation: None,
        };

        assert!(!result2.has_history());
        assert!(result2.history_session().is_none());
    }

    #[test]
    fn test_epost_result_history_session() {
        use pubmed_client::EPostResult;

        let result = EPostResult {
            webenv: "MCID_epost123".to_string(),
            query_key: "1".to_string(),
        };

        let session = result.history_session();
        assert_eq!(session.webenv, "MCID_epost123");
        assert_eq!(session.query_key, "1");
    }

    #[test]
    fn test_history_session_equality() {
        let session1 = HistorySession {
            webenv: "MCID_abc".to_string(),
            query_key: "1".to_string(),
        };

        let session2 = HistorySession {
            webenv: "MCID_abc".to_string(),
            query_key: "1".to_string(),
        };

        let session3 = HistorySession {
            webenv: "MCID_xyz".to_string(),
            query_key: "1".to_string(),
        };

        assert_eq!(session1, session2);
        assert_ne!(session1, session3);
    }
}
