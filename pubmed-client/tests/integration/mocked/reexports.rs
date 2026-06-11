//! Smoke tests: verify that pubmed-parser and pubmed-formatter types are accessible
//! through pubmed-client re-exports and that one representative parse succeeds.

use pubmed_client::pmc::parse_pmc_xml;
use pubmed_client::pmc::{HeadingStyle, MarkdownConfig, PmcMarkdownConverter, ReferenceStyle};
use pubmed_client::pubmed::parser::parse_article_from_xml;

#[path = "../common/mod.rs"]
mod common;

#[test]
fn reexport_parse_pmc_xml() {
    let test_case = match common::get_pmc_xml_test_cases().into_iter().next() {
        Some(tc) => tc,
        None => return, // no fixtures available, skip
    };
    let xml = test_case.read_xml_content_or_panic();
    if common::is_git_lfs_pointer(&xml) {
        return;
    }
    let result = parse_pmc_xml(&xml, &test_case.pmcid);
    assert!(result.is_ok(), "parse_pmc_xml failed: {:?}", result.err());
}

#[test]
fn reexport_parse_pubmed_xml() {
    let test_case = match common::get_pubmed_xml_test_cases().into_iter().next() {
        Some(tc) => tc,
        None => return,
    };
    let xml = test_case.read_xml_content_or_panic();
    if common::is_git_lfs_pointer(&xml) {
        return;
    }
    let result = parse_article_from_xml(&xml, &test_case.pmid);
    assert!(
        result.is_ok(),
        "parse_article_from_xml failed: {:?}",
        result.err()
    );
}

#[test]
fn reexport_markdown_converter_types_accessible() {
    // Verify the builder types compile and are accessible via pubmed-client re-exports.
    let _converter = PmcMarkdownConverter::with_config(MarkdownConfig {
        heading_style: HeadingStyle::ATX,
        reference_style: ReferenceStyle::Numbered,
        ..Default::default()
    });
}
