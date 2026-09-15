use pubmed_client::{ClientConfig, ParseError, PmcClient, PubMedError};
use tempfile::tempdir;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_download_files_invalid_pmcid() {
    let client = PmcClient::new();
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test with invalid PMCID
    let result = client
        .download_files("invalid_pmcid", temp_dir.path())
        .await;

    assert!(result.is_err());
    if let Err(PubMedError::ParseError(ParseError::InvalidPmcid { pmcid })) = result {
        assert_eq!(pmcid, "invalid_pmcid");
    } else {
        panic!("Expected InvalidPmcid error, got: {:?}", result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_download_files_empty_pmcid() {
    let client = PmcClient::new();
    let temp_dir = tempdir().expect("Failed to create temp dir");

    // Test with empty PMCID
    let result = client.download_files("", temp_dir.path()).await;

    assert!(result.is_err());
    if let Err(PubMedError::ParseError(ParseError::InvalidPmcid { pmcid })) = result {
        assert_eq!(pmcid, "");
    } else {
        panic!("Expected InvalidPmcid error, got: {:?}", result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_download_files_directory_creation() {
    let client = PmcClient::new();
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let nested_path = temp_dir.path().join("nested").join("directory");

    // Test with a PMCID that likely won't be available in OA
    // This should fail with PmcNotAvailable, but only after creating the directory
    let result = client.download_files("PMC1234567", &nested_path).await;

    // Check that the directory was created
    assert!(nested_path.exists());

    // Should fail with not available error
    assert!(result.is_err());
    match result.unwrap_err() {
        PubMedError::ParseError(ParseError::PmcNotAvailable { id }) => {
            assert_eq!(id, "PMC1234567");
        }
        PubMedError::ApiError { status, .. } => {
            // Could also be a 404 or similar API error
            assert!(status == 404 || status >= 400);
        }
        PubMedError::ParseError(ParseError::IoError { .. }) => {
            // Could fail with IO error if the cloud response isn't usable
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
    let result1 = client.download_files("1234567", temp_dir.path()).await;
    let result2 = client.download_files("PMC1234567", temp_dir.path()).await;

    // Both should fail with the same error type
    assert!(result1.is_err());
    assert!(result2.is_err());

    // The errors should be similar (both should reference PMC1234567)
    match (result1.unwrap_err(), result2.unwrap_err()) {
        (
            PubMedError::ParseError(ParseError::PmcNotAvailable { id: id1 }),
            PubMedError::ParseError(ParseError::PmcNotAvailable { id: id2 }),
        ) => {
            assert_eq!(id1, "1234567");
            assert_eq!(id2, "PMC1234567");
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

// --- In-memory figure fetching (`fetch_figures`), against a stubbed bucket ---

/// A 4x3 PNG, so `imagesize` has a real header to read dimensions from.
#[cfg(not(target_arch = "wasm32"))]
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x03, 0x08, 0x02, 0x00, 0x00, 0x00, 0x3b, 0x96, 0x39,
    0x91, 0x00, 0x00, 0x00, 0x10, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xc0, 0x00,
    0x47, 0x0c, 0x38, 0x39, 0x00, 0xf5, 0x31, 0x0b, 0xf5, 0x35, 0x7b, 0xfb, 0x82, 0x00, 0x00, 0x00,
    0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

#[cfg(not(target_arch = "wasm32"))]
const ARTICLE_XML: &str = r#"<?xml version="1.0"?>
<article xmlns:xlink="http://www.w3.org/1999/xlink">
  <front>
    <article-meta>
      <title-group><article-title>Figures over the wire</article-title></title-group>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Results</title>
      <fig id="fig1">
        <label>Figure 1</label>
        <caption><p>The first figure.</p></caption>
        <graphic xlink:href="gr1_lrg"/>
      </fig>
      <fig id="fig2">
        <label>Figure 2</label>
        <caption><p>The second figure.</p></caption>
        <graphic xlink:href="gr2_lrg"/>
      </fig>
    </sec>
  </body>
</article>
"#;

/// Stub the OA Cloud bucket: one listing, the JATS XML, and two figure images.
///
/// `supplement.pdf` is in the listing but referenced by no `<fig>`; no mock
/// serves it, so a request for it would fail the test outright — which is the
/// point, `fetch_figures` must not download the whole package.
#[cfg(not(target_arch = "wasm32"))]
async fn mock_oa_cloud() -> wiremock::MockServer {
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;

    let listing = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
<Contents><Key>PMC7906746.1/PMC7906746.1.xml</Key><Size>1</Size></Contents>
<Contents><Key>PMC7906746.1/gr1_lrg.png</Key><Size>1</Size></Contents>
<Contents><Key>PMC7906746.1/gr2_lrg.png</Key><Size>1</Size></Contents>
<Contents><Key>PMC7906746.1/supplement.pdf</Key><Size>1</Size></Contents>
</ListBucketResult>"#;

    Mock::given(method("GET"))
        .and(path("/"))
        .and(query_param("prefix", "PMC7906746."))
        .respond_with(ResponseTemplate::new(200).set_body_string(listing))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/PMC7906746.1/PMC7906746.1.xml"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ARTICLE_XML))
        .mount(&server)
        .await;

    for name in ["gr1_lrg.png", "gr2_lrg.png"] {
        Mock::given(method("GET"))
            .and(path(format!("/PMC7906746.1/{name}")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(TINY_PNG))
            .mount(&server)
            .await;
    }

    server
}

#[cfg(not(target_arch = "wasm32"))]
fn cloud_client(server: &wiremock::MockServer) -> PmcClient {
    PmcClient::with_config(ClientConfig::new().with_oa_cloud_base_url(server.uri()))
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn fetch_figures_returns_bytes_metadata_and_dimensions() {
    let server = mock_oa_cloud().await;

    let blobs = cloud_client(&server)
        .fetch_figures("PMC7906746")
        .await
        .expect("the stubbed article should yield figures");

    assert_eq!(blobs.len(), 2, "both referenced figures should come back");

    let first = &blobs[0];
    assert_eq!(first.figure.id, "fig1");
    assert_eq!(first.figure.label.as_deref(), Some("Figure 1"));
    assert_eq!(first.file_name, "gr1_lrg.png");
    assert_eq!(first.content_type, "image/png");
    assert_eq!(first.data, TINY_PNG);
    assert_eq!(first.dimensions, Some((4, 3)));

    assert_eq!(blobs[1].figure.id, "fig2");
    assert_eq!(blobs[1].file_name, "gr2_lrg.png");
}

/// A figure request must fetch the article's XML and its images — not the PDF,
/// supplementary files, or anything else in the package.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn fetch_figures_downloads_only_the_referenced_images() {
    let server = mock_oa_cloud().await;

    cloud_client(&server)
        .fetch_figures("PMC7906746")
        .await
        .expect("the stubbed article should yield figures");

    let requested: Vec<String> = server
        .received_requests()
        .await
        .expect("wiremock records requests")
        .iter()
        .map(|request| request.url.path().to_string())
        .collect();

    assert!(
        !requested
            .iter()
            .any(|path| path.ends_with("supplement.pdf")),
        "unreferenced package files must not be downloaded, got: {requested:?}"
    );
    assert_eq!(
        requested.len(),
        4,
        "expected the listing, the XML and two images, got: {requested:?}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn a_figure_selection_matches_ids_and_labels_and_caps_the_downloads() {
    use pubmed_client::FigureSelection;

    let server = mock_oa_cloud().await;
    let client = cloud_client(&server);

    // A label the caller read in a caption selects the same figure as its id.
    let by_label = client
        .fetch_figures_with("PMC7906746", &FigureSelection::new().with_ids(["Figure 2"]))
        .await
        .expect("selecting by label should work");
    assert_eq!(by_label.len(), 1);
    assert_eq!(by_label[0].figure.id, "fig2");

    let by_id = client
        .fetch_figures_with("PMC7906746", &FigureSelection::new().with_ids(["fig1"]))
        .await
        .expect("selecting by id should work");
    assert_eq!(by_id.len(), 1);
    assert_eq!(by_id[0].figure.id, "fig1");

    // The limit is applied before downloading, so only one image is fetched.
    let before = server.received_requests().await.unwrap().len();
    let limited = client
        .fetch_figures_with("PMC7906746", &FigureSelection::new().with_limit(1))
        .await
        .expect("a limited selection should work");
    let after = server.received_requests().await.unwrap().len();

    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0].figure.id, "fig1");
    assert_eq!(
        after - before,
        3,
        "a limit of 1 should cost the listing, the XML and a single image"
    );
}

/// An id nobody recognizes yields nothing rather than silently falling back to
/// every figure.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn an_unknown_figure_id_selects_nothing() {
    use pubmed_client::FigureSelection;

    let server = mock_oa_cloud().await;

    let blobs = cloud_client(&server)
        .fetch_figures_with("PMC7906746", &FigureSelection::new().with_ids(["fig99"]))
        .await
        .expect("an unmatched selection is empty, not an error");

    assert!(blobs.is_empty(), "got: {blobs:?}");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn fetch_figures_rejects_an_invalid_pmcid_before_any_request() {
    let server = mock_oa_cloud().await;

    let err = cloud_client(&server)
        .fetch_figures("not-a-pmcid")
        .await
        .expect_err("an invalid PMCID must not reach the network");

    assert!(
        matches!(
            err,
            PubMedError::ParseError(ParseError::InvalidPmcid { .. })
        ),
        "got: {err:?}"
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn fetch_figures_reports_an_article_absent_from_the_bucket() {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><KeyCount>0</KeyCount></ListBucketResult>"#,
        ))
        .mount(&server)
        .await;

    let err = cloud_client(&server)
        .fetch_figures("PMC1234567")
        .await
        .expect_err("an empty listing means the article is not in the OA Cloud");

    assert!(
        matches!(
            err,
            PubMedError::ParseError(ParseError::PmcNotAvailable { .. })
        ),
        "got: {err:?}"
    );
}
