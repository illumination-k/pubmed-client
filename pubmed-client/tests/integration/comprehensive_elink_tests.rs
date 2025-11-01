//! ELink API model parsing validation tests
//!
//! Tests that ELink models can parse real API responses by simulating the actual parsing flow.

use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pubmed_client::{ClientConfig, PubMedClient};

/// Test PMIDs to check
const TEST_PMIDS: &[u32] = &[31978945, 33515491, 32887691, 25760099];

/// Test that ELink models can parse real related articles API responses
#[tokio::test]
#[traced_test]
async fn test_related_articles_model_parsing() {
    let mock_server = MockServer::start().await;
    let mut parsed_count = 0;

    for pmid in TEST_PMIDS {
        // Try both relative paths depending on where the test is run from
        let fixture_path_workspace = format!(
            "pubmed-client/tests/integration/test_data/api_responses/elink/related_{pmid}.json"
        );
        let fixture_path_local =
            format!("tests/integration/test_data/api_responses/elink/related_{pmid}.json");

        let fixture_path = if std::path::Path::new(&fixture_path_workspace).exists() {
            fixture_path_workspace
        } else if std::path::Path::new(&fixture_path_local).exists() {
            fixture_path_local
        } else {
            format!("test_data/api_responses/elink/related_{pmid}.json") // fallback
        };

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(pmid = pmid, "Related articles fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Mock the ELink API response with real fixture data
        Mock::given(method("GET"))
            .and(path("/elink.fcgi"))
            .and(query_param("dbfrom", "pubmed"))
            .and(query_param("db", "pubmed"))
            .and(query_param("id", pmid.to_string()))
            .and(query_param("linkname", "pubmed_pubmed"))
            .and(query_param("retmode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(content))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Create client pointing to mock server
        let config = ClientConfig::new().with_base_url(mock_server.uri());
        let client = PubMedClient::with_config(config);

        // Test that the actual client method can parse the fixture
        match client.get_related_articles(&[*pmid]).await {
            Ok(related_articles) => {
                // Validate the parsed model
                assert!(
                    related_articles.source_pmids.contains(pmid),
                    "Should contain source PMID"
                );
                assert_eq!(
                    related_articles.link_type, "pubmed_pubmed",
                    "Should have correct link type"
                );

                parsed_count += 1;
                info!(
                    pmid = pmid,
                    related_count = related_articles.related_pmids.len(),
                    "Successfully parsed related articles with actual model"
                );
            }
            Err(e) => {
                warn!(
                    pmid = pmid,
                    error = %e,
                    "Failed to parse related articles with actual model"
                );
            }
        }
    }

    assert!(
        parsed_count > 0,
        "Should successfully parse at least some related articles with actual models"
    );
}

/// Test that ELink models can parse real PMC links API responses
#[tokio::test]
#[traced_test]
async fn test_pmc_links_model_parsing() {
    let mock_server = MockServer::start().await;
    let mut parsed_count = 0;

    for pmid in TEST_PMIDS {
        // Try both relative paths depending on where the test is run from
        let fixture_path_workspace = format!(
            "pubmed-client/tests/integration/test_data/api_responses/elink/pmc_links_{pmid}.json"
        );
        let fixture_path_local =
            format!("tests/integration/test_data/api_responses/elink/pmc_links_{pmid}.json");

        let fixture_path = if std::path::Path::new(&fixture_path_workspace).exists() {
            fixture_path_workspace
        } else if std::path::Path::new(&fixture_path_local).exists() {
            fixture_path_local
        } else {
            format!("test_data/api_responses/elink/pmc_links_{pmid}.json") // fallback
        };

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(pmid = pmid, "PMC links fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Mock the ELink API response with real fixture data
        Mock::given(method("GET"))
            .and(path("/elink.fcgi"))
            .and(query_param("dbfrom", "pubmed"))
            .and(query_param("db", "pmc"))
            .and(query_param("id", pmid.to_string()))
            .and(query_param("linkname", "pubmed_pmc"))
            .and(query_param("retmode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(content))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Create client pointing to mock server
        let config = ClientConfig::new().with_base_url(mock_server.uri());
        let client = PubMedClient::with_config(config);

        // Test that the actual client method can parse the fixture
        match client.get_pmc_links(&[*pmid]).await {
            Ok(pmc_links) => {
                // Validate the parsed model
                assert!(
                    pmc_links.source_pmids.contains(pmid),
                    "Should contain source PMID"
                );

                parsed_count += 1;
                info!(
                    pmid = pmid,
                    pmc_count = pmc_links.pmc_ids.len(),
                    "Successfully parsed PMC links with actual model"
                );
            }
            Err(e) => {
                warn!(
                    pmid = pmid,
                    error = %e,
                    "Failed to parse PMC links with actual model"
                );
            }
        }
    }

    assert!(
        parsed_count > 0,
        "Should successfully parse at least some PMC links with actual models"
    );
}

/// Test that ELink models can parse real citations API responses
#[tokio::test]
#[traced_test]
async fn test_citations_model_parsing() {
    let mock_server = MockServer::start().await;
    let mut parsed_count = 0;

    for pmid in TEST_PMIDS {
        // Try both relative paths depending on where the test is run from
        let fixture_path_workspace = format!(
            "pubmed-client/tests/integration/test_data/api_responses/elink/citations_{pmid}.json"
        );
        let fixture_path_local =
            format!("tests/integration/test_data/api_responses/elink/citations_{pmid}.json");

        let fixture_path = if std::path::Path::new(&fixture_path_workspace).exists() {
            fixture_path_workspace
        } else if std::path::Path::new(&fixture_path_local).exists() {
            fixture_path_local
        } else {
            format!("test_data/api_responses/elink/citations_{pmid}.json") // fallback
        };

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(pmid = pmid, "Citations fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Mock the ELink API response with real fixture data
        Mock::given(method("GET"))
            .and(path("/elink.fcgi"))
            .and(query_param("dbfrom", "pubmed"))
            .and(query_param("db", "pubmed"))
            .and(query_param("id", pmid.to_string()))
            .and(query_param("linkname", "pubmed_pubmed_citedin"))
            .and(query_param("retmode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(content))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Create client pointing to mock server
        let config = ClientConfig::new().with_base_url(mock_server.uri());
        let client = PubMedClient::with_config(config);

        // Test that the actual client method can parse the fixture
        match client.get_citations(&[*pmid]).await {
            Ok(citations) => {
                // Validate the parsed model
                assert!(
                    citations.source_pmids.contains(pmid),
                    "Should contain source PMID"
                );
                assert_eq!(
                    citations.link_type, "pubmed_pubmed_citedin",
                    "Should have correct link type"
                );

                parsed_count += 1;
                info!(
                    pmid = pmid,
                    citations_count = citations.citing_pmids.len(),
                    "Successfully parsed citations with actual model"
                );
            }
            Err(e) => {
                warn!(
                    pmid = pmid,
                    error = %e,
                    "Failed to parse citations with actual model"
                );
            }
        }
    }

    assert!(
        parsed_count > 0,
        "Should successfully parse at least some citations with actual models"
    );
}
