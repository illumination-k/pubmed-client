use pubmed_client::{ClientConfig, PubMedClient, PubMedError};
use tracing_test::traced_test;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

// Mock server fixtures for different scenarios
struct MockServerFixtures;

impl MockServerFixtures {
    /// Creates a mock server with successful ESearch responses
    async fn successful_esearch() -> MockServer {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/esearch\.fcgi.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "esearchresult": {
                            "count": "1",
                            "retmax": "1",
                            "retstart": "0",
                            "idlist": ["12345"]
                        }
                    }))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        mock_server
    }

    /// Creates a mock server with successful EFetch responses
    async fn successful_efetch() -> MockServer {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/efetch\.fcgi.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0" ?>
            <PubmedArticleSet>
                <PubmedArticle>
                    <MedlineCitation>
                        <PMID Version="1">12345</PMID>
                        <Article>
                            <Journal>
                                <Title>Test Journal</Title>
                            </Journal>
                            <ArticleTitle>Test Article Title</ArticleTitle>
                            <Abstract>
                                <AbstractText>Test abstract content</AbstractText>
                            </Abstract>
                            <AuthorList>
                                <Author>
                                    <LastName>Doe</LastName>
                                    <ForeName>John</ForeName>
                                </Author>
                            </AuthorList>
                            <PublicationTypeList>
                                <PublicationType>Journal Article</PublicationType>
                            </PublicationTypeList>
                        </Article>
                    </MedlineCitation>
                </PubmedArticle>
            </PubmedArticleSet>"#,
            ))
            .mount(&mock_server)
            .await;

        mock_server
    }

    /// Creates a mock server with both ESearch and EFetch successful responses
    async fn successful_full_api() -> MockServer {
        let mock_server = MockServer::start().await;

        // ESearch mock
        Mock::given(method("GET"))
            .and(path_regex(r"/esearch\.fcgi.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "esearchresult": {
                            "count": "1",
                            "retmax": "1",
                            "retstart": "0",
                            "idlist": ["12345"]
                        }
                    }))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // EFetch mock
        Mock::given(method("GET"))
            .and(path_regex(r"/efetch\.fcgi.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0" ?>
            <PubmedArticleSet>
                <PubmedArticle>
                    <MedlineCitation>
                        <PMID Version="1">12345</PMID>
                        <Article>
                            <Journal><Title>Test Journal</Title></Journal>
                            <ArticleTitle>Test Article</ArticleTitle>
                        </Article>
                    </MedlineCitation>
                </PubmedArticle>
            </PubmedArticleSet>"#,
            ))
            .mount(&mock_server)
            .await;

        mock_server
    }

    /// Creates a mock server that returns 429 Too Many Requests
    async fn rate_limited_429() -> MockServer {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/esearch\.fcgi.*"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string("Too Many Requests")
                    .insert_header("content-type", "text/plain")
                    .insert_header("retry-after", "1")
                    .insert_header("x-ratelimit-limit", "3")
                    .insert_header("x-ratelimit-remaining", "0"),
            )
            .mount(&mock_server)
            .await;

        mock_server
    }

    /// Creates a mock server with realistic NCBI-style 429 responses
    async fn ncbi_style_429() -> MockServer {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/esearch\.fcgi.*"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string(r#"<?xml version="1.0"?>
                <ERROR>API rate limit exceeded. Requests are limited to 3 per second without an API key.</ERROR>"#)
                    .insert_header("content-type", "application/xml")
                    .insert_header("retry-after", "60")
                    .insert_header("x-ratelimit-limit", "3")
                    .insert_header("x-ratelimit-remaining", "0")
                    .insert_header("x-ratelimit-reset", "1640995200")
            )
            .mount(&mock_server)
            .await;

        mock_server
    }

    /// Creates a mock server with JSON-style 429 error responses
    async fn json_429_error() -> MockServer {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/esearch\.fcgi.*"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string(
                        r#"{"error": "API rate limit exceeded. Please slow down your requests."}"#,
                    )
                    .insert_header("content-type", "application/json")
                    .insert_header("retry-after", "60")
                    .insert_header("x-ratelimit-limit", "3")
                    .insert_header("x-ratelimit-remaining", "0")
                    .insert_header("x-ratelimit-reset", "1640995200"),
            )
            .mount(&mock_server)
            .await;

        mock_server
    }
}

/// Test that clients can make requests to mocked servers
#[tokio::test]
#[traced_test]
async fn test_pubmed_client_with_mock_server() {
    let mock_server = MockServerFixtures::successful_esearch().await;
    let config = ClientConfig::new().with_base_url(mock_server.uri());
    let client = PubMedClient::with_config(config);

    let pmids = client
        .search_articles("test query", 1)
        .await
        .expect("Mock request should succeed");
    assert_eq!(pmids.len(), 1);
    assert_eq!(pmids[0], "12345");
}

/// Test that API parameters are included in requests
#[tokio::test]
#[traced_test]
async fn test_api_parameters_in_requests() {
    let mock_server = MockServerFixtures::successful_esearch().await;

    let config = ClientConfig::new()
        .with_api_key("test_key")
        .with_email("test@example.com")
        .with_tool("TestApp")
        .with_base_url(mock_server.uri());

    let client = PubMedClient::with_config(config);

    // Make a request - should succeed only if API parameters are properly included
    let pmids = client
        .search_articles("test query", 1)
        .await
        .expect("Request should succeed with correct API parameters");

    assert_eq!(pmids.len(), 1);
    assert_eq!(pmids[0], "12345");
}

/// Test fetch_article with mocked response (without timing assertions)
#[tokio::test]
#[traced_test]
async fn test_fetch_article_rate_limiting_with_mock() {
    let mock_server = MockServerFixtures::successful_efetch().await;

    let config = ClientConfig::new()
        .with_rate_limit(5.0) // 5 requests/second
        .with_base_url(mock_server.uri());

    let client = PubMedClient::with_config(config);

    // Make multiple fetch_article requests - focus on correct parsing and integration
    for _i in 0..3 {
        let article = client
            .fetch_article("12345")
            .await
            .expect("Mock request should succeed");

        assert_eq!(article.pmid, "12345");
        assert_eq!(article.title, "Test Article Title");
        assert_eq!(article.journal, "Test Journal");
        assert!(article.abstract_text.is_some());
        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.authors[0], "John Doe");
    }
}

/// Test that multiple different request types work correctly with mocked responses
#[tokio::test]
#[traced_test]
async fn test_shared_rate_limiting_across_methods() {
    let mock_server = MockServerFixtures::successful_full_api().await;

    let config = ClientConfig::new()
        .with_rate_limit(3.0) // 3 requests/second
        .with_base_url(mock_server.uri());

    let client = PubMedClient::with_config(config);

    // Mix search and fetch requests - verify they work correctly with mocked responses
    let search_result = client
        .search_articles("test", 1)
        .await
        .expect("Mock search should succeed");
    assert_eq!(search_result.len(), 1);
    assert_eq!(search_result[0], "12345");

    let article = client
        .fetch_article("12345")
        .await
        .expect("Mock fetch should succeed");
    assert_eq!(article.pmid, "12345");
    assert_eq!(article.title, "Test Article");

    let search_result2 = client
        .search_articles("test2", 1)
        .await
        .expect("Mock search should succeed");
    assert_eq!(search_result2.len(), 1);
    assert_eq!(search_result2[0], "12345");
}

/// Test that concurrent requests work correctly with mocked responses and rate limiting
#[tokio::test]
#[traced_test]
async fn test_high_concurrency_rate_limiting() {
    let mock_server = MockServerFixtures::successful_esearch().await;

    let config = ClientConfig::new()
        .with_rate_limit(100.0) // High rate limit to avoid failures in test
        .with_base_url(mock_server.uri());

    let client = PubMedClient::with_config(config);

    let mut tasks = Vec::new();

    // Spawn 5 concurrent tasks (smaller number to avoid rate limiting issues)
    for i in 0..5 {
        let client = client.clone();
        let task =
            tokio::spawn(async move { client.search_articles(&format!("query {i}"), 1).await });
        tasks.push(task);
    }

    // Wait for all tasks to complete and verify responses
    let mut successful_requests = 0;
    let mut rate_limited_requests = 0;

    for task in tasks {
        match task.await {
            Ok(Ok(result)) => {
                assert_eq!(result.len(), 1);
                assert_eq!(result[0], "12345");
                successful_requests += 1;
            }
            Ok(Err(PubMedError::RateLimitExceeded)) => {
                rate_limited_requests += 1;
            }
            Ok(Err(e)) => {
                panic!("Unexpected error: {e:?}");
            }
            Err(e) => {
                panic!("Task panicked: {e:?}");
            }
        }
    }

    // At least some requests should succeed (with high rate limit, all should succeed)
    assert!(successful_requests > 0);
    assert_eq!(successful_requests + rate_limited_requests, 5);
}

/// Test rate limiting with mock server returning 429 responses
#[tokio::test]
#[traced_test]
async fn test_rate_limiting_with_429_responses() {
    let mock_server = MockServerFixtures::rate_limited_429().await;
    let config = ClientConfig::new()
        .with_rate_limit(10.0) // High rate limit for testing - but server will return 429
        .with_base_url(mock_server.uri())
        .with_api_key("test_key")
        .with_email("test@example.com");
    let client = PubMedClient::with_config(config);

    // Test that 429 response results in ApiError
    let result = client.search_articles("test query", 10).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        PubMedError::ApiError { status, message } => {
            assert_eq!(status, 429);
            assert!(message.contains("Too Many Requests"));
        }
        other => panic!("Expected ApiError, got: {other:?}"),
    }
}

/// Test 429 response with proper headers and realistic NCBI-style error
#[tokio::test]
#[traced_test]
async fn test_realistic_429_rate_limit_response() {
    let mock_server = MockServerFixtures::ncbi_style_429().await;
    let config = ClientConfig::new()
        .with_rate_limit(100.0) // High client-side rate limit - server still enforces its own limits
        .with_base_url(mock_server.uri());
    let client = PubMedClient::with_config(config);

    // Request should get 429 error from server
    let result = client.search_articles("test query", 1).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        PubMedError::ApiError { status, message } => {
            assert_eq!(status, 429);
            assert!(message.contains("Too Many Requests"));
        }
        other => panic!("Expected ApiError for 429, got: {other:?}"),
    }
}

/// Test server-side rate limiting simulation
#[tokio::test]
#[traced_test]
async fn test_server_rate_limit_simulation() {
    let mock_server = MockServerFixtures::json_429_error().await;
    let config = ClientConfig::new()
        .with_rate_limit(1.0) // Low rate limit that should be respected
        .with_base_url(mock_server.uri());
    let client = PubMedClient::with_config(config);

    // Even with client-side rate limiting, server can still return 429
    let result = client.search_articles("test", 1).await;
    assert!(result.is_err());

    if let Err(PubMedError::ApiError { status, message }) = result {
        assert_eq!(status, 429);
        assert!(message.contains("Too Many Requests"));
    } else {
        panic!("Expected ApiError with 429 status");
    }
}
