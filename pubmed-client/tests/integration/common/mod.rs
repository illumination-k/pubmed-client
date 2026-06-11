//! Common test utilities — thin wrappers around pubmed_test_utils.
#![allow(dead_code, unused_imports)]

#[cfg(feature = "integration-tests")]
pub mod integration_test_utils;

pub use pubmed_test_utils::{PmcXmlTestCase, PubMedXmlTestCase, is_git_lfs_pointer};

pub fn workspace_root() -> std::path::PathBuf {
    pubmed_test_utils::workspace_root_from(env!("CARGO_MANIFEST_DIR"))
}

pub fn get_pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    pubmed_test_utils::get_pmc_xml_test_cases_from(env!("CARGO_MANIFEST_DIR"))
}

pub fn get_pmc_xml_test_case(filename: &str) -> Option<PmcXmlTestCase> {
    pubmed_test_utils::get_pmc_xml_test_case_from(env!("CARGO_MANIFEST_DIR"), filename)
}

pub fn pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    get_pmc_xml_test_cases()
}

pub fn get_pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    pubmed_test_utils::get_pubmed_xml_test_cases_from(env!("CARGO_MANIFEST_DIR"))
}

pub fn get_pubmed_xml_test_case(pmid: &str) -> Option<PubMedXmlTestCase> {
    pubmed_test_utils::get_pubmed_xml_test_case_from(env!("CARGO_MANIFEST_DIR"), pmid)
}

pub fn pubmed_xml_test_cases() -> Vec<PubMedXmlTestCase> {
    get_pubmed_xml_test_cases()
}
