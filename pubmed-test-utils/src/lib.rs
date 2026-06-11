//! Shared test utilities for pubmed-* crates.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// Returns the workspace root directory using CARGO_MANIFEST_DIR of the caller.
///
/// Integration test binaries should call this macro instead, since
/// `env!("CARGO_MANIFEST_DIR")` resolves at compile time in the calling crate.
/// Use the [`workspace_root_from`] function passing `env!("CARGO_MANIFEST_DIR")`.
pub fn workspace_root_from(manifest_dir: &str) -> PathBuf {
    Path::new(manifest_dir)
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .to_path_buf()
}

/// Test case for a PMC XML file.
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

/// Test case for a PubMed XML file.
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

fn collect_xml_files(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

/// Returns all PMC XML test cases from `<workspace_root>/test_data/pmc_xml/`.
///
/// Pass `env!("CARGO_MANIFEST_DIR")` as `manifest_dir`.
pub fn get_pmc_xml_test_cases_from(manifest_dir: &str) -> Vec<PmcXmlTestCase> {
    let dir = workspace_root_from(manifest_dir).join("test_data/pmc_xml");
    collect_xml_files(&dir)
        .into_iter()
        .map(PmcXmlTestCase::new)
        .collect()
}

/// Returns a specific PMC XML test case by filename.
pub fn get_pmc_xml_test_case_from(manifest_dir: &str, filename: &str) -> Option<PmcXmlTestCase> {
    let path = workspace_root_from(manifest_dir)
        .join("test_data/pmc_xml")
        .join(filename);
    path.exists().then(|| PmcXmlTestCase::new(path))
}

/// Returns all PubMed XML test cases from `<workspace_root>/test_data/pubmed_xml/`.
///
/// Pass `env!("CARGO_MANIFEST_DIR")` as `manifest_dir`.
pub fn get_pubmed_xml_test_cases_from(manifest_dir: &str) -> Vec<PubMedXmlTestCase> {
    let dir = workspace_root_from(manifest_dir).join("test_data/pubmed_xml");
    collect_xml_files(&dir)
        .into_iter()
        .map(PubMedXmlTestCase::new)
        .collect()
}

/// Returns a specific PubMed XML test case by PMID.
pub fn get_pubmed_xml_test_case_from(manifest_dir: &str, pmid: &str) -> Option<PubMedXmlTestCase> {
    let path = workspace_root_from(manifest_dir)
        .join("test_data/pubmed_xml")
        .join(format!("{pmid}.xml"));
    path.exists().then(|| PubMedXmlTestCase::new(path))
}

/// Returns true if `content` is a Git LFS pointer rather than real file content.
pub fn is_git_lfs_pointer(content: &str) -> bool {
    content.starts_with("version https://git-lfs.github.com/spec/v1")
}
