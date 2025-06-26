//! Common test utilities for PMC and PubMed XML parsing tests

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "integration-tests")]
use pubmed_client_rs::{Client, ClientConfig, PmcClient, PubMedClient};

/// Test case structure for PMC XML files
#[derive(Debug, Clone)]
pub struct PmcXmlTestCase {
    pub file_path: PathBuf,
    pub pmcid: String,
}

impl PmcXmlTestCase {
    /// Create a new test case from a file path
    pub fn new(file_path: PathBuf) -> Self {
        let pmcid = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self { file_path, pmcid }
    }

    /// Get the filename as a string
    pub fn filename(&self) -> &str {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.xml")
    }

    /// Read the XML content of this test case
    pub fn read_xml_content(&self) -> Result<String, std::io::Error> {
        fs::read_to_string(&self.file_path)
    }

    /// Read the XML content or panic with a descriptive message
    #[allow(dead_code)]
    pub fn read_xml_content_or_panic(&self) -> String {
        self.read_xml_content()
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", self.file_path))
    }
}

/// Get all PMC XML test files from the test data directory
pub fn get_pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    let xml_dir = Path::new("tests/integration/test_data/pmc_xml");

    if !xml_dir.exists() {
        return Vec::new();
    }

    let mut xml_files = Vec::new();

    if let Ok(entries) = fs::read_dir(xml_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                xml_files.push(path);
            }
        }
    }

    // Sort by filename for consistent test ordering
    xml_files.sort();

    xml_files.into_iter().map(PmcXmlTestCase::new).collect()
}

/// Get a specific PMC XML test case by filename
pub fn get_pmc_xml_test_case(filename: &str) -> Option<PmcXmlTestCase> {
    let xml_path = Path::new("tests/integration/test_data/pmc_xml").join(filename);

    if xml_path.exists() {
        Some(PmcXmlTestCase::new(xml_path))
    } else {
        None
    }
}

/// Rstest fixture for all PMC XML test cases
#[allow(dead_code)]
pub fn pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    get_pmc_xml_test_cases()
}

/// Test case structure for PubMed XML files
#[derive(Debug, Clone)]
pub struct PubMedXmlTestCase {
    pub file_path: PathBuf,
    pub pmid: String,
}

impl PubMedXmlTestCase {
    /// Create a new test case from a file path
    pub fn new(file_path: PathBuf) -> Self {
        let pmid = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self { file_path, pmid }
    }

    /// Get the filename as a string
    pub fn filename(&self) -> &str {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.xml")
    }

    /// Read the XML content of this test case
    pub fn read_xml_content(&self) -> Result<String, std::io::Error> {
        fs::read_to_string(&self.file_path)
    }

    /// Read the XML content or panic with a descriptive message
    #[allow(dead_code)]
    pub fn read_xml_content_or_panic(&self) -> String {
        self.read_xml_content()
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", self.file_path))
    }
}

/// Get all PubMed XML test files from the test data directory
pub fn get_pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    let xml_dir = Path::new("tests/integration/test_data/pubmed_xml");

    if !xml_dir.exists() {
        return Vec::new();
    }

    let mut xml_files = Vec::new();

    if let Ok(entries) = fs::read_dir(xml_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                xml_files.push(path);
            }
        }
    }

    // Sort by filename for consistent test ordering
    xml_files.sort();

    xml_files.into_iter().map(PubMedXmlTestCase::new).collect()
}

/// Get a specific PubMed XML test case by PMID
pub fn get_pubmed_xml_test_case(pmid: &str) -> Option<PubMedXmlTestCase> {
    let xml_path =
        Path::new("tests/integration/test_data/pubmed_xml").join(format!("{}.xml", pmid));

    if xml_path.exists() {
        Some(PubMedXmlTestCase::new(xml_path))
    } else {
        None
    }
}

/// Rstest fixture for all PubMed XML test cases
#[allow(dead_code)]
pub fn pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    get_pubmed_xml_test_cases()
}

// Integration test configuration and utilities
#[cfg(feature = "integration-tests")]
pub const INTEGRATION_ENABLED: bool = true;

#[cfg(not(feature = "integration-tests"))]
pub const INTEGRATION_ENABLED: bool = false;

/// Helper function to check if real API tests should be run
/// Requires both the integration-tests feature and the PUBMED_REAL_API_TESTS env var
pub fn should_run_real_api_tests() -> bool {
    #[cfg(not(feature = "integration-tests"))]
    {
        false
    }

    #[cfg(feature = "integration-tests")]
    {
        std::env::var("PUBMED_REAL_API_TESTS").is_ok()
    }
}

/// Create a test client with appropriate configuration for integration tests
#[cfg(feature = "integration-tests")]
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
#[cfg(feature = "integration-tests")]
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
#[cfg(feature = "integration-tests")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pmc_xml_test_cases() {
        let test_cases = get_pmc_xml_test_cases();

        // We should have some test cases (assuming test data exists)
        if !test_cases.is_empty() {
            for test_case in &test_cases {
                assert!(test_case.file_path.exists());
                assert!(test_case.filename().ends_with(".xml"));
                assert!(!test_case.pmcid.is_empty());

                // Test that we can read the content
                let content = test_case.read_xml_content();
                assert!(content.is_ok());

                if let Ok(xml_content) = content {
                    assert!(!xml_content.is_empty());
                    assert!(xml_content.contains("<article"));
                }
            }
        }
    }

    #[test]
    fn test_get_specific_test_case() {
        let test_cases = get_pmc_xml_test_cases();

        if let Some(first_case) = test_cases.first() {
            let filename = first_case.filename();
            let specific_case = get_pmc_xml_test_case(filename);

            assert!(specific_case.is_some());
            let specific_case = specific_case.unwrap();
            assert_eq!(specific_case.filename(), filename);
            assert_eq!(specific_case.pmcid, first_case.pmcid);
        }
    }

    #[test]
    fn test_nonexistent_test_case() {
        let nonexistent = get_pmc_xml_test_case("nonexistent.xml");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_get_pubmed_xml_test_cases() {
        let test_cases = get_pubmed_xml_test_cases();

        // We should have some test cases (assuming test data exists)
        if !test_cases.is_empty() {
            for test_case in &test_cases {
                assert!(test_case.file_path.exists());
                assert!(test_case.filename().ends_with(".xml"));
                assert!(!test_case.pmid.is_empty());

                // Test that we can read the content
                let content = test_case.read_xml_content();
                assert!(content.is_ok());

                if let Ok(xml_content) = content {
                    assert!(!xml_content.is_empty());
                    assert!(
                        xml_content.contains("<PubmedArticle")
                            || xml_content.contains("<MedlineCitation")
                    );
                }
            }
        }
    }

    #[test]
    fn test_get_specific_pubmed_test_case() {
        let test_cases = get_pubmed_xml_test_cases();

        if let Some(first_case) = test_cases.first() {
            let pmid = &first_case.pmid;
            let specific_case = get_pubmed_xml_test_case(pmid);

            assert!(specific_case.is_some());
            let specific_case = specific_case.unwrap();
            assert_eq!(specific_case.pmid, *pmid);
            assert!(specific_case.filename().contains(pmid));
        }
    }

    #[test]
    fn test_nonexistent_pubmed_test_case() {
        let nonexistent = get_pubmed_xml_test_case("99999999");
        assert!(nonexistent.is_none());
    }
}
