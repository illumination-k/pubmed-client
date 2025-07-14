use pubmed_client_rs::pmc::tar::PmcTarClient;
use pubmed_client_rs::ClientConfig;
use std::path::Path;
use tracing::info;
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn test_extract_figures_uses_nxml_from_tar() {
    let config = ClientConfig::new();
    let client = PmcTarClient::new(config);

    // Test with a specific PMC ID
    let pmcid = "PMC9680858";
    let output_dir = Path::new("./test_extracted_figures_integration");

    info!("Extracting figures with captions for {}...", pmcid);

    // This should now use the NXML from the tar file instead of making an API call
    let result = client.extract_figures_with_captions(pmcid, output_dir).await;

    assert!(result.is_ok(), "Failed to extract figures: {:?}", result.err());

    let figures = result.unwrap();
    assert!(!figures.is_empty(), "No figures found");

    info!("Successfully extracted {} figures", figures.len());

    // Verify figures have expected data
    for figure in &figures {
        assert!(!figure.figure.id.is_empty(), "Figure ID is empty");
        assert!(!figure.figure.caption.is_empty(), "Figure caption is empty");
        assert!(!figure.extracted_file_path.is_empty(), "Extracted file path is empty");

        // Verify the file actually exists
        let path = Path::new(&figure.extracted_file_path);
        assert!(path.exists(), "Extracted file does not exist: {}", figure.extracted_file_path);
    }

    // Clean up
    if output_dir.exists() {
        std::fs::remove_dir_all(output_dir).ok();
    }
}
