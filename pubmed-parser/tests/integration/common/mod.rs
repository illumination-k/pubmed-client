//! Common test utilities for PMC and PubMed XML parsing tests
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .to_path_buf()
}

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

    pub fn read_xml_content_or_panic(&self) -> String {
        self.read_xml_content()
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", self.file_path))
    }
}

/// Get all PMC XML test files from the test data directory
pub fn get_pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    let xml_dir = workspace_root().join("test_data/pmc_xml");

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
    let xml_path = workspace_root().join("test_data/pmc_xml").join(filename);

    if xml_path.exists() {
        Some(PmcXmlTestCase::new(xml_path))
    } else {
        None
    }
}

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

    pub fn read_xml_content_or_panic(&self) -> String {
        self.read_xml_content()
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", self.file_path))
    }
}

/// Get all PubMed XML test files from the test data directory
pub fn get_pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    let xml_dir = workspace_root().join("test_data/pubmed_xml");

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
    let xml_path = workspace_root()
        .join("test_data/pubmed_xml")
        .join(format!("{pmid}.xml"));

    if xml_path.exists() {
        Some(PubMedXmlTestCase::new(xml_path))
    } else {
        None
    }
}

pub fn pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    get_pubmed_xml_test_cases()
}

/// Check if file content is a Git LFS pointer (not actual content)
pub fn is_git_lfs_pointer(content: &str) -> bool {
    content.starts_with("version https://git-lfs.github.com/spec/v1")
}
