use pubmed_client_rs::{ClientConfig, PmcTarClient, PubMedError};
use tempfile::tempdir;

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
    let figure = pubmed_client_rs::Figure {
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
    let figure = pubmed_client_rs::Figure {
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
    let figure = pubmed_client_rs::Figure {
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
    let figure = pubmed_client_rs::Figure {
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
