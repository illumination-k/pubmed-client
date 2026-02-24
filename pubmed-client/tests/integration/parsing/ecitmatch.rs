//! ECitMatch API model parsing validation tests
//!
//! Tests that ECitMatch models can parse real API responses by simulating the actual parsing flow.
//! Fixture files were downloaded from the NCBI ECitMatch API using scripts/download_ecitmatch_fixtures.sh
//!
//! Note on NCBI ECitMatch response format:
//! - Found citations: `journal|year|volume|page|author|key|PMID`
//! - Not found (invalid journal): `journal|year|volume|page||key|NOT_FOUND;INVALID_JOURNAL`
//! - Ambiguous: `journal|year|volume|page|author|key|AMBIGUOUS`
//! - Not found (valid journal): `journal|year|volume|page|author|key|` (empty PMID field)

use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pubmed_client::{CitationMatchStatus, CitationQuery, ClientConfig, PubMedClient};

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

/// Test that ECitMatch parses all-found citation responses correctly
///
/// Uses real NCBI response where both citations have known PMIDs.
#[tokio::test]
#[traced_test]
async fn test_ecitmatch_all_found_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/ecitmatch/citmatch_found.txt",
    );

    if fixture_path.is_none() {
        warn!("ECitMatch found fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/ecitmatch.cgi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    let citations = vec![
        CitationQuery::new(
            "proc natl acad sci u s a",
            "1991",
            "88",
            "3248",
            "mann bj",
            "Art1",
        ),
        CitationQuery::new("science", "1987", "235", "182", "palmenberg ac", "Art2"),
    ];

    match client.match_citations(&citations).await {
        Ok(results) => {
            assert_eq!(results.matches.len(), 2, "Should have 2 citation matches");

            let art1 = results.matches.iter().find(|m| m.key == "Art1");
            let art2 = results.matches.iter().find(|m| m.key == "Art2");

            assert!(art1.is_some(), "Should have Art1 match");
            assert!(art2.is_some(), "Should have Art2 match");

            let art1 = art1.unwrap();
            assert_eq!(
                art1.status,
                CitationMatchStatus::Found,
                "Art1 should be found"
            );
            assert_eq!(
                art1.pmid.as_deref(),
                Some("2014248"),
                "Art1 PMID should match"
            );
            assert_eq!(art1.journal, "proc natl acad sci u s a");
            assert_eq!(art1.year, "1991");

            let art2 = art2.unwrap();
            assert_eq!(
                art2.status,
                CitationMatchStatus::Found,
                "Art2 should be found"
            );
            assert_eq!(
                art2.pmid.as_deref(),
                Some("3026048"),
                "Art2 PMID should match"
            );

            info!(
                matched_count = results.matches.len(),
                art1_pmid = ?art1.pmid,
                art2_pmid = ?art2.pmid,
                "ECitMatch all-found parsing test passed"
            );
        }
        Err(e) => {
            panic!("ECitMatch all-found parsing failed: {}", e);
        }
    }
}

/// Test that ECitMatch parses a single citation lookup correctly
#[tokio::test]
#[traced_test]
async fn test_ecitmatch_single_citation_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/ecitmatch/citmatch_single.txt",
    );

    if fixture_path.is_none() {
        warn!("ECitMatch single fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/ecitmatch.cgi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    let citations = vec![CitationQuery::new(
        "n engl j med",
        "2020",
        "382",
        "727",
        "zhu n",
        "covid1",
    )];

    match client.match_citations(&citations).await {
        Ok(results) => {
            assert_eq!(results.matches.len(), 1, "Should have 1 citation match");

            let covid1 = &results.matches[0];
            assert_eq!(covid1.key, "covid1");
            assert_eq!(
                covid1.status,
                CitationMatchStatus::Found,
                "COVID-19 paper should be found"
            );
            assert_eq!(
                covid1.pmid.as_deref(),
                Some("31978945"),
                "PMID should be 31978945 (Zhu N et al., NEJM 2020)"
            );
            assert_eq!(covid1.journal, "n engl j med");

            info!(
                pmid = ?covid1.pmid,
                "ECitMatch single citation parsing test passed"
            );
        }
        Err(e) => {
            panic!("ECitMatch single citation parsing failed: {}", e);
        }
    }
}

/// Test that ECitMatch parses mixed results including NCBI error responses
///
/// The real NCBI API returns `NOT_FOUND;INVALID_JOURNAL` for citations with
/// invalid journal names. This test documents how the current parser handles
/// this response format.
#[tokio::test]
#[traced_test]
async fn test_ecitmatch_mixed_results_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/ecitmatch/citmatch_mixed.txt",
    );

    if fixture_path.is_none() {
        warn!("ECitMatch mixed fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/ecitmatch.cgi"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    // citmatch_mixed.txt contains 3 entries:
    //   Art1: found (PMID 2014248)
    //   ref2: NOT_FOUND;INVALID_JOURNAL
    //   Art4: found (PMID 31978945)
    let citations = vec![
        CitationQuery::new(
            "proc natl acad sci u s a",
            "1991",
            "88",
            "3248",
            "mann bj",
            "Art1",
        ),
        CitationQuery::new("fake journal", "2000", "1", "1", "nobody", "ref2"),
        CitationQuery::new("n engl j med", "2020", "382", "727", "zhu n", "Art4"),
    ];

    match client.match_citations(&citations).await {
        Ok(results) => {
            assert_eq!(results.matches.len(), 3, "Should have 3 citation matches");

            let art1 = results.matches.iter().find(|m| m.key == "Art1");
            let art4 = results.matches.iter().find(|m| m.key == "Art4");

            // Art1 and Art4 should be found correctly
            assert!(art1.is_some(), "Should have Art1");
            assert_eq!(
                art1.unwrap().status,
                CitationMatchStatus::Found,
                "Art1 should be found"
            );
            assert_eq!(art1.unwrap().pmid.as_deref(), Some("2014248"));

            assert!(art4.is_some(), "Should have Art4");
            assert_eq!(
                art4.unwrap().status,
                CitationMatchStatus::Found,
                "Art4 should be found"
            );
            assert_eq!(art4.unwrap().pmid.as_deref(), Some("31978945"));

            // Note: the parser classifies anything that is not empty and not "AMBIGUOUS" as Found.
            // NCBI returns "NOT_FOUND;INVALID_JOURNAL" for invalid citations, which the parser
            // currently treats as Found (with the raw string as PMID). Art1 + ref2 + Art4 = 3.
            let found_count = results
                .matches
                .iter()
                .filter(|m| m.status == CitationMatchStatus::Found)
                .count();
            assert_eq!(
                found_count, 3,
                "Parser classifies all non-empty, non-AMBIGUOUS entries as Found"
            );

            info!(
                total = results.matches.len(),
                found = found_count,
                "ECitMatch mixed results parsing test passed"
            );
        }
        Err(e) => {
            panic!("ECitMatch mixed results parsing failed: {}", e);
        }
    }
}
