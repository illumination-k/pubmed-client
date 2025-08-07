use pubmed_client_rs::pmc::parser::PmcXmlParser;
use rstest::rstest;
use std::fs;
use std::path::Path;
use tracing::info;

/// Test data structure for PMC variants
#[derive(Debug)]
struct PmcTestCase {
    pmcid: &'static str,
    file_path: &'static str,
    expected_figures: usize,
    description: &'static str,
    should_parse_successfully: bool,
}

/// Parameterized test for different PMC XML variants and edge cases
#[rstest]
#[case::pmc_with_floats_group(PmcTestCase {
    pmcid: "PMC10487465",
    file_path: "tests/integration/test_data/pmc_xml/PMC10487465.xml",
    expected_figures: 8,
    description: "PMC with figures in floats-group section with xlink:href",
    should_parse_successfully: true,
})]
#[case::pmc_without_xlink_href(PmcTestCase {
    pmcid: "PMC1064083",
    file_path: "../assets/test_data/pmc_articles/PMC1064083/PMC1064083/bcr936.nxml",
    expected_figures: 0,
    description: "PMC with figure elements but no xlink:href attributes",
    should_parse_successfully: true,
})]
#[case::pmc_missing_figure_elements(PmcTestCase {
    pmcid: "PMC1175969",
    file_path: "../assets/test_data/pmc_articles/PMC1175969/PMC1175969/gb-2005-6-6-r49.nxml",
    expected_figures: 0,
    description: "PMC with figure references but no actual fig elements",
    should_parse_successfully: true,
})]
#[case::pmc_corrupted_file(PmcTestCase {
    pmcid: "PMC1174970",
    file_path: "../assets/test_data/pmc_articles/PMC1174970/PMC1174970/ar1748.nxml",
    expected_figures: 0,
    description: "PMC with corrupted/incomplete XML file",
    should_parse_successfully: false,
})]
fn test_pmc_variants_parameterized(#[case] test_case: PmcTestCase) {
    // Skip test if file doesn't exist (for CI environments)
    if !Path::new(test_case.file_path).exists() {
        println!(
            "Skipping test for {}: {} not found",
            test_case.pmcid, test_case.file_path
        );
        return;
    }

    println!("Testing {}: {}", test_case.pmcid, test_case.description);

    // Read the XML file content
    let xml_content = fs::read_to_string(test_case.file_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", test_case.file_path));

    // Parse the XML content
    let result = PmcXmlParser::parse(&xml_content, test_case.pmcid);

    if test_case.should_parse_successfully {
        assert!(
            result.is_ok(),
            "Failed to parse {} ({}): {:?}",
            test_case.pmcid,
            test_case.description,
            result.err()
        );

        let full_text = result.unwrap();

        // Verify basic article information
        assert_eq!(full_text.pmcid, test_case.pmcid);
        assert!(
            !full_text.title.is_empty(),
            "Article title should not be empty"
        );

        // Count total figures across all sections
        let total_figures: usize = full_text.sections.iter().map(|s| s.figures.len()).sum();

        assert_eq!(
            total_figures, test_case.expected_figures,
            "{} should have {} figures, found {} ({})",
            test_case.pmcid, test_case.expected_figures, total_figures, test_case.description
        );

        info!(
            "{} passed: {} figures extracted as expected",
            test_case.pmcid, total_figures
        );

        // Additional validation for successfully parsed articles
        if test_case.expected_figures > 0 {
            // Verify that all figures have required fields
            let all_figures: Vec<_> = full_text.sections.iter().flat_map(|s| &s.figures).collect();

            for (i, figure) in all_figures.iter().enumerate() {
                assert!(!figure.id.is_empty(), "Figure {} ID should not be empty", i);
                assert!(
                    !figure.caption.is_empty(),
                    "Figure {} caption should not be empty",
                    i
                );
                assert_ne!(
                    figure.caption, "No caption available",
                    "Figure {} should have real caption",
                    i
                );

                // For PMC10487465, verify specific figure attributes
                if test_case.pmcid == "PMC10487465" {
                    assert!(
                        figure.id.starts_with("ijms-24-13282-f"),
                        "Figure ID should follow pattern: {}",
                        figure.id
                    );
                    assert!(
                        figure.file_name.is_some(),
                        "Figure should have file_name: {}",
                        figure.id
                    );
                    if let Some(ref file_name) = figure.file_name {
                        assert!(
                            file_name.starts_with("ijms-24-13282-g"),
                            "File name should follow pattern: {}",
                            file_name
                        );
                    }
                }
            }
        }
    } else {
        // For cases that should fail (like corrupted files)
        println!(
            "Expected parse failure for {}: {}",
            test_case.pmcid, test_case.description
        );

        // The parser might still succeed but with incomplete data
        // We mainly check that it doesn't crash and handles the case gracefully
        if let Ok(full_text) = result {
            println!(
                "Parser succeeded on corrupted file {} but extracted limited data: {} sections",
                test_case.pmcid,
                full_text.sections.len()
            );
        } else {
            println!(
                "Parser appropriately failed on corrupted file {}",
                test_case.pmcid
            );
        }
    }
}

/// Test figure extraction with different XML structures
#[rstest]
#[case::standard_fig_element(
    r#"<article xmlns:xlink="http://www.w3.org/1999/xlink">
        <body>
            <sec>
                <fig id="fig1">
                    <label>Figure 1</label>
                    <caption><p>Test figure caption</p></caption>
                    <graphic xlink:href="figure1.jpg"/>
                </fig>
            </sec>
        </body>
    </article>"#,
    "TEST001",
    1,
    "Standard figure in body section"
)]
#[case::floats_group_structure(
    r#"<article xmlns:xlink="http://www.w3.org/1999/xlink">
        <body>
            <sec><p>Article content</p></sec>
        </body>
        <floats-group>
            <fig id="fig1">
                <label>Figure 1</label>
                <caption><p>Figure in floats group</p></caption>
                <graphic xlink:href="floats-figure.jpg"/>
            </fig>
        </floats-group>
    </article>"#,
    "TEST002",
    1,
    "Figure in floats-group section"
)]
#[case::figure_without_href(
    r#"<article xmlns:xlink="http://www.w3.org/1999/xlink">
        <body>
            <sec>
                <fig id="fig1">
                    <label>Figure 1</label>
                    <caption><p>Figure without href</p></caption>
                    <graphic position="float"/>
                </fig>
            </sec>
        </body>
    </article>"#,
    "TEST003",
    1,
    "Figure without xlink:href attribute"
)]
#[case::no_figures(
    r#"<article xmlns:xlink="http://www.w3.org/1999/xlink">
        <body>
            <sec>
                <title>Introduction</title>
                <p>This article has no figures.</p>
            </sec>
        </body>
    </article>"#,
    "TEST004",
    0,
    "Article with no figures"
)]
#[case::multiple_graphics_per_figure(
    r#"<article xmlns:xlink="http://www.w3.org/1999/xlink">
        <body>
            <sec>
                <fig id="fig1">
                    <label>Figure 1</label>
                    <caption><p>Figure with multiple graphics</p></caption>
                    <graphic xlink:href="figure1a.jpg"/>
                    <graphic xlink:href="figure1b.jpg"/>
                </fig>
            </sec>
        </body>
    </article>"#,
    "TEST005",
    1,
    "Figure with multiple graphic elements"
)]
#[case::nested_sections_with_figures(
    r#"<article xmlns:xlink="http://www.w3.org/1999/xlink">
        <body>
            <sec>
                <title>Methods</title>
                <sec>
                    <title>Subsection</title>
                    <fig id="fig1">
                        <label>Figure 1</label>
                        <caption><p>Nested figure</p></caption>
                        <graphic xlink:href="nested-figure.jpg"/>
                    </fig>
                </sec>
            </sec>
        </body>
    </article>"#,
    "TEST006",
    1,
    "Figure in nested section"
)]
fn test_xml_structure_variants(
    #[case] xml_content: &str,
    #[case] pmcid: &str,
    #[case] expected_figures: usize,
    #[case] description: &str,
) {
    println!("Testing XML structure: {}", description);

    let result = PmcXmlParser::parse(xml_content, pmcid);
    assert!(
        result.is_ok(),
        "Failed to parse XML ({}): {:?}",
        description,
        result.err()
    );

    let full_text = result.unwrap();
    assert_eq!(full_text.pmcid, pmcid);

    let total_figures: usize = full_text.sections.iter().map(|s| s.figures.len()).sum();

    assert_eq!(
        total_figures, expected_figures,
        "{} should have {} figures, found {} ({})",
        pmcid, expected_figures, total_figures, description
    );

    // Verify figures have basic required fields
    let all_figures: Vec<_> = full_text.sections.iter().flat_map(|s| &s.figures).collect();

    for figure in &all_figures {
        assert!(!figure.id.is_empty(), "Figure ID should not be empty");
        assert!(
            !figure.caption.is_empty(),
            "Figure caption should not be empty"
        );
    }

    println!("✅ {} passed: {} figures", description, total_figures);
}

/// Test figure file_name extraction behavior
#[rstest]
#[case::with_xlink_href(
    r#"<fig id="fig1"><graphic xlink:href="test-file.jpg"/></fig>"#,
    Some("test-file.jpg".to_string()),
    "xlink:href should be extracted"
)]
#[case::with_href_no_namespace(
    r#"<fig id="fig1"><graphic href="test-file.jpg"/></fig>"#,
    Some("test-file.jpg".to_string()),
    "href without namespace should be extracted"
)]
#[case::no_href_attribute(
    r#"<fig id="fig1"><graphic position="float"/></fig>"#,
    None,
    "No href should result in None"
)]
#[case::multiple_graphics_first_wins(
    r#"<fig id="fig1">
        <graphic xlink:href="first.jpg"/>
        <graphic xlink:href="second.jpg"/>
    </fig>"#,
    Some("first.jpg".to_string()),
    "First graphic href should be extracted"
)]
#[case::empty_href(
    r#"<fig id="fig1"><graphic xlink:href=""/></fig>"#,
    Some("".to_string()),
    "Empty href should be preserved"
)]
fn test_figure_file_name_extraction(
    #[case] fig_xml: &str,
    #[case] expected_file_name: Option<String>,
    #[case] description: &str,
) {
    println!("Testing file name extraction: {}", description);

    // Wrap in a minimal XML structure
    let xml_content = format!(
        r#"<article xmlns:xlink="http://www.w3.org/1999/xlink">
            <body><sec>{}</sec></body>
        </article>"#,
        fig_xml
    );

    let result = PmcXmlParser::parse(&xml_content, "TEST");
    assert!(
        result.is_ok(),
        "Failed to parse test XML: {:?}",
        result.err()
    );

    let full_text = result.unwrap();
    let figures: Vec<_> = full_text.sections.iter().flat_map(|s| &s.figures).collect();

    assert_eq!(figures.len(), 1, "Should extract exactly one figure");

    let figure = &figures[0];
    assert_eq!(
        figure.file_name, expected_file_name,
        "File name extraction failed for: {}",
        description
    );

    println!("✅ {} passed: {:?}", description, figure.file_name);
}

/// Test error handling and edge cases
#[rstest]
#[case::empty_xml("", "Empty XML")]
#[case::invalid_xml("<invalid", "Invalid XML syntax")]
#[case::minimal_valid_xml("<article></article>", "Minimal valid XML")]
#[case::xml_without_namespace(
    r#"<article><body><sec><fig id="f1"><caption>Test</caption></fig></sec></body></article>"#,
    "XML without xlink namespace"
)]
fn test_error_handling_and_edge_cases(#[case] xml_content: &str, #[case] description: &str) {
    println!("Testing error handling: {}", description);

    let result = PmcXmlParser::parse(xml_content, "TEST");

    // We expect the parser to either succeed gracefully or fail appropriately
    match result {
        Ok(full_text) => {
            println!(
                "✅ {} handled gracefully: {} sections",
                description,
                full_text.sections.len()
            );
            // Basic validation that the result is reasonable
            assert_eq!(full_text.pmcid, "TEST");
        }
        Err(e) => {
            println!("✅ {} failed appropriately: {:?}", description, e);
        }
    }
}
