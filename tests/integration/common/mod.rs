//! Common test utilities for PMC XML parsing tests

use std::fs;
use std::path::{Path, PathBuf};

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
    pub fn read_xml_content_or_panic(&self) -> String {
        self.read_xml_content()
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", self.file_path))
    }
}

/// Get all PMC XML test files from the test data directory
pub fn get_pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    let xml_dir = Path::new("tests/test_data/pmc_xml");

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
    let xml_path = Path::new("tests/test_data/pmc_xml").join(filename);

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
}
