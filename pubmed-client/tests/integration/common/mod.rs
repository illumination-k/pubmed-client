//! Common test utilities for PMC and PubMed XML parsing tests

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "integration-tests")]
pub mod integration_test_utils;

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
    // Try both relative paths depending on where the test is run from
    let xml_dir_workspace = Path::new("pubmed-client/tests/integration/test_data/pmc_xml");
    let xml_dir_local = Path::new("tests/integration/test_data/pmc_xml");
    let xml_dir_fallback = Path::new("test_data/pmc_xml");

    let xml_dir = if xml_dir_workspace.exists() {
        xml_dir_workspace
    } else if xml_dir_local.exists() {
        xml_dir_local
    } else {
        xml_dir_fallback
    };

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
    // Try both relative paths depending on where the test is run from
    let xml_path_workspace =
        Path::new("pubmed-client/tests/integration/test_data/pmc_xml").join(filename);
    let xml_path_local = Path::new("tests/integration/test_data/pmc_xml").join(filename);
    let xml_path_fallback = Path::new("test_data/pmc_xml").join(filename);

    if xml_path_workspace.exists() {
        Some(PmcXmlTestCase::new(xml_path_workspace))
    } else if xml_path_local.exists() {
        Some(PmcXmlTestCase::new(xml_path_local))
    } else if xml_path_fallback.exists() {
        Some(PmcXmlTestCase::new(xml_path_fallback))
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
    // Try both relative paths depending on where the test is run from
    let xml_dir_workspace = Path::new("pubmed-client/tests/integration/test_data/pubmed_xml");
    let xml_dir_local = Path::new("tests/integration/test_data/pubmed_xml");
    let xml_dir_fallback = Path::new("test_data/pubmed_xml");

    let xml_dir = if xml_dir_workspace.exists() {
        xml_dir_workspace
    } else if xml_dir_local.exists() {
        xml_dir_local
    } else {
        xml_dir_fallback
    };

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
    // Try both relative paths depending on where the test is run from
    let xml_path_workspace = Path::new("pubmed-client/tests/integration/test_data/pubmed_xml")
        .join(format!("{pmid}.xml"));
    let xml_path_local =
        Path::new("tests/integration/test_data/pubmed_xml").join(format!("{pmid}.xml"));
    let xml_path_fallback = Path::new("test_data/pubmed_xml").join(format!("{pmid}.xml"));

    let xml_path = if xml_path_workspace.exists() {
        xml_path_workspace
    } else if xml_path_local.exists() {
        xml_path_local
    } else {
        xml_path_fallback
    };

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
