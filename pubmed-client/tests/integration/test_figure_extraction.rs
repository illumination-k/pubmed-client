use pubmed_client::{ClientConfig, PmcClient, PmcTarClient, PubMedError};
use std::path::Path;
use tempfile::tempdir;
use tracing::info;
use tracing_test::traced_test;

/// Macro to test figure extraction from PMCID
///
/// This macro takes a PMCID as input and:
/// 1. Checks if a test XML file exists in tests/integration/test_data/pmc_xml/
/// 2. If found, uses the local file; otherwise downloads and saves from API
/// 3. Parses the XML to extract figure metadata
/// 4. Asserts that at least one figure is found
macro_rules! test_pmcid_figure_extraction {
    ($pmcid:expr) => {
        paste::paste! {
            #[cfg(not(target_arch = "wasm32"))]
            #[traced_test]
            #[tokio::test]
            async fn [<test_figure_extraction_ $pmcid:lower>]() {
                let pmcid = $pmcid;
                info!(pmcid = %pmcid, "Testing figure extraction");

                // Construct path to test data file
                let test_data_path = format!(
                    "tests/integration/test_data/pmc_xml/PMC{}.xml",
                    pmcid.trim_start_matches("PMC")
                );

                let xml_content = if Path::new(&test_data_path).exists() {
                    info!(pmcid = %pmcid, path = %test_data_path, "Using local test data file");
                    std::fs::read_to_string(&test_data_path)
                        .unwrap_or_else(|e| panic!("Failed to read test data file {}: {}", test_data_path, e))
                } else {
                    info!(pmcid = %pmcid, "Local test data not found, downloading from API");
                    let config = ClientConfig::new();
                    let client = PmcClient::with_config(config);
                    let raw_xml = client.fetch_xml(pmcid).await
                        .unwrap_or_else(|e| panic!("Failed to download XML for {}: {}", pmcid, e));
                    std::fs::write(&test_data_path, &raw_xml).unwrap_or_else(|e| panic!("Failed to save test data file {}: {}", test_data_path, e));
                    raw_xml
                };

                // Parse XML to extract figures
                let normalized_pmcid = if pmcid.starts_with("PMC") {
                    pmcid.to_string()
                } else {
                    format!("PMC{}", pmcid)
                };

                let pmc_full_text = pubmed_client::pmc::parse_pmc_xml(&xml_content, &normalized_pmcid)
                    .unwrap_or_else(|e| panic!("Failed to parse XML for {}: {}", pmcid, e));

                // Collect all figures from all sections
                let mut all_figures = Vec::new();

                fn collect_figures_from_section(section: &pubmed_client::ArticleSection, figures: &mut Vec<pubmed_client::Figure>) {
                    figures.extend(section.figures.clone());
                    for subsection in &section.subsections {
                        collect_figures_from_section(subsection, figures);
                    }
                }

                for section in &pmc_full_text.sections {
                    collect_figures_from_section(section, &mut all_figures);
                }

                let figures_count = all_figures.len();
                info!(pmcid = %pmcid, figures_count = figures_count, "Figure extraction completed");

                // Log detailed section information for debugging
                info!(pmcid = %pmcid, total_sections = pmc_full_text.sections.len(), "Section summary");
                for (section_idx, section) in pmc_full_text.sections.iter().enumerate() {
                    fn log_section_figures(section: &pubmed_client::ArticleSection, pmcid: &str, section_path: String, level: usize) {
                        let indent = "  ".repeat(level);
                        info!(
                            pmcid = %pmcid,
                            section_path = %section_path,
                            section_type = %section.section_type,
                            section_title = ?section.title,
                            figures_count = section.figures.len(),
                            tables_count = section.tables.len(),
                            content_length = section.content.len(),
                            subsections_count = section.subsections.len(),
                            "{}Section details", indent
                        );

                        for (fig_idx, figure) in section.figures.iter().enumerate() {
                            info!(
                                pmcid = %pmcid,
                                section_path = %section_path,
                                figure_index = fig_idx,
                                figure_id = %figure.id,
                                figure_label = ?figure.label,
                                caption_length = figure.caption.len(),
                                fig_type = ?figure.fig_type,
                                file_name = ?figure.file_name,
                                "{}  Figure in section", indent
                            );
                        }

                        for (sub_idx, subsection) in section.subsections.iter().enumerate() {
                            let sub_path = format!("{}.{}", section_path, sub_idx);
                            log_section_figures(subsection, pmcid, sub_path, level + 1);
                        }
                    }

                    log_section_figures(section, pmcid, format!("section_{}", section_idx), 0);
                }

                // If no figures found, provide detailed debugging information
                if figures_count == 0 {
                    info!(pmcid = %pmcid, "No figures found - debugging information:");
                    info!(pmcid = %pmcid, title = %pmc_full_text.title, "Article title");
                    info!(pmcid = %pmcid, pmid = ?pmc_full_text.pmid, "Associated PMID");
                    info!(pmcid = %pmcid, article_type = ?pmc_full_text.article_type, "Article type");

                    // Check if there are supplementary materials that might contain figures
                    if !pmc_full_text.supplementary_materials.is_empty() {
                        info!(pmcid = %pmcid, supp_materials_count = pmc_full_text.supplementary_materials.len(), "Supplementary materials found");
                        for (i, supp) in pmc_full_text.supplementary_materials.iter().enumerate() {
                            info!(
                                pmcid = %pmcid,
                                supp_index = i,
                                supp_id = %supp.id,
                                supp_title = ?supp.title,
                                content_type = ?supp.content_type,
                                file_url = ?supp.file_url,
                                file_type = ?supp.file_type,
                                "Supplementary material"
                            );
                        }
                    }

                    // Sample some content to see what's in the sections
                    for (i, section) in pmc_full_text.sections.iter().take(3).enumerate() {
                        let content_sample = if section.content.len() > 100 {
                            format!("{}...", &section.content[..100])
                        } else {
                            section.content.clone()
                        };
                        info!(
                            pmcid = %pmcid,
                            section_index = i,
                            section_type = %section.section_type,
                            content_sample = %content_sample,
                            "Section content sample"
                        );
                    }
                }

                assert!(
                    figures_count > 0,
                    "Expected at least one figure for PMCID {}, but found {}. Check the info logs above for detailed debugging information.",
                    pmcid,
                    figures_count
                );

                // Log figure details for successful cases
                for (i, figure) in all_figures.iter().enumerate() {
                    info!(
                        pmcid = %pmcid,
                        figure_index = i,
                        figure_id = %figure.id,
                        figure_label = ?figure.label,
                        caption_length = figure.caption.len(),
                        fig_type = ?figure.fig_type,
                        file_name = ?figure.file_name,
                        "Successfully extracted figure"
                    );
                }
            }
        }
    };
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_extract_figures_with_captions_invalid_pmcid() {
    let config = ClientConfig::new();
    let client = PmcTarClient::new(config);
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test with invalid PMCID
    let result = client
        .extract_figures_with_captions("invalid_pmcid", temp_dir.path())
        .await;

    assert!(result.is_err());
    if let Err(PubMedError::InvalidPmid { pmid }) = result {
        assert_eq!(pmid, "PMCinvalid_pmcid");
    } else {
        panic!("Expected InvalidPmid error, got: {:?}", result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_extract_figures_with_captions_empty_pmcid() {
    let config = ClientConfig::new();
    let client = PmcTarClient::new(config);
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test with empty PMCID
    let result = client
        .extract_figures_with_captions("", temp_dir.path())
        .await;

    assert!(result.is_err());
    if let Err(PubMedError::InvalidPmid { pmid }) = result {
        assert_eq!(pmid, "PMC");
    } else {
        panic!("Expected InvalidPmid error, got: {:?}", result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_extract_figures_with_captions_directory_creation() {
    let config = ClientConfig::new();
    let client = PmcTarClient::new(config);
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let nested_path = temp_dir.path().join("figures").join("extracted");

    // Test with a PMCID that likely won't be available in OA
    let result = client
        .extract_figures_with_captions("PMC1234567", &nested_path)
        .await;

    // Check that the directory was created
    assert!(nested_path.exists());

    // Should fail with error but directory creation should succeed
    assert!(result.is_err());
    match result.unwrap_err() {
        PubMedError::PmcNotAvailableById { pmcid } => {
            assert_eq!(pmcid, "PMC1234567");
        }
        PubMedError::ApiError { status, .. } => {
            assert!(status == 404 || status >= 400);
        }
        PubMedError::IoError { .. } => {
            // Could fail with IO error if the response isn't valid
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_figure_matching_functions() {
    let config = ClientConfig::new();
    let _client = PmcTarClient::new(config);

    // Test the internal matching logic with mock data
    let figure_id = "fig1".to_string();
    let figure_label = Some("Figure 1".to_string());

    let extracted_files = vec![
        "/path/to/fig1.jpg".to_string(),
        "/path/to/table1.png".to_string(),
        "/path/to/figure1.pdf".to_string(),
        "/path/to/other.txt".to_string(),
    ];

    let image_extensions = [
        "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
    ];

    // Create a mock figure
    let figure = pubmed_client::Figure {
        id: figure_id,
        label: figure_label,
        caption: "Test figure caption".to_string(),
        alt_text: None,
        fig_type: None,
        file_path: None,
        file_name: None,
    };

    // Test figure ID matching
    let matching_file =
        PmcTarClient::find_matching_file(&figure, &extracted_files, &image_extensions);
    assert!(matching_file.is_some());
    assert_eq!(matching_file.unwrap(), "/path/to/fig1.jpg");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_figure_matching_by_label() {
    let config = ClientConfig::new();
    let _client = PmcTarClient::new(config);

    let extracted_files = vec![
        "/path/to/some_figure1.jpg".to_string(),
        "/path/to/table_data.png".to_string(),
    ];

    let image_extensions = [
        "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
    ];

    // Create a figure with a label that should match
    let figure = pubmed_client::Figure {
        id: "unknown".to_string(),
        label: Some("Figure 1".to_string()),
        caption: "Test figure caption".to_string(),
        alt_text: None,
        fig_type: None,
        file_path: None,
        file_name: None,
    };

    let matching_file =
        PmcTarClient::find_matching_file(&figure, &extracted_files, &image_extensions);
    assert!(matching_file.is_some());
    assert_eq!(matching_file.unwrap(), "/path/to/some_figure1.jpg");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_figure_matching_by_filename() {
    let config = ClientConfig::new();
    let _client = PmcTarClient::new(config);

    let extracted_files = vec![
        "/path/to/graph_data.jpg".to_string(),
        "/path/to/specific_file.png".to_string(),
    ];

    let image_extensions = [
        "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
    ];

    // Create a figure with a specific filename
    let figure = pubmed_client::Figure {
        id: "fig_unknown".to_string(),
        label: None,
        caption: "Test figure caption".to_string(),
        alt_text: None,
        fig_type: None,
        file_path: None,
        file_name: Some("specific_file".to_string()),
    };

    let matching_file =
        PmcTarClient::find_matching_file(&figure, &extracted_files, &image_extensions);
    assert!(matching_file.is_some());
    assert_eq!(matching_file.unwrap(), "/path/to/specific_file.png");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_figure_no_match() {
    let config = ClientConfig::new();
    let _client = PmcTarClient::new(config);

    let extracted_files = vec![
        "/path/to/table1.csv".to_string(),
        "/path/to/data.txt".to_string(),
    ];

    let image_extensions = [
        "jpg", "jpeg", "png", "gif", "tiff", "tif", "svg", "eps", "pdf",
    ];

    // Create a figure that won't match any files
    let figure = pubmed_client::Figure {
        id: "nonexistent".to_string(),
        label: Some("Nonexistent Figure".to_string()),
        caption: "Test figure caption".to_string(),
        alt_text: None,
        fig_type: None,
        file_path: None,
        file_name: None,
    };

    let matching_file =
        PmcTarClient::find_matching_file(&figure, &extracted_files, &image_extensions);
    assert!(matching_file.is_none());
}

// Note: We don't test actual successful figure extraction in the regular test suite
// to avoid making real network requests and potentially overwhelming the NCBI servers.
// Real API tests would be run separately with the PUBMED_REAL_API_TESTS environment variable.

// Example usage of the figure extraction macro with test data
// These tests use local test data files when available, or download from API when needed

test_pmcid_figure_extraction!("PMC7906746");
test_pmcid_figure_extraction!("PMC6000000");
test_pmcid_figure_extraction!("PMC10455298");
test_pmcid_figure_extraction!("PMC11084381");
