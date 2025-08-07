use pubmed_client_rs::pmc::parser::PmcXmlParser;
use pubmed_client_rs::pmc::tar::PmcTarClient;
use pubmed_client_rs::ClientConfig;
use std::fs;
use std::path::Path;
use tracing::info;

#[tokio::test]
async fn test_extract_figures_uses_nxml_from_tar() {
    let config = ClientConfig::new();
    let client = PmcTarClient::new(config);

    // Test with a specific PMC ID
    let pmcid = "PMC9680858";
    let output_dir = Path::new("./test_extracted_figures_integration");

    info!("Extracting figures with captions for {}...", pmcid);

    // This should now use the NXML from the tar file instead of making an API call
    let result = client
        .extract_figures_with_captions(pmcid, output_dir)
        .await;

    assert!(
        result.is_ok(),
        "Failed to extract figures: {:?}",
        result.err()
    );

    let figures = result.unwrap();
    assert!(!figures.is_empty(), "No figures found");

    info!("Successfully extracted {} figures", figures.len());

    // Verify figures have expected data
    for figure in &figures {
        assert!(!figure.figure.id.is_empty(), "Figure ID is empty");
        assert!(!figure.figure.caption.is_empty(), "Figure caption is empty");
        assert!(
            !figure.extracted_file_path.is_empty(),
            "Extracted file path is empty"
        );

        // Verify the file actually exists
        let path = Path::new(&figure.extracted_file_path);
        assert!(
            path.exists(),
            "Extracted file does not exist: {}",
            figure.extracted_file_path
        );
    }

    // Clean up
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}

/// Integration test for NXML reading and figure extraction using PMC10487465
/// This test verifies that the enhanced XML parser correctly extracts figure metadata
/// from real PMC articles with xlink:href attributes in graphic elements.
#[test]
fn test_pmc10487465_nxml_parsing() {
    let test_file_path = "tests/integration/test_data/pmc_xml/PMC10487465.xml";

    // Skip test if file doesn't exist (for CI environments)
    if !std::path::Path::new(test_file_path).exists() {
        println!("Skipping test: {} not found", test_file_path);
        return;
    }

    // Read the NXML file content
    let xml_content = fs::read_to_string(test_file_path).expect("Failed to read NXML file");

    // Parse the XML content
    let result = PmcXmlParser::parse(&xml_content, "PMC10487465");
    assert!(
        result.is_ok(),
        "Failed to parse PMC10487465 NXML: {:?}",
        result.err()
    );

    let full_text = result.unwrap();

    // Verify basic article information
    assert_eq!(full_text.pmcid, "PMC10487465");
    assert!(
        !full_text.title.is_empty(),
        "Article title should not be empty"
    );
    assert!(
        !full_text.sections.is_empty(),
        "Article should have sections"
    );

    // Count total figures across all sections
    let mut total_figures = 0;
    let mut figures_with_file_name = 0;
    let mut figures_with_proper_href = 0;

    for section in &full_text.sections {
        for figure in &section.figures {
            total_figures += 1;

            println!("Figure found:");
            println!("  ID: {}", figure.id);
            println!("  Label: {:?}", figure.label);
            println!("  File name: {:?}", figure.file_name);
            println!(
                "  Caption: {}",
                &figure.caption.chars().take(100).collect::<String>()
            );

            if figure.file_name.is_some() {
                figures_with_file_name += 1;
            }

            // Check if file_name follows expected pattern (ijms-24-13282-g00X)
            if let Some(ref file_name) = figure.file_name {
                if file_name.starts_with("ijms-24-13282-g") {
                    figures_with_proper_href += 1;
                }
            }
        }
    }

    // Verify that figures were extracted
    println!("Total figures found: {}", total_figures);
    println!("Number of sections: {}", full_text.sections.len());
    for (i, section) in full_text.sections.iter().enumerate() {
        println!("Section {}: {} figures", i, section.figures.len());
    }
    assert!(
        total_figures > 0,
        "Should find figures in PMC10487465. Found {} figures across {} sections",
        total_figures,
        full_text.sections.len()
    );

    // PMC10487465 should have 8 figures based on our earlier analysis
    assert_eq!(
        total_figures, 8,
        "PMC10487465 should have exactly 8 figures"
    );

    // All figures should have file_name extracted from xlink:href
    assert_eq!(
        figures_with_file_name, total_figures,
        "All figures should have file_name extracted from graphic elements"
    );

    // All figures should follow the ijms-24-13282-g pattern
    assert_eq!(
        figures_with_proper_href, total_figures,
        "All figures should have proper xlink:href format (ijms-24-13282-g00X)"
    );

    // Verify specific figure IDs and file names
    let all_figures: Vec<_> = full_text.sections.iter().flat_map(|s| &s.figures).collect();

    // Check first figure
    let first_figure = all_figures
        .first()
        .expect("Should have at least one figure");
    assert_eq!(first_figure.id, "ijms-24-13282-f001");
    assert_eq!(
        first_figure.file_name,
        Some("ijms-24-13282-g001".to_string())
    );

    // Check last figure
    let last_figure = all_figures.last().expect("Should have at least one figure");
    assert_eq!(last_figure.id, "ijms-24-13282-f008");
    assert_eq!(
        last_figure.file_name,
        Some("ijms-24-13282-g008".to_string())
    );

    // Verify that each figure has meaningful content
    for figure in &all_figures {
        assert!(!figure.id.is_empty(), "Figure ID should not be empty");
        assert!(
            !figure.caption.is_empty(),
            "Figure caption should not be empty"
        );
        assert!(
            figure.caption != "No caption available",
            "Figure should have real caption"
        );

        // Verify label format
        if let Some(ref label) = figure.label {
            assert!(
                label.starts_with("Figure"),
                "Label should start with 'Figure'"
            );
        }
    }

    println!("✅ PMC10487465 NXML parsing test completed successfully!");
    println!("   - Total figures: {}", total_figures);
    println!("   - Figures with file_name: {}", figures_with_file_name);
    println!(
        "   - Figures with proper href: {}",
        figures_with_proper_href
    );
}

/// Test that verifies the enhanced graphic href extraction specifically
#[test]
fn test_enhanced_graphic_href_extraction() {
    // Simple XML fragment that mimics PMC10487465 structure
    let test_xml = r#"
    <article xmlns:xlink="http://www.w3.org/1999/xlink">
        <body>
            <sec>
                <fig id="test-fig-001">
                    <label>Figure 1</label>
                    <caption><p>Test figure caption</p></caption>
                    <graphic xlink:href="test-graphic-001" position="float"/>
                </fig>
                <fig id="test-fig-002">
                    <label>Figure 2</label>
                    <caption><p>Another test figure</p></caption>
                    <graphic xlink:href="test-graphic-002"/>
                </fig>
            </sec>
        </body>
    </article>
    "#;

    let result = PmcXmlParser::parse(test_xml, "TEST001");
    assert!(
        result.is_ok(),
        "Failed to parse test XML: {:?}",
        result.err()
    );

    let full_text = result.unwrap();
    let figures: Vec<_> = full_text.sections.iter().flat_map(|s| &s.figures).collect();

    assert_eq!(figures.len(), 2, "Should extract 2 figures from test XML");

    // Verify first figure
    assert_eq!(figures[0].id, "test-fig-001");
    assert_eq!(figures[0].file_name, Some("test-graphic-001".to_string()));
    assert_eq!(figures[0].label, Some("Figure 1".to_string()));

    // Verify second figure
    assert_eq!(figures[1].id, "test-fig-002");
    assert_eq!(figures[1].file_name, Some("test-graphic-002".to_string()));
    assert_eq!(figures[1].label, Some("Figure 2".to_string()));

    println!("✅ Enhanced graphic href extraction test passed!");
}

/// Test comparison between different PMC articles to verify parser robustness
#[test]
fn test_nxml_parser_robustness() {
    let test_files = vec![
        (
            "tests/integration/test_data/pmc_xml/PMC10487465.xml",
            "PMC10487465",
            8, // Expected figure count
        ),
        (
            "../assets/test_data/pmc_articles/PMC1064083/PMC1064083/bcr936.nxml",
            "PMC1064083",
            0, // Expected figure count (this PMC has figures without xlink:href - future enhancement)
        ),
        (
            "../assets/test_data/pmc_articles/PMC1175969/PMC1175969/gb-2005-6-6-r49.nxml",
            "PMC1175969",
            0, // Expected figure count (this PMC has figure references but no <fig> elements)
        ),
    ];

    for (file_path, pmcid, expected_figures) in test_files {
        if !std::path::Path::new(file_path).exists() {
            println!("Skipping robustness test: {} not found", file_path);
            continue;
        }

        println!("Testing parser robustness with {}", pmcid);

        let xml_content = fs::read_to_string(file_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", file_path));

        let result = PmcXmlParser::parse(&xml_content, pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse {}: {:?}",
            pmcid,
            result.err()
        );

        let full_text = result.unwrap();
        let total_figures = full_text
            .sections
            .iter()
            .map(|s| s.figures.len())
            .sum::<usize>();

        assert_eq!(
            total_figures, expected_figures,
            "{} should have {} figures, found {}",
            pmcid, expected_figures, total_figures
        );

        // Log figure information for debugging
        for section in &full_text.sections {
            for figure in &section.figures {
                println!(
                    "  {} - Figure: ID={}, file_name={:?}",
                    pmcid, figure.id, figure.file_name
                );
            }
        }
    }
}

/// Test that validates the figure matching would work with actual files
#[test]
fn test_figure_file_matching_simulation() {
    let test_file_path = "tests/integration/test_data/pmc_xml/PMC10487465.xml";

    if !std::path::Path::new(test_file_path).exists() {
        println!("Skipping file matching test: {} not found", test_file_path);
        return;
    }

    let xml_content = fs::read_to_string(test_file_path).expect("Failed to read NXML file");

    let full_text = PmcXmlParser::parse(&xml_content, "PMC10487465").expect("Failed to parse NXML");

    // Simulate the actual files that exist in the directory
    let simulated_files = vec![
        "/tmp/PMC10487465/ijms-24-13282-g001.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g001.gif".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g002.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g002.gif".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g003.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g003.gif".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g004.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g004.gif".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g005.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g005.gif".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g006.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g006.gif".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g007.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g007.gif".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g008.jpg".to_string(),
        "/tmp/PMC10487465/ijms-24-13282-g008.gif".to_string(),
    ];

    let image_extensions = [
        "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
    ];

    // Test that each figure would match with the correct files
    let figures: Vec<_> = full_text.sections.iter().flat_map(|s| &s.figures).collect();

    for figure in &figures {
        println!(
            "Testing matching for figure: ID={}, file_name={:?}",
            figure.id, figure.file_name
        );

        // Use the actual find_matching_file function
        let matched_file =
            PmcTarClient::find_matching_file(figure, &simulated_files, &image_extensions);

        assert!(
            matched_file.is_some(),
            "Figure {} with file_name {:?} should match a file",
            figure.id,
            figure.file_name
        );

        let matched_path = matched_file.unwrap();
        println!("  Matched file: {}", matched_path);

        // Verify the match makes sense
        if let Some(ref file_name) = figure.file_name {
            assert!(
                matched_path.contains(file_name),
                "Matched file should contain the figure file_name"
            );
        }
    }

    println!("✅ All figures matched successfully with simulated files!");
}
