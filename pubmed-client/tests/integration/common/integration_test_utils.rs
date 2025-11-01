//! Utilities specifically for integration tests that make real API calls
//! These are only used by pubmed_api_tests, pmc_api_tests, and error_handling_tests

#![allow(dead_code)]

use pubmed_client::{Client, ClientConfig, PmcClient, PubMedClient};

/// Helper function to check if real API tests should be run
/// Requires both the integration-tests feature and the PUBMED_REAL_API_TESTS env var
pub fn should_run_real_api_tests() -> bool {
    std::env::var("PUBMED_REAL_API_TESTS").is_ok()
}

/// Create a test client with appropriate configuration for integration tests
pub fn create_test_client() -> Client {
    let mut config = ClientConfig::new()
        .with_email("test@example.com")
        .with_tool("pubmed-client-rs-integration-tests")
        .with_rate_limit(2.0); // Conservative rate limiting for tests

    // Use API key if available
    if let Ok(api_key) = std::env::var("NCBI_API_KEY") {
        config = config.with_api_key(&api_key).with_rate_limit(8.0); // Higher limit with API key, but still conservative
    }

    Client::with_config(config)
}

/// Create a PubMed-specific client for integration tests
pub fn create_test_pubmed_client() -> PubMedClient {
    let mut config = ClientConfig::new()
        .with_email("test@example.com")
        .with_tool("pubmed-client-rs-pubmed-integration-tests")
        .with_rate_limit(2.0);

    if let Ok(api_key) = std::env::var("NCBI_API_KEY") {
        config = config.with_api_key(&api_key).with_rate_limit(8.0);
    }

    PubMedClient::with_config(config)
}

/// Create a PMC-specific client for integration tests
pub fn create_test_pmc_client() -> PmcClient {
    let mut config = ClientConfig::new()
        .with_email("test@example.com")
        .with_tool("pubmed-client-rs-pmc-integration-tests")
        .with_rate_limit(2.0);

    if let Ok(api_key) = std::env::var("NCBI_API_KEY") {
        config = config.with_api_key(&api_key).with_rate_limit(8.0);
    }

    PmcClient::with_config(config)
}

/// Known PMIDs for integration testing (these are stable, well-formed articles)
pub const TEST_PMIDS: &[u32] = &[
    31978945, // COVID-19 research
    25760099, // CRISPR-Cas9 research
    33515491, // Cancer treatment
    32887691, // Machine learning in medicine
    28495875, // Genomics research
];

/// Known PMIDs as strings for string-based operations
pub const TEST_PMIDS_STR: &[&str] = &[
    "31978945", // COVID-19 research
    "25760099", // CRISPR-Cas9 research
    "33515491", // Cancer treatment
    "32887691", // Machine learning in medicine
    "28495875", // Genomics research
];

/// Known PMCIDs for integration testing
pub const TEST_PMCIDS: &[&str] = &[
    "PMC7138338", // COVID-19 article
    "PMC4395896", // CRISPR article
    "PMC7894017", // Cancer research
    "PMC7567892", // Machine learning
    "PMC5431048", // Genomics
];

/// Test queries for search functionality
pub const TEST_SEARCH_QUERIES: &[&str] = &[
    "COVID-19[Title]",
    "CRISPR[Title]",
    "cancer treatment[Title]",
    "machine learning[Title]",
    "genomics[Title]",
];
