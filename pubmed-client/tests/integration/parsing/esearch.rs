//! ESearch API model parsing validation tests
//!
//! Tests that ESearch models can parse real API responses by simulating the actual parsing flow.
//! Fixture files were downloaded from the NCBI ESearch API using scripts/download_esearch_fixtures.sh

use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pubmed_client::{ClientConfig, PubMedClient};

/// Helper to resolve fixture path from either workspace root or crate root
fn resolve_fixture_path(relative: &str) -> Option<String> {
    let workspace = format!("pubmed-client/{}", relative);
    let local = relative.to_string();
    if std::path::Path::new(&workspace).exists() {
        Some(workspace)
    } else if std::path::Path::new(&local).exists() {
        Some(local)
    } else {
        None
    }
}

/// Test that ESearch returns PMIDs correctly from a basic search response
#[tokio::test]
#[traced_test]
async fn test_esearch_basic_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/esearch/search_covid19.json",
    );

    if fixture_path.is_none() {
        warn!("ESearch basic fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/esearch.fcgi"))
        .and(query_param("db", "pubmed"))
        .and(query_param("retmode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    match client.search_articles("covid-19", 5, None).await {
        Ok(pmids) => {
            assert!(!pmids.is_empty(), "Should return PMIDs");
            assert_eq!(pmids.len(), 5, "Should return exactly 5 PMIDs (retmax=5)");
            // Verify PMIDs are numeric strings
            for pmid in &pmids {
                assert!(
                    pmid.parse::<u64>().is_ok(),
                    "PMID '{}' should be a numeric string",
                    pmid
                );
            }
            info!(
                pmids_count = pmids.len(),
                first_pmid = %pmids[0],
                "ESearch basic parsing test passed"
            );
        }
        Err(e) => {
            panic!("ESearch parsing failed: {}", e);
        }
    }
}

/// Test that ESearch with history server returns WebEnv and query_key
#[tokio::test]
#[traced_test]
async fn test_esearch_with_history_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/esearch/search_with_history.json",
    );

    if fixture_path.is_none() {
        warn!("ESearch with history fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/esearch.fcgi"))
        .and(query_param("db", "pubmed"))
        .and(query_param("usehistory", "y"))
        .and(query_param("retmode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    match client.search_with_history("asthma", 10).await {
        Ok(result) => {
            assert!(result.total_count > 0, "Should have results for 'asthma'");
            assert_eq!(result.pmids.len(), 10, "Should return 10 PMIDs (retmax=10)");
            assert!(
                result.webenv.is_some(),
                "Should have WebEnv for history server"
            );
            assert!(
                result.query_key.is_some(),
                "Should have query_key for history server"
            );
            assert!(
                result.query_translation.is_some(),
                "Should have query translation"
            );

            let webenv = result.webenv.as_ref().unwrap();
            assert!(
                webenv.starts_with("MCID_"),
                "WebEnv should start with 'MCID_', got: {}",
                webenv
            );

            let session = result.history_session();
            assert!(session.is_some(), "Should have history session");
            let session = session.unwrap();
            assert_eq!(session.webenv, *webenv, "Session webenv should match");
            assert_eq!(
                session.query_key,
                result.query_key.as_ref().unwrap().as_str(),
                "Session query_key should match"
            );

            info!(
                total = result.total_count,
                pmids_returned = result.pmids.len(),
                webenv = %webenv,
                "ESearch with history parsing test passed"
            );
        }
        Err(e) => {
            panic!("ESearch with history parsing failed: {}", e);
        }
    }
}

/// Test that ESearch handles truly empty results correctly
#[tokio::test]
#[traced_test]
async fn test_esearch_empty_results_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path =
        resolve_fixture_path("tests/integration/test_data/api_responses/esearch/search_empty.json");

    if fixture_path.is_none() {
        warn!("ESearch empty results fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/esearch.fcgi"))
        .and(query_param("db", "pubmed"))
        .and(query_param("retmode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    match client
        .search_articles("zxqwvjkplmhftyrbsound12345678unique", 5, None)
        .await
    {
        Ok(pmids) => {
            assert!(pmids.is_empty(), "Should return empty list for no results");
            info!("ESearch empty results parsing test passed");
        }
        Err(e) => {
            panic!("ESearch empty results parsing failed: {}", e);
        }
    }
}
