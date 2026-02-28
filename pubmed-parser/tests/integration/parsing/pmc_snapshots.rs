//! Snapshot tests for PMC parser using real XML data.
//!
//! These tests pin the full `PmcFullText` output for representative articles.
//! Any parser change that alters output will produce a visible diff via `insta`.
//!
//! To update snapshots after intentional parser changes:
//! ```bash
//! cargo insta review
//! ```

use pubmed_parser::pmc::parse_pmc_xml;
use std::path::Path;

fn parse_test_article(pmcid: &str) -> pubmed_parser::pmc::PmcFullText {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let xml_path = workspace_root.join(format!("test_data/pmc_xml/{pmcid}.xml"));
    let xml_content = std::fs::read_to_string(&xml_path)
        .unwrap_or_else(|_| panic!("Failed to read {xml_path:?}"));
    parse_pmc_xml(&xml_content, pmcid).unwrap_or_else(|e| panic!("Failed to parse {pmcid}: {e}"))
}

/// PMC10000000: Minimal 1867 editorial (4KB).
/// No authors, DOI, abstract, sections, or references.
/// Tests edge case handling for sparse articles.
#[test]
fn test_snapshot_pmc10000000_minimal_editorial() {
    let article = parse_test_article("PMC10000000");
    insta::assert_json_snapshot!("PMC10000000", article);
}

/// PMC7906746: Lancet COVID-19 comment (41KB).
/// 23 authors, 1 figure, 22 references, copyright, acknowledgments.
/// Wrapped in `<pmc-articleset>` DTD container.
#[test]
fn test_snapshot_pmc7906746_lancet_comment() {
    let article = parse_test_article("PMC7906746");
    insta::assert_json_snapshot!("PMC7906746", article);
}

/// PMC10821037: Vaccines research article (264KB).
/// 15 authors with ORCIDs and CRediT roles, 6 keywords, funding with award ID,
/// 3 history dates, elocation-id, abstract, supplementary materials,
/// corresponding author, license URL. Exercises nearly all PmcFullText fields.
#[test]
fn test_snapshot_pmc10821037_vaccines_research() {
    let article = parse_test_article("PMC10821037");
    insta::assert_json_snapshot!("PMC10821037", article);
}
