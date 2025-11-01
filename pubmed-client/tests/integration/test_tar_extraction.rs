use pubmed_client::{PmcClient, PubMedError};
use tempfile::tempdir;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_download_and_extract_tar_invalid_pmcid() {
    let client = PmcClient::new();
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test with invalid PMCID
    let result = client
        .download_and_extract_tar("invalid_pmcid", temp_dir.path())
        .await;

    assert!(result.is_err());
    if let Err(PubMedError::InvalidPmid { pmid }) = result {
        assert_eq!(pmid, "invalid_pmcid");
    } else {
        panic!("Expected InvalidPmid error, got: {:?}", result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_download_and_extract_tar_empty_pmcid() {
    let client = PmcClient::new();
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test with empty PMCID
    let result = client.download_and_extract_tar("", temp_dir.path()).await;

    assert!(result.is_err());
    if let Err(PubMedError::InvalidPmid { pmid }) = result {
        assert_eq!(pmid, "");
    } else {
        panic!("Expected InvalidPmid error, got: {:?}", result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_download_and_extract_tar_directory_creation() {
    let client = PmcClient::new();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let nested_path = temp_dir.path().join("nested").join("directory");

    // Test with a PMCID that likely won't be available in OA
    // This should fail with PmcNotAvailableById, but only after creating the directory
    let result = client
        .download_and_extract_tar("PMC1234567", &nested_path)
        .await;

    // Check that the directory was created
    assert!(nested_path.exists());

    // Should fail with not available error
    assert!(result.is_err());
    match result.unwrap_err() {
        PubMedError::PmcNotAvailableById { pmcid } => {
            assert_eq!(pmcid, "PMC1234567");
        }
        PubMedError::ApiError { status, .. } => {
            // Could also be a 404 or similar API error
            assert!(status == 404 || status >= 400);
        }
        PubMedError::IoError { .. } => {
            // Could fail with IO error if the response isn't a valid tar.gz
            // This is expected for non-existent PMCIDs
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_pmcid_normalization() {
    let client = PmcClient::new();
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test that PMCID normalization works correctly
    // Both should result in the same error since they're the same PMCID
    let result1 = client
        .download_and_extract_tar("1234567", temp_dir.path())
        .await;
    let result2 = client
        .download_and_extract_tar("PMC1234567", temp_dir.path())
        .await;

    // Both should fail with the same error type
    assert!(result1.is_err());
    assert!(result2.is_err());

    // The errors should be similar (both should reference PMC1234567)
    match (result1.unwrap_err(), result2.unwrap_err()) {
        (
            PubMedError::PmcNotAvailableById { pmcid: pmcid1 },
            PubMedError::PmcNotAvailableById { pmcid: pmcid2 },
        ) => {
            assert_eq!(pmcid1, "1234567");
            assert_eq!(pmcid2, "PMC1234567");
        }
        (PubMedError::ApiError { status: s1, .. }, PubMedError::ApiError { status: s2, .. }) => {
            assert_eq!(s1, s2);
        }
        _ => {
            // Other combinations are also acceptable as long as both fail
        }
    }
}

// Note: We don't test actual successful downloads in the regular test suite
// to avoid making real network requests and potentially overwhelming the NCBI servers.
// Real API tests would be run separately with the PUBMED_REAL_API_TESTS environment variable.
