//! ESpell API model parsing validation tests
//!
//! Tests that ESpell models can parse real API responses by simulating the actual parsing flow.
//! Fixture files were downloaded from the NCBI ESpell API using scripts/download_espell_fixtures.sh

use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pubmed_client::{ClientConfig, PubMedClient};

/// Test cases: (fixture filename, input term, expect_corrections)
const SPELL_TEST_CASES: &[(&str, &str, bool)] = &[
    ("espell_asthmaa.xml", "asthmaa", true),
    ("espell_fiberblast.xml", "fiberblast", true),
    ("espell_correct.xml", "asthma", false),
];

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

/// Test that ESpell correctly identifies corrections for misspelled terms
#[tokio::test]
#[traced_test]
async fn test_espell_with_corrections_parsing() {
    let mock_server = MockServer::start().await;
    let mut tested_count = 0;

    for (fixture, term, expect_corrections) in SPELL_TEST_CASES {
        let fixture_path = resolve_fixture_path(&format!(
            "tests/integration/test_data/api_responses/espell/{fixture}"
        ));

        if fixture_path.is_none() {
            warn!(fixture = fixture, "ESpell fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

        Mock::given(method("GET"))
            .and(path("/espell.fcgi"))
            .and(query_param("db", "pubmed"))
            .and(query_param("term", *term))
            .respond_with(ResponseTemplate::new(200).set_body_string(content))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = ClientConfig::new()
            .with_base_url(mock_server.uri())
            .with_rate_limit(100.0);
        let client = PubMedClient::with_config(config);

        match client.spell_check(term).await {
            Ok(result) => {
                assert_eq!(result.database, "pubmed", "Database should be pubmed");
                assert_eq!(result.query, *term, "Query should match input term");
                assert_eq!(
                    result.has_corrections(),
                    *expect_corrections,
                    "Correction expectation mismatch for '{}': has_corrections={}, expected={}",
                    term,
                    result.has_corrections(),
                    expect_corrections
                );

                tested_count += 1;
                info!(
                    term = term,
                    corrected = %result.corrected_query,
                    has_corrections = result.has_corrections(),
                    "ESpell parsing test passed for term"
                );
            }
            Err(e) => {
                warn!(term = term, error = %e, "Failed to parse ESpell response");
            }
        }
    }

    assert!(
        tested_count > 0,
        "Should successfully test at least one spell check case"
    );
}

/// Test that ESpell returns the expected correction for 'asthmaa' -> 'asthma'
#[tokio::test]
#[traced_test]
async fn test_espell_asthmaa_correction() {
    let mock_server = MockServer::start().await;

    let fixture_path =
        resolve_fixture_path("tests/integration/test_data/api_responses/espell/espell_asthmaa.xml");

    if fixture_path.is_none() {
        warn!("ESpell asthmaa fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/espell.fcgi"))
        .and(query_param("db", "pubmed"))
        .and(query_param("term", "asthmaa"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    match client.spell_check("asthmaa").await {
        Ok(result) => {
            assert_eq!(result.query, "asthmaa", "Input query should be preserved");
            assert_eq!(
                result.corrected_query, "asthma",
                "Corrected query should be 'asthma'"
            );
            assert!(result.has_corrections(), "Should have corrections");

            let replacements = result.replacements();
            assert!(
                !replacements.is_empty(),
                "Should have at least one replacement"
            );
            assert!(
                replacements.contains(&"asthma"),
                "Should contain 'asthma' as a replacement"
            );

            info!(
                query = %result.query,
                corrected = %result.corrected_query,
                replacements = ?result.replacements(),
                "ESpell asthmaa correction test passed"
            );
        }
        Err(e) => {
            panic!("ESpell asthmaa parsing failed: {}", e);
        }
    }
}

/// Test that ESpell handles multiple corrections in a single query
#[tokio::test]
#[traced_test]
async fn test_espell_multiple_corrections_parsing() {
    let mock_server = MockServer::start().await;

    let fixture_path = resolve_fixture_path(
        "tests/integration/test_data/api_responses/espell/espell_multiple_corrections.xml",
    );

    if fixture_path.is_none() {
        warn!("ESpell multiple corrections fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/espell.fcgi"))
        .and(query_param("db", "pubmed"))
        .and(query_param("term", "asthmaa OR alergies"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    match client.spell_check("asthmaa OR alergies").await {
        Ok(result) => {
            assert_eq!(result.query, "asthmaa OR alergies");
            assert_eq!(
                result.corrected_query, "asthma or allergies",
                "Both misspellings should be corrected"
            );
            assert!(result.has_corrections(), "Should have corrections");

            let replacements = result.replacements();
            assert_eq!(replacements.len(), 2, "Should have 2 replacements");
            assert!(replacements.contains(&"asthma"), "Should contain 'asthma'");
            assert!(
                replacements.contains(&"allergies"),
                "Should contain 'allergies'"
            );

            info!(
                query = %result.query,
                corrected = %result.corrected_query,
                replacements_count = replacements.len(),
                "ESpell multiple corrections parsing test passed"
            );
        }
        Err(e) => {
            panic!("ESpell multiple corrections parsing failed: {}", e);
        }
    }
}

/// Test that ESpell returns no corrections for a correctly-spelled term
#[tokio::test]
#[traced_test]
async fn test_espell_no_corrections_for_correct_term() {
    let mock_server = MockServer::start().await;

    let fixture_path =
        resolve_fixture_path("tests/integration/test_data/api_responses/espell/espell_correct.xml");

    if fixture_path.is_none() {
        warn!("ESpell correct term fixture not found, skipping");
        return;
    }

    let content = fs::read_to_string(fixture_path.unwrap()).expect("Should read fixture");

    Mock::given(method("GET"))
        .and(path("/espell.fcgi"))
        .and(query_param("db", "pubmed"))
        .and(query_param("term", "asthma"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0);
    let client = PubMedClient::with_config(config);

    match client.spell_check("asthma").await {
        Ok(result) => {
            assert_eq!(result.query, "asthma", "Query should be preserved");
            assert!(
                !result.has_corrections(),
                "Correctly-spelled term should have no corrections"
            );
            assert!(
                result.replacements().is_empty(),
                "Should have no replacements"
            );

            info!(
                query = %result.query,
                "ESpell no-corrections test passed"
            );
        }
        Err(e) => {
            panic!("ESpell no-corrections parsing failed: {}", e);
        }
    }
}
