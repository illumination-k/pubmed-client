//! Integration tests for batch fetch_articles using mocked HTTP responses
//!
//! These tests verify the batch fetching functionality without making real API calls.
//! They use wiremock to simulate NCBI EFetch responses.

use pubmed_client::{ClientConfig, PubMedClient};
use tracing_test::traced_test;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Multi-article XML response for batch fetch testing
const BATCH_EFETCH_RESPONSE_3_ARTICLES: &str = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">31978945</PMID>
            <Article>
                <Journal><Title>Nature</Title></Journal>
                <ArticleTitle>A pneumonia outbreak associated with a new coronavirus</ArticleTitle>
                <Abstract>
                    <AbstractText>In December 2019, a cluster of patients with pneumonia...</AbstractText>
                </Abstract>
                <AuthorList>
                    <Author>
                        <LastName>Wu</LastName>
                        <ForeName>Fan</ForeName>
                    </Author>
                    <Author>
                        <LastName>Zhao</LastName>
                        <ForeName>Su</ForeName>
                    </Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Journal Article</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
        <PubmedData>
            <ArticleIdList>
                <ArticleId IdType="pubmed">31978945</ArticleId>
                <ArticleId IdType="doi">10.1038/s41586-020-2008-3</ArticleId>
            </ArticleIdList>
        </PubmedData>
    </PubmedArticle>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">33515491</PMID>
            <Article>
                <Journal><Title>Lancet Oncology</Title></Journal>
                <ArticleTitle>Cancer treatment advances in 2020</ArticleTitle>
                <Abstract>
                    <AbstractText>Recent advances in cancer treatment have shown promise...</AbstractText>
                </Abstract>
                <AuthorList>
                    <Author>
                        <LastName>Smith</LastName>
                        <ForeName>John</ForeName>
                    </Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Review</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
        <PubmedData>
            <ArticleIdList>
                <ArticleId IdType="pubmed">33515491</ArticleId>
            </ArticleIdList>
        </PubmedData>
    </PubmedArticle>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">25760099</PMID>
            <Article>
                <Journal><Title>Science</Title></Journal>
                <ArticleTitle>CRISPR-Cas9 gene editing technology</ArticleTitle>
                <Abstract>
                    <AbstractText>The CRISPR-Cas9 system has revolutionized genome editing...</AbstractText>
                </Abstract>
                <AuthorList>
                    <Author>
                        <LastName>Doudna</LastName>
                        <ForeName>Jennifer</ForeName>
                    </Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Journal Article</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
        <PubmedData>
            <ArticleIdList>
                <ArticleId IdType="pubmed">25760099</ArticleId>
            </ArticleIdList>
        </PubmedData>
    </PubmedArticle>
</PubmedArticleSet>"#;

const SINGLE_ARTICLE_RESPONSE: &str = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">12345678</PMID>
            <Article>
                <Journal><Title>Test Journal</Title></Journal>
                <ArticleTitle>Single Test Article</ArticleTitle>
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

/// Helper to create a mock server with a batch efetch response
async fn setup_batch_efetch_mock(body: &str) -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body.to_string())
                .insert_header("content-type", "application/xml"),
        )
        .mount(&mock_server)
        .await;

    mock_server
}

/// Helper to create a client pointing at a mock server
fn create_mock_client(mock_server: &MockServer) -> PubMedClient {
    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0); // High rate limit for tests

    PubMedClient::with_config(config)
}

/// Test batch fetching multiple articles in a single request
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_multiple_articles() {
    let mock_server = setup_batch_efetch_mock(BATCH_EFETCH_RESPONSE_3_ARTICLES).await;
    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_articles(&["31978945", "33515491", "25760099"])
        .await
        .expect("Batch fetch should succeed");

    assert_eq!(articles.len(), 3, "Should return 3 articles");

    // Verify first article (COVID-19)
    let covid = articles.iter().find(|a| a.pmid == "31978945").unwrap();
    assert!(covid.title.contains("pneumonia"));
    assert_eq!(covid.journal, "Nature");
    assert_eq!(covid.authors.len(), 2);
    assert!(covid.abstract_text.is_some());
    // DOI extracted from PubmedData/ArticleIdList fallback
    assert_eq!(covid.doi.as_deref(), Some("10.1038/s41586-020-2008-3"));

    // Verify second article (Cancer)
    let cancer = articles.iter().find(|a| a.pmid == "33515491").unwrap();
    assert!(cancer.title.contains("Cancer"));
    assert_eq!(cancer.journal, "Lancet Oncology");

    // Verify third article (CRISPR)
    let crispr = articles.iter().find(|a| a.pmid == "25760099").unwrap();
    assert!(crispr.title.contains("CRISPR"));
    assert_eq!(crispr.journal, "Science");
}

/// Test batch fetching a single article
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_single_article() {
    let mock_server = setup_batch_efetch_mock(SINGLE_ARTICLE_RESPONSE).await;
    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_articles(&["12345678"])
        .await
        .expect("Single article batch fetch should succeed");

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].pmid, "12345678");
    assert_eq!(articles[0].title, "Single Test Article");
    assert_eq!(articles[0].journal, "Test Journal");
}

/// Test batch fetch with empty input returns empty vec
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_empty_input() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_articles(&[])
        .await
        .expect("Empty batch should return Ok");

    assert!(articles.is_empty());

    // Verify no requests were made
    let received_requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        received_requests.len(),
        0,
        "No HTTP requests should be made for empty input"
    );
}

/// Test batch fetch rejects invalid PMIDs before making network requests
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_invalid_pmid_rejected() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let result = client.fetch_articles(&["not_a_number"]).await;
    assert!(result.is_err(), "Invalid PMID should cause error");

    // Verify no requests were made
    let received_requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        received_requests.len(),
        0,
        "No HTTP requests should be made for invalid PMIDs"
    );
}

/// Test batch fetch rejects mixed valid/invalid PMIDs before making requests
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_mixed_valid_invalid_pmids() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let result = client
        .fetch_articles(&["31978945", "invalid", "25760099"])
        .await;
    assert!(result.is_err(), "Mixed valid/invalid PMIDs should fail");

    // No requests should be made if validation fails
    let received_requests = mock_server.received_requests().await.unwrap();
    assert_eq!(received_requests.len(), 0);
}

/// Test batch fetch rejects zero PMIDs
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_zero_pmid_rejected() {
    let mock_server = MockServer::start().await;
    let client = create_mock_client(&mock_server);

    let result = client.fetch_articles(&["0"]).await;
    assert!(result.is_err(), "PMID 0 should be rejected");
}

/// Test that batch fetch sends comma-separated IDs in a single request
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_sends_single_request() {
    let mock_server = MockServer::start().await;

    // Set up mock that expects comma-separated IDs
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BATCH_EFETCH_RESPONSE_3_ARTICLES))
        .expect(1) // Exactly one request
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_articles(&["31978945", "33515491", "25760099"])
        .await
        .expect("Batch fetch should succeed");

    assert_eq!(articles.len(), 3);

    // wiremock will verify expect(1) on drop
}

/// Test that search_and_fetch now uses batch internally
#[tokio::test]
#[traced_test]
async fn test_search_and_fetch_uses_batch() {
    let mock_server = MockServer::start().await;

    // ESearch returns 3 PMIDs
    Mock::given(method("GET"))
        .and(path_regex(r"/esearch\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "esearchresult": {
                        "count": "3",
                        "retmax": "3",
                        "retstart": "0",
                        "idlist": ["31978945", "33515491", "25760099"]
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    // EFetch should be called exactly once with all PMIDs (batch)
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BATCH_EFETCH_RESPONSE_3_ARTICLES))
        .expect(1) // Only 1 fetch request, not 3
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let articles = client
        .search_and_fetch("test query", 3)
        .await
        .expect("search_and_fetch should succeed");

    assert_eq!(articles.len(), 3);

    // wiremock verifies expect(1) for efetch on drop
}

/// Test batch fetch handles server error gracefully
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client.fetch_articles(&["31978945", "33515491"]).await;

    assert!(result.is_err(), "Server error should propagate");
}

/// Test batch fetch handles empty XML response
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_empty_xml_response() {
    let mock_server = setup_batch_efetch_mock("").await;
    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_articles(&["31978945"])
        .await
        .expect("Empty response should return Ok with empty vec");

    assert!(articles.is_empty());
}

/// Test batch fetch handles XML with no articles
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_empty_article_set() {
    let xml = r#"<?xml version="1.0" ?>
<PubmedArticleSet>
</PubmedArticleSet>"#;

    let mock_server = setup_batch_efetch_mock(xml).await;
    let client = create_mock_client(&mock_server);

    let articles = client
        .fetch_articles(&["99999999"])
        .await
        .expect("Empty article set should return Ok");

    assert!(articles.is_empty());
}

/// Test batch fetch with duplicate PMIDs
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_duplicate_pmids() {
    let mock_server = setup_batch_efetch_mock(SINGLE_ARTICLE_RESPONSE).await;
    let client = create_mock_client(&mock_server);

    // NCBI handles dedup on their side; we just verify no crash
    let articles = client
        .fetch_articles(&["12345678", "12345678"])
        .await
        .expect("Duplicate PMIDs should not cause error");

    assert!(!articles.is_empty());
}

/// Test batch fetch with rate-limited server (429)
#[tokio::test]
#[traced_test]
async fn test_batch_fetch_rate_limited() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Too Many Requests")
                .insert_header("retry-after", "1"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let result = client.fetch_articles(&["31978945", "33515491"]).await;

    assert!(result.is_err(), "429 response should result in error");
}
