//! Simple ELink API parsing tests using real fixtures
//!
//! Tests that the ELink models can successfully parse real NCBI API responses.

use serde_json::Value;
use std::fs;
use tracing::{info, warn};
use tracing_test::traced_test;

use pubmed_client_rs::pubmed::models::{Citations, PmcLinks, RelatedArticles};

/// Test PMIDs to check
const TEST_PMIDS: &[u32] = &[31978945, 33515491, 32887691, 25760099, 28495875];

/// Test that we can parse related articles responses
#[tokio::test]
#[traced_test]
async fn test_parse_related_articles() {
    let mut parsed_count = 0;

    for pmid in TEST_PMIDS {
        let fixture_path = format!(
            "tests/integration/test_data/api_responses/elink/related_{}.json",
            pmid
        );

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(pmid = pmid, "Related articles fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Test basic JSON parsing
        let json: Value = serde_json::from_str(&content).expect("Should parse JSON");

        // Test that we can parse as RelatedArticles model
        let related_articles: RelatedArticles =
            serde_json::from_value(json).expect("Should parse as RelatedArticles model");

        // Basic validation
        assert!(related_articles.source_pmids.contains(pmid));

        parsed_count += 1;
        info!(
            pmid = pmid,
            count = related_articles.related_pmids.len(),
            "Successfully parsed related articles"
        );
    }

    info!(
        parsed = parsed_count,
        "Related articles parsing test complete"
    );
    assert!(
        parsed_count > 0,
        "Should successfully parse at least some related articles"
    );
}

/// Test that we can parse PMC links responses
#[tokio::test]
#[traced_test]
async fn test_parse_pmc_links() {
    let mut parsed_count = 0;

    for pmid in TEST_PMIDS {
        let fixture_path = format!(
            "tests/integration/test_data/api_responses/elink/pmc_links_{}.json",
            pmid
        );

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(pmid = pmid, "PMC links fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Test basic JSON parsing
        let json: Value = serde_json::from_str(&content).expect("Should parse JSON");

        // Test that we can parse as PmcLinks model
        let pmc_links: PmcLinks =
            serde_json::from_value(json).expect("Should parse as PmcLinks model");

        // Basic validation
        assert!(pmc_links.source_pmids.contains(pmid));

        parsed_count += 1;
        info!(
            pmid = pmid,
            count = pmc_links.pmc_ids.len(),
            "Successfully parsed PMC links"
        );
    }

    info!(parsed = parsed_count, "PMC links parsing test complete");
    assert!(
        parsed_count > 0,
        "Should successfully parse at least some PMC links"
    );
}

/// Test that we can parse citations responses
#[tokio::test]
#[traced_test]
async fn test_parse_citations() {
    let mut parsed_count = 0;

    for pmid in TEST_PMIDS {
        let fixture_path = format!(
            "tests/integration/test_data/api_responses/elink/citations_{}.json",
            pmid
        );

        if !std::path::Path::new(&fixture_path).exists() {
            warn!(pmid = pmid, "Citations fixture not found, skipping");
            continue;
        }

        let content = fs::read_to_string(&fixture_path).expect("Should read fixture");

        // Test basic JSON parsing
        let json: Value = serde_json::from_str(&content).expect("Should parse JSON");

        // Test that we can parse as Citations model
        let citations: Citations =
            serde_json::from_value(json).expect("Should parse as Citations model");

        // Basic validation
        assert!(citations.source_pmids.contains(pmid));

        parsed_count += 1;
        info!(
            pmid = pmid,
            count = citations.citing_pmids.len(),
            "Successfully parsed citations"
        );
    }

    info!(parsed = parsed_count, "Citations parsing test complete");
    assert!(
        parsed_count > 0,
        "Should successfully parse at least some citations"
    );
}
