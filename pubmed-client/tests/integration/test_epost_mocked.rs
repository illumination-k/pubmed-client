//! Integration tests for EPost API using mocked HTTP responses
//!
//! These tests verify the EPost functionality without making real API calls.
//! They use wiremock to simulate NCBI EPost responses.

use pubmed_client::{ClientConfig, PubMedClient};
use tracing_test::traced_test;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a client pointing at a mock server
fn create_mock_client(mock_server: &MockServer) -> PubMedClient {
    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0); // High rate limit for tests

    PubMedClient::with_config(config)
}

/// Test successful EPost with valid PMIDs
#[tokio::test]
#[traced_test]
async fn test_epost_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "epostresult": {
                        "webenv": "MCID_67890abcdef",
                        "querykey": "1"
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client
        .epost(&["31978945", "33515491", "25760099"])
        .await
        .expect("EPost should succeed");

    assert_eq!(result.webenv, "MCID_67890abcdef");
    assert_eq!(result.query_key, "1");

    // Verify history_session() conversion
    let session = result.history_session();
    assert_eq!(session.webenv, "MCID_67890abcdef");
    assert_eq!(session.query_key, "1");
}

/// Test EPost with a single PMID
#[tokio::test]
#[traced_test]
async fn test_epost_single_pmid() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "epostresult": {
                        "webenv": "MCID_single123",
                        "querykey": "1"
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client
        .epost(&["31978945"])
        .await
        .expect("EPost with single PMID should succeed");

    assert_eq!(result.webenv, "MCID_single123");
    assert_eq!(result.query_key, "1");
}

/// Test EPost followed by fetch_from_history
#[tokio::test]
#[traced_test]
async fn test_epost_then_fetch_from_history() {
    let mock_server = MockServer::start().await;

    // EPost mock
    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "epostresult": {
                        "webenv": "MCID_test_session",
                        "querykey": "1"
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // EFetch mock for history-based fetch
    let efetch_xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">31978945</PMID>
            <Article>
                <Journal><Title>Nature</Title></Journal>
                <ArticleTitle>Test Article</ArticleTitle>
                <AuthorList>
                    <Author>
                        <LastName>Test</LastName>
                        <ForeName>Author</ForeName>
                    </Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Journal Article</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
    </PubmedArticle>
</PubmedArticleSet>"#;

    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(efetch_xml)
                .insert_header("content-type", "application/xml"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    // Upload PMIDs via EPost
    let epost_result = client
        .epost(&["31978945"])
        .await
        .expect("EPost should succeed");

    // Use session to fetch articles
    let session = epost_result.history_session();
    let articles = client
        .fetch_from_history(&session, 0, 10)
        .await
        .expect("Fetch from history should succeed");

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].pmid, "31978945");
    assert_eq!(articles[0].title, "Test Article");
}

/// Test EPost to existing session
#[tokio::test]
#[traced_test]
async fn test_epost_to_session() {
    let mock_server = MockServer::start().await;

    // First EPost creates the session
    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "epostresult": {
                        "webenv": "MCID_shared_session",
                        "querykey": "2"
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    // First upload
    let result1 = client
        .epost(&["31978945"])
        .await
        .expect("First EPost should succeed");

    // Append to existing session
    let result2 = client
        .epost_to_session(&["33515491"], &result1.history_session())
        .await
        .expect("EPost to session should succeed");

    // WebEnv should be the shared session
    assert_eq!(result2.webenv, "MCID_shared_session");
    assert_eq!(result2.query_key, "2");
}

/// Test EPost with empty input returns error
#[tokio::test]
#[traced_test]
async fn test_epost_empty_input() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let result = client.epost(&[]).await;
    assert!(result.is_err(), "Empty PMID list should fail");

    // Verify no requests were made
    let received_requests = mock_server.received_requests().await.unwrap();
    assert_eq!(received_requests.len(), 0);
}

/// Test EPost with invalid PMID
#[tokio::test]
#[traced_test]
async fn test_epost_invalid_pmid() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let result = client.epost(&["not_a_number"]).await;
    assert!(result.is_err(), "Invalid PMID should fail");

    // Verify no requests were made
    let received_requests = mock_server.received_requests().await.unwrap();
    assert_eq!(received_requests.len(), 0);
}

/// Test EPost with mixed valid/invalid PMIDs
#[tokio::test]
#[traced_test]
async fn test_epost_mixed_valid_invalid_pmids() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let result = client.epost(&["31978945", "invalid", "25760099"]).await;
    assert!(result.is_err(), "Mixed valid/invalid PMIDs should fail");

    // No requests should be made if validation fails
    let received_requests = mock_server.received_requests().await.unwrap();
    assert_eq!(received_requests.len(), 0);
}

/// Test EPost with API error response
#[tokio::test]
#[traced_test]
async fn test_epost_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "epostresult": {
                        "ERROR": "Invalid db name specified"
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client.epost(&["31978945"]).await;
    assert!(result.is_err(), "API error should propagate");
}

/// Test EPost with server error (500)
#[tokio::test]
#[traced_test]
async fn test_epost_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client.epost(&["31978945"]).await;
    assert!(result.is_err(), "Server error should propagate");
}

/// Test EPost with missing webenv in response
#[tokio::test]
#[traced_test]
async fn test_epost_missing_webenv() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "epostresult": {
                        "querykey": "1"
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client.epost(&["31978945"]).await;
    assert!(
        result.is_err(),
        "Missing webenv should return WebEnvNotAvailable error"
    );
}

/// Test EPost with rate limited response (429)
#[tokio::test]
#[traced_test]
async fn test_epost_rate_limited() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Too Many Requests")
                .insert_header("retry-after", "1"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client.epost(&["31978945"]).await;
    assert!(result.is_err(), "429 response should result in error");
}

/// Test EPostResult model
#[test]
fn test_epost_result_model() {
    use pubmed_client::EPostResult;

    let result = EPostResult {
        webenv: "MCID_test123".to_string(),
        query_key: "1".to_string(),
    };

    let session = result.history_session();
    assert_eq!(session.webenv, "MCID_test123");
    assert_eq!(session.query_key, "1");
}

// ================================================================================================
// fetch_all_by_pmids tests
// ================================================================================================

const EFETCH_3_ARTICLES: &str = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">31978945</PMID>
            <Article>
                <Journal><Title>Nature</Title></Journal>
                <ArticleTitle>Article One</ArticleTitle>
                <AuthorList>
                    <Author><LastName>Wu</LastName><ForeName>Fan</ForeName></Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Journal Article</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
    </PubmedArticle>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">33515491</PMID>
            <Article>
                <Journal><Title>Lancet</Title></Journal>
                <ArticleTitle>Article Two</ArticleTitle>
                <AuthorList>
                    <Author><LastName>Smith</LastName><ForeName>John</ForeName></Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Review</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
    </PubmedArticle>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">25760099</PMID>
            <Article>
                <Journal><Title>Science</Title></Journal>
                <ArticleTitle>Article Three</ArticleTitle>
                <AuthorList>
                    <Author><LastName>Doudna</LastName><ForeName>Jennifer</ForeName></Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Journal Article</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
    </PubmedArticle>
</PubmedArticleSet>"#;

/// Helper to set up both EPost and EFetch mocks for fetch_all_by_pmids tests
async fn setup_epost_and_efetch_mocks(mock_server: &MockServer, efetch_xml: &str) {
    // EPost mock
    Mock::given(method("POST"))
        .and(path_regex(r"/epost\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "epostresult": {
                        "webenv": "MCID_fetch_all_test",
                        "querykey": "1"
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .mount(mock_server)
        .await;

    // EFetch from history mock
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(efetch_xml.to_string())
                .insert_header("content-type", "application/xml"),
        )
        .mount(mock_server)
        .await;
}

/// Test fetch_all_by_pmids with multiple PMIDs
#[tokio::test]
#[traced_test]
async fn test_fetch_all_by_pmids_success() {
    let mock_server = MockServer::start().await;
    setup_epost_and_efetch_mocks(&mock_server, EFETCH_3_ARTICLES).await;

    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_all_by_pmids(&["31978945", "33515491", "25760099"])
        .await
        .expect("fetch_all_by_pmids should succeed");

    assert_eq!(articles.len(), 3);
    assert!(articles.iter().any(|a| a.pmid == "31978945"));
    assert!(articles.iter().any(|a| a.pmid == "33515491"));
    assert!(articles.iter().any(|a| a.pmid == "25760099"));
}

/// Test fetch_all_by_pmids with empty input
#[tokio::test]
#[traced_test]
async fn test_fetch_all_by_pmids_empty() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_all_by_pmids(&[])
        .await
        .expect("Empty input should return Ok");

    assert!(articles.is_empty());

    // No requests should have been made
    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(received.len(), 0);
}

/// Test fetch_all_by_pmids rejects invalid PMIDs
#[tokio::test]
#[traced_test]
async fn test_fetch_all_by_pmids_invalid_pmid() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let result = client
        .fetch_all_by_pmids(&["31978945", "invalid"])
        .await;

    assert!(result.is_err());

    // No requests should have been made
    let received = mock_server.received_requests().await.unwrap();
    assert_eq!(received.len(), 0);
}

/// Test fetch_all_by_pmids with single PMID
#[tokio::test]
#[traced_test]
async fn test_fetch_all_by_pmids_single() {
    let single_xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">31978945</PMID>
            <Article>
                <Journal><Title>Nature</Title></Journal>
                <ArticleTitle>Single Article</ArticleTitle>
                <AuthorList>
                    <Author><LastName>Wu</LastName><ForeName>Fan</ForeName></Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Journal Article</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
    </PubmedArticle>
</PubmedArticleSet>"#;

    let mock_server = MockServer::start().await;
    setup_epost_and_efetch_mocks(&mock_server, single_xml).await;

    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_all_by_pmids(&["31978945"])
        .await
        .expect("Single PMID should succeed");

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].pmid, "31978945");
    assert_eq!(articles[0].title, "Single Article");
}
