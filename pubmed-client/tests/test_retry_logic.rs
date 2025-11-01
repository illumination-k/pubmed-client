//! Integration tests for retry logic with exponential backoff
//!
//! Tests the retry mechanism against simulated network failures and transient errors.

use pubmed_client::retry::RetryConfig;
use pubmed_client::{ClientConfig, PubMedClient, PubMedError};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tracing_test::traced_test;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Custom responder that fails N times before succeeding
struct FailNTimesResponder {
    failures_remaining: Arc<AtomicUsize>,
    failure_status: u16,
}

impl FailNTimesResponder {
    fn new(failures: usize, status: u16) -> Self {
        Self {
            failures_remaining: Arc::new(AtomicUsize::new(failures)),
            failure_status: status,
        }
    }
}

impl wiremock::Respond for FailNTimesResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let remaining = self.failures_remaining.fetch_sub(1, Ordering::SeqCst);
        if remaining > 0 {
            ResponseTemplate::new(self.failure_status)
        } else {
            // Return a valid article response
            ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0"?>
                    <PubmedArticleSet>
                        <PubmedArticle>
                            <MedlineCitation>
                                <PMID>12345678</PMID>
                                <Article>
                                    <ArticleTitle>Test Article</ArticleTitle>
                                    <Abstract>
                                        <AbstractText>This is a test abstract.</AbstractText>
                                    </Abstract>
                                    <AuthorList>
                                        <Author>
                                            <LastName>Doe</LastName>
                                            <ForeName>John</ForeName>
                                        </Author>
                                    </AuthorList>
                                    <Journal>
                                        <Title>Test Journal</Title>
                                    </Journal>
                                </Article>
                            </MedlineCitation>
                        </PubmedArticle>
                    </PubmedArticleSet>"#,
            )
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_retry_on_server_error() {
    let mock_server = MockServer::start().await;

    // Set up mock to fail twice with 500 error, then succeed
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(FailNTimesResponder::new(2, 500))
        .expect(3) // Should be called 3 times total
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig::new()
        .with_max_retries(3)
        .with_initial_delay(Duration::from_millis(100))
        .without_jitter();

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_retry_config(retry_config)
        .with_rate_limit(100.0); // High rate limit for testing

    let client = PubMedClient::with_config(config);

    let start = Instant::now();
    let result = client.fetch_article("12345678").await;
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Expected success after retries");
    let article = result.unwrap();
    assert_eq!(article.pmid, "12345678");
    assert_eq!(article.title, "Test Article");

    // Should have taken at least 300ms (100ms + 200ms delays)
    assert!(
        elapsed >= Duration::from_millis(300),
        "Expected delays not applied"
    );
}

#[tokio::test]
#[traced_test]
async fn test_retry_on_rate_limit() {
    let mock_server = MockServer::start().await;

    // Set up mock to return 429 once, then succeed
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(FailNTimesResponder::new(1, 429))
        .expect(2)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig::new()
        .with_max_retries(2)
        .with_initial_delay(Duration::from_millis(50))
        .without_jitter();

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_retry_config(retry_config)
        .with_rate_limit(100.0);

    let client = PubMedClient::with_config(config);

    let result = client.fetch_article("12345678").await;

    // Should succeed after retry
    assert!(result.is_ok());
}

#[tokio::test]
#[traced_test]
async fn test_no_retry_on_client_error() {
    let mock_server = MockServer::start().await;

    // Set up mock to return 404 error
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig::new()
        .with_max_retries(3)
        .with_initial_delay(Duration::from_millis(100));

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_retry_config(retry_config)
        .with_rate_limit(100.0);

    let client = PubMedClient::with_config(config);

    let start = Instant::now();
    let result = client.fetch_article("12345678").await;
    let elapsed = start.elapsed();

    // Should fail immediately without retries
    assert!(result.is_err());
    assert!(
        elapsed < Duration::from_millis(100),
        "Should not have retried"
    );
}

#[tokio::test]
#[traced_test]
async fn test_max_retries_exceeded() {
    let mock_server = MockServer::start().await;

    // Set up mock to always fail with 503
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(ResponseTemplate::new(503))
        .expect(3) // Initial + 2 retries
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig::new()
        .with_max_retries(2)
        .with_initial_delay(Duration::from_millis(50))
        .without_jitter();

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_retry_config(retry_config)
        .with_rate_limit(100.0);

    let client = PubMedClient::with_config(config);

    let result = client.fetch_article("12345678").await;

    // Should fail after exhausting retries
    assert!(result.is_err());
    match result {
        Err(PubMedError::ApiError { status, message: _ }) => {
            assert_eq!(status, 503, "Expected 503 status code");
        }
        Err(e) => {
            panic!("Expected ApiError, got: {:?}", e);
        }
        Ok(_) => {
            panic!("Expected error, but got success");
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_exponential_backoff_timing() {
    let mock_server = MockServer::start().await;

    // Set up mock to fail 3 times, then succeed
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(FailNTimesResponder::new(3, 500))
        .expect(4)
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig::new()
        .with_max_retries(3)
        .with_initial_delay(Duration::from_millis(100))
        .without_jitter();

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_retry_config(retry_config)
        .with_rate_limit(100.0);

    let client = PubMedClient::with_config(config);

    let start = Instant::now();
    let result = client.fetch_article("12345678").await;
    let elapsed = start.elapsed();

    // Should succeed
    assert!(result.is_ok());

    // Should have exponential delays: 100ms + 200ms + 400ms = 700ms minimum
    assert!(
        elapsed >= Duration::from_millis(700),
        "Expected exponential backoff delays, got {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_retry_disabled() {
    let mock_server = MockServer::start().await;

    // Set up mock to fail once
    Mock::given(method("GET"))
        .and(path("/efetch.fcgi"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1) // Should only try once
        .mount(&mock_server)
        .await;

    let retry_config = RetryConfig::new().with_max_retries(0); // No retries

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_retry_config(retry_config)
        .with_rate_limit(100.0);

    let client = PubMedClient::with_config(config);

    let result = client.fetch_article("12345678").await;

    // Should fail immediately
    assert!(result.is_err());
}
