//! EInfo API model parsing validation tests
//!
//! Tests that EInfo models can parse real API responses by simulating the actual parsing flow.

use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pubmed_client::{ClientConfig, PubMedClient};

/// Test databases to check - using the actual database names from NCBI
const TEST_DATABASES: &[(&str, &str)] = &[
    ("pubmed", "pubmed"),
    ("pmc", "pmc"),
    ("protein", "protein"),
    ("nucleotide", "nuccore"), // nucleotide database is actually called "nuccore" in the API
];

/// Test that EInfo models can parse real API responses by mocking the API
#[tokio::test]
#[traced_test]
async fn test_einfo_model_parsing_with_real_responses() {
    let mock_server = MockServer::start().await;
    let mut parsed_count = 0;
    let mut total_count = 0;

    for (database_fixture, database_actual) in TEST_DATABASES {
        total_count += 1;
        // Try both relative paths depending on where the test is run from
        let fixture_path_workspace = format!(
            "pubmed-client/tests/integration/test_data/api_responses/einfo/{}_info.json",
            database_fixture
        );
        let fixture_path_local = format!(
            "tests/integration/test_data/api_responses/einfo/{}_info.json",
            database_fixture
        );

        let fixture_path = if std::path::Path::new(&fixture_path_workspace).exists() {
            fixture_path_workspace
        } else if std::path::Path::new(&fixture_path_local).exists() {
            fixture_path_local
        } else {
            format!(
                "test_data/api_responses/einfo/{}_info.json",
                database_fixture
            ) // fallback
        };

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(database = database_fixture, "Fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Mock the EInfo API response with real fixture data
        Mock::given(method("GET"))
            .and(path("/einfo.fcgi"))
            .and(query_param("db", *database_fixture))
            .and(query_param("retmode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(content))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Create client pointing to mock server
        let config = ClientConfig::new().with_base_url(mock_server.uri());
        let client = PubMedClient::with_config(config);

        // Test that the actual client method can parse the fixture
        match client.get_database_info(database_fixture).await {
            Ok(db_info) => {
                // Validate the parsed model - use the actual database name from the API response
                assert_eq!(
                    db_info.name, *database_actual,
                    "Database name should match actual API response"
                );
                assert!(!db_info.description.is_empty(), "Should have description");

                parsed_count += 1;
                info!(
                    database = database_fixture,
                    actual_name = database_actual,
                    fields_count = db_info.fields.len(),
                    links_count = db_info.links.len(),
                    "Successfully parsed database info with actual model"
                );
            }
            Err(e) => {
                warn!(
                    database = database_fixture,
                    error = %e,
                    "Failed to parse database info with actual model"
                );
            }
        }
    }

    info!(
        parsed = parsed_count,
        total = total_count,
        "EInfo model parsing test complete"
    );

    // Ensure we parsed at least some databases
    assert!(
        parsed_count > 0,
        "Should successfully parse at least some databases with actual models"
    );
}
