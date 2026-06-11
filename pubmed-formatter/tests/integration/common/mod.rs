//! Common test utilities — thin wrappers around pubmed_test_utils.
#![allow(dead_code)]

pub use pubmed_test_utils::PmcXmlTestCase;

pub fn get_pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    pubmed_test_utils::get_pmc_xml_test_cases_from(env!("CARGO_MANIFEST_DIR"))
}

pub fn get_pmc_xml_test_case(filename: &str) -> Option<PmcXmlTestCase> {
    pubmed_test_utils::get_pmc_xml_test_case_from(env!("CARGO_MANIFEST_DIR"), filename)
}

pub fn pmc_xml_test_cases() -> Vec<PmcXmlTestCase> {
    get_pmc_xml_test_cases()
}
