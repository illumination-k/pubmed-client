//! Simple EInfo API parsing tests using real fixtures
//!
//! Tests that the EInfo models can successfully parse real NCBI API responses.

use serde_json::Value;
use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;

use pubmed_client_rs::pubmed::models::DatabaseInfo;

/// Test databases to check
const TEST_DATABASES: &[&str] = &[
    "pubmed",
    "pmc",
    "protein",
    "nucleotide",
    "genome",
    "structure",
    "taxonomy",
    "snp",
    "assembly",
    "bioproject",
];

/// Test that we can parse the database list response
#[tokio::test]
#[traced_test]
async fn test_parse_database_list() {
    let fixture_path = "tests/integration/test_data/api_responses/einfo/database_list.json";

    if !std::path::Path::new(fixture_path).exists() {
        warn!("Database list fixture not found, skipping test");
        return;
    }

    let content = fs::read_to_string(fixture_path).expect("Should read database list fixture");

    let json: Value = serde_json::from_str(&content).expect("Should parse database list JSON");

    // Check basic structure exists
    assert!(
        json["einforesult"]["dblist"].is_array(),
        "Should have database list"
    );

    info!("Successfully parsed database list fixture");
}

/// Test that we can parse individual database info responses
#[tokio::test]
#[traced_test]
async fn test_parse_database_info() {
    let mut parsed_count = 0;
    let mut total_count = 0;

    for database in TEST_DATABASES {
        total_count += 1;
        let fixture_path = format!(
            "tests/integration/test_data/api_responses/einfo/{}_info.json",
            database
        );

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(database = database, "Fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Test basic JSON parsing
        let json: Value = serde_json::from_str(&content).expect("Should parse JSON");

        // Test that required fields exist
        let einfo_result = &json["einforesult"];
        assert!(
            einfo_result["dbinfo"].is_array(),
            "Should have dbinfo array"
        );

        let db_info_array = einfo_result["dbinfo"].as_array().unwrap();
        assert!(!db_info_array.is_empty(), "Should have database info");

        let db_info = &db_info_array[0];
        assert_eq!(
            db_info["dbname"].as_str().unwrap(),
            *database,
            "Database name should match"
        );

        // Test that we can parse as DatabaseInfo model
        let database_info: DatabaseInfo =
            serde_json::from_value(db_info.clone()).expect("Should parse as DatabaseInfo model");

        assert_eq!(database_info.name, *database);
        assert!(!database_info.description.is_empty());

        parsed_count += 1;
        info!(database = database, "Successfully parsed database info");
    }

    info!(
        parsed = parsed_count,
        total = total_count,
        "EInfo parsing test complete"
    );

    // Ensure we parsed at least some databases
    assert!(
        parsed_count > 0,
        "Should successfully parse at least some databases"
    );
}
