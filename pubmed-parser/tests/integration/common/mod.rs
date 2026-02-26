//! Common test utilities for PMC and PubMed XML parsing tests
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// Test case structure for PMC XML files
#[derive(Debug, Clone)]
pub struct PmcXmlTestCase {
    pub file_path: PathBuf,
    pub pmcid: String,
}

impl PmcXmlTestCase {
    pub fn new(file_path: PathBuf) -> Self {
        let pmcid = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        Self { file_path, pmcid }
    }

    pub fn filename(&self) -> &str {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.xml")
    }

    pub fn read_xml_content(&self) -> Result<String, std::io::Error> {
        fs::read_to_string(&self.file_path)
    }

    #[allow(dead_code)]
    pub fn read_xml_content_or_panic(&self) -> String {
        self.read_xml_content()
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", self.file_path))
    }
}

/// Get all PMC XML test files from the test data directory
pub fn get_pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    let xml_dir_workspace = Path::new("pubmed-parser/tests/integration/test_data/pmc_xml");
    let xml_dir_local = Path::new("tests/integration/test_data/pmc_xml");

    let xml_dir = if xml_dir_workspace.exists() {
        xml_dir_workspace
    } else {
        xml_dir_local
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
    xml_files.sort();
    xml_files.into_iter().map(PmcXmlTestCase::new).collect()
}

/// Get a specific PMC XML test case by filename
pub fn get_pmc_xml_test_case(filename: &str) -> Option<PmcXmlTestCase> {
    let xml_path_workspace =
        Path::new("pubmed-parser/tests/integration/test_data/pmc_xml").join(filename);
    let xml_path_local = Path::new("tests/integration/test_data/pmc_xml").join(filename);

    if xml_path_workspace.exists() {
        Some(PmcXmlTestCase::new(xml_path_workspace))
    } else if xml_path_local.exists() {
        Some(PmcXmlTestCase::new(xml_path_local))
    } else {
        None
    }
}

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
    pub fn new(file_path: PathBuf) -> Self {
        let pmid = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        Self { file_path, pmid }
    }

    pub fn filename(&self) -> &str {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.xml")
    }

    pub fn read_xml_content(&self) -> Result<String, std::io::Error> {
        fs::read_to_string(&self.file_path)
    }

    #[allow(dead_code)]
    pub fn read_xml_content_or_panic(&self) -> String {
        self.read_xml_content()
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", self.file_path))
    }
}

/// Get all PubMed XML test files from the test data directory
pub fn get_pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    let xml_dir_workspace = Path::new("pubmed-parser/tests/integration/test_data/pubmed_xml");
    let xml_dir_local = Path::new("tests/integration/test_data/pubmed_xml");

    let xml_dir = if xml_dir_workspace.exists() {
        xml_dir_workspace
    } else {
        xml_dir_local
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
    xml_files.sort();
    xml_files.into_iter().map(PubMedXmlTestCase::new).collect()
}

/// Get a specific PubMed XML test case by PMID
pub fn get_pubmed_xml_test_case(pmid: &str) -> Option<PubMedXmlTestCase> {
    let xml_path_workspace = Path::new("pubmed-parser/tests/integration/test_data/pubmed_xml")
        .join(format!("{pmid}.xml"));
    let xml_path_local =
        Path::new("tests/integration/test_data/pubmed_xml").join(format!("{pmid}.xml"));

    if xml_path_workspace.exists() {
        Some(PubMedXmlTestCase::new(xml_path_workspace))
    } else if xml_path_local.exists() {
        Some(PubMedXmlTestCase::new(xml_path_local))
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    get_pubmed_xml_test_cases()
}

/// Check if file content is a Git LFS pointer (not actual content)
pub fn is_git_lfs_pointer(content: &str) -> bool {
    content.starts_with("version https://git-lfs.github.com/spec/v1")
}
