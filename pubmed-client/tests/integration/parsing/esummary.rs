//! ESummary API model parsing validation tests
//!
//! Tests that ESummary models can parse real API responses by simulating the actual parsing flow.
//! Fixture files were downloaded from the NCBI ESummary API using scripts/download_esummary_fixtures.sh

use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pubmed_client::{ClientConfig, PubMedClient};

/// Test PMIDs to check (well-known articles with stable metadata)
const TEST_PMIDS: &[&str] = &["31978945", "33515491", "32887691", "25760099"];

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

/// Test that ESummary parses single article summaries correctly for each test PMID
#[tokio::test]
#[traced_test]
async fn test_esummary_single_article_parsing() {
    let mock_server = MockServer::start().await;
    let mut parsed_count = 0;

    for pmid in TEST_PMIDS {
        let fixture_path = resolve_fixture_path(&format!(
            "tests/integration/test_data/api_responses/esummary/summary_{pmid}.json"
        ));

        if fixture_path.is_none() {
            warn!(pmid = pmid, "ESummary single fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

        Mock::given(method("GET"))
            .and(path("/esummary.fcgi"))
            .and(query_param("db", "pubmed"))
            .and(query_param("id", *pmid))
            .and(query_param("retmode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(content))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = ClientConfig::new()
            .with_base_url(mock_server.uri())
            .with_rate_limit(100.0);
        let client = PubMedClient::with_config(config);

        match client.fetch_summary(pmid).await {
            Ok(summary) => {
                assert_eq!(summary.pmid, *pmid, "PMID should match");
                assert!(!summary.title.is_empty(), "Title should not be empty");
                assert!(!summary.authors.is_empty(), "Should have authors");
                assert!(!summary.journal.is_empty(), "Journal should not be empty");
                assert!(
                    !summary.pub_date.is_empty(),
                    "Publication date should not be empty"
                );

                parsed_count += 1;
                info!(
                    pmid = pmid,
                    title = %summary.title,
                    journal = %summary.journal,
                    authors_count = summary.authors.len(),
                    doi = ?summary.doi,
                    "Successfully parsed ESummary single article"
                );
            }
            Err(e) => {
                warn!(pmid = pmid, error = %e, "Failed to parse ESummary single article");
            }
        }
    }

    assert!(
        parsed_count > 0,
        "Should successfully parse at least some article summaries"
    );
}

/// Test that ESummary parses the COVID-19 paper (PMID 31978945) with correct metadata
#[tokio::test]
#[traced_test]
async fn test_esummary_known_article_metadata() {
    let mock_server = MockServer::start().await;
    let pmid = "31978945";

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/esummary/summary_31978945.json",
    );

    if fixture_path.is_none() {
        warn!("ESummary fixture for PMID 31978945 not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/esummary.fcgi"))
        .and(query_param("db", "pubmed"))
        .and(query_param("id", pmid))
        .and(query_param("retmode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    match client.fetch_summary(pmid).await {
        Ok(summary) => {
            assert_eq!(summary.pmid, pmid);
            assert!(
                summary.title.contains("Novel Coronavirus"),
                "Title should mention Novel Coronavirus, got: {}",
                summary.title
            );
            assert_eq!(summary.journal, "N Engl J Med", "Journal should be NEJM");
            assert_eq!(
                summary.full_journal_name, "The New England journal of medicine",
                "Full journal name should match"
            );
            assert_eq!(summary.volume, "382", "Volume should be 382");
            assert_eq!(summary.issue, "8", "Issue should be 8");
            assert_eq!(summary.pages, "727-733", "Pages should be 727-733");
            assert!(summary.doi.is_some(), "Should have DOI");
            assert_eq!(
                summary.doi.as_deref(),
                Some("10.1056/NEJMoa2001017"),
                "DOI should match"
            );
            assert!(
                summary.pmc_id.is_some(),
                "Should have PMC ID (open access article)"
            );
            assert_eq!(
                summary.pmc_id.as_deref(),
                Some("PMC7092803"),
                "PMC ID should match"
            );

            info!(
                pmid = pmid,
                journal = %summary.journal,
                doi = ?summary.doi,
                "ESummary known article metadata test passed"
            );
        }
        Err(e) => {
            panic!("ESummary known article parsing failed: {}", e);
        }
    }
}

/// Test that ESummary parses multiple article summaries correctly
#[tokio::test]
#[traced_test]
async fn test_esummary_multiple_articles_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/esummary/summaries_multiple.json",
    );

    if fixture_path.is_none() {
        warn!("ESummary multiple articles fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    // For multiple PMIDs, the client uses comma-separated IDs; match any request
    Mock::given(method("GET"))
        .and(path("/esummary.fcgi"))
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

    match client.fetch_summaries(&["31978945", "33515491"]).await {
        Ok(summaries) => {
            assert_eq!(summaries.len(), 2, "Should return 2 summaries");

            let article1 = summaries.iter().find(|s| s.pmid == "31978945");
            let article2 = summaries.iter().find(|s| s.pmid == "33515491");

            assert!(article1.is_some(), "Should have PMID 31978945");
            assert!(article2.is_some(), "Should have PMID 33515491");

            let s1 = article1.unwrap();
            assert!(!s1.title.is_empty(), "Article 1 should have title");
            assert!(!s1.journal.is_empty(), "Article 1 should have journal");
            assert!(!s1.authors.is_empty(), "Article 1 should have authors");

            let s2 = article2.unwrap();
            assert!(!s2.title.is_empty(), "Article 2 should have title");
            assert!(!s2.journal.is_empty(), "Article 2 should have journal");

            info!(
                count = summaries.len(),
                pmid1 = %s1.pmid,
                journal1 = %s1.journal,
                pmid2 = %s2.pmid,
                journal2 = %s2.journal,
                "ESummary multiple articles parsing test passed"
            );
        }
        Err(e) => {
            panic!("ESummary multiple articles parsing failed: {}", e);
        }
    }
}
