//! Integration tests for search sort and query translation features
//!
//! These tests verify that sort parameters are passed correctly to the API
//! and that query translation is parsed from the response.

use pubmed_client::pubmed::SortOrder;
use pubmed_client::{ClientConfig, PubMedClient, SearchQuery};
use tracing_test::traced_test;
use wiremock::matchers::{method, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: JSON response from ESearch with querytranslation
fn esearch_json_response_with_translation(
    pmids: &[&str],
    total_count: usize,
    query_translation: &str,
) -> String {
    let id_list: Vec<String> = pmids.iter().map(|id| format!("\"{}\"", id)).collect();
    let escaped = query_translation.replace('"', "\\\"");
    format!(
        r#"{{
            "esearchresult": {{
                "count": "{}",
                "retmax": "{}",
                "retstart": "0",
                "querytranslation": "{}",
                "idlist": [{}]
            }}
        }}"#,
        total_count,
        pmids.len(),
        escaped,
        id_list.join(",")
    )
}

/// Helper: JSON response from ESearch with history and querytranslation
fn esearch_json_response_with_history_and_translation(
    pmids: &[&str],
    total_count: usize,
    query_translation: &str,
) -> String {
    let id_list: Vec<String> = pmids.iter().map(|id| format!("\"{}\"", id)).collect();
    let escaped = query_translation.replace('"', "\\\"");
    format!(
        r#"{{
            "esearchresult": {{
                "count": "{}",
                "retmax": "{}",
                "retstart": "0",
                "querytranslation": "{}",
                "webenv": "MCID_test123",
                "querykey": "1",
                "idlist": [{}]
            }}
        }}"#,
        total_count,
        pmids.len(),
        escaped,
        id_list.join(",")
    )
}

/// Helper: create PubMedClient pointing to mock server
fn create_test_client(base_url: &str) -> PubMedClient {
    let config = ClientConfig::new()
        .with_base_url(base_url)
        .with_rate_limit(100.0)
        .with_tool("test-client");
    PubMedClient::with_config(config)
}

// ================================================================================================
// Sort Order Tests
// ================================================================================================

#[tokio::test]
#[traced_test]
async fn test_search_articles_with_sort_pub_date() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(query_param("sort", "pub_date"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_translation(&["111", "222", "333"], 3, "asthma[All Fields]"),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let pmids = client
        .search_articles_with_options("asthma", 10, Some(&SortOrder::PublicationDate))
        .await
        .unwrap();

    assert_eq!(pmids.len(), 3);
    assert_eq!(pmids[0], "111");
}

#[tokio::test]
#[traced_test]
async fn test_search_articles_with_sort_first_author() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(query_param("sort", "Author"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_translation(&["444", "555"], 2, "cancer[All Fields]"),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let pmids = client
        .search_articles_with_options("cancer", 10, Some(&SortOrder::FirstAuthor))
        .await
        .unwrap();

    assert_eq!(pmids.len(), 2);
}

#[tokio::test]
#[traced_test]
async fn test_search_articles_with_sort_journal_name() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(query_param("sort", "JournalName"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_translation(&["666"], 1, "covid[All Fields]"),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let pmids = client
        .search_articles_with_options("covid", 10, Some(&SortOrder::JournalName))
        .await
        .unwrap();

    assert_eq!(pmids.len(), 1);
}

#[tokio::test]
#[traced_test]
async fn test_search_articles_without_sort() {
    let mock_server = MockServer::start().await;

    // When no sort is specified, the sort parameter should not be in the URL
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_translation(&["777"], 1, "test[All Fields]"),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let pmids = client
        .search_articles_with_options("test", 10, None)
        .await
        .unwrap();

    assert_eq!(pmids.len(), 1);
}

#[tokio::test]
#[traced_test]
async fn test_search_query_builder_with_sort() {
    let mock_server = MockServer::start().await;

    // ESearch mock (expects sort=pub_date)
    Mock::given(method("GET"))
        .and(query_param("sort", "pub_date"))
        .and(query_param("db", "pubmed"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_translation(
                &["123"],
                1,
                "\"cancer\"[MeSH Terms] OR \"cancer\"[All Fields]",
            ),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let query = SearchQuery::new()
        .query("cancer")
        .sort(SortOrder::PublicationDate)
        .limit(10);

    let pmids = query.search(&client).await.unwrap();
    assert_eq!(pmids.len(), 1);
    assert_eq!(pmids[0], "123");
}

// ================================================================================================
// Query Translation Tests
// ================================================================================================

#[tokio::test]
#[traced_test]
async fn test_search_with_history_returns_query_translation() {
    let mock_server = MockServer::start().await;

    let expected_translation = "\"asthma\"[MeSH Terms] OR \"asthma\"[All Fields]";

    Mock::given(method("GET"))
        .and(query_param("usehistory", "y"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_history_and_translation(
                &["111", "222"],
                500,
                expected_translation,
            ),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.search_with_history("asthma", 10).await.unwrap();

    assert_eq!(result.pmids.len(), 2);
    assert_eq!(result.total_count, 500);
    assert_eq!(
        result.query_translation.as_deref(),
        Some(expected_translation)
    );
    assert!(result.has_history());
}

#[tokio::test]
#[traced_test]
async fn test_search_with_history_no_query_translation() {
    let mock_server = MockServer::start().await;

    // Response without querytranslation field
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "esearchresult": {
                    "count": "1",
                    "retmax": "1",
                    "retstart": "0",
                    "webenv": "MCID_abc",
                    "querykey": "1",
                    "idlist": ["999"]
                }
            }"#,
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.search_with_history("test", 10).await.unwrap();

    assert_eq!(result.pmids.len(), 1);
    assert!(result.query_translation.is_none());
}

#[tokio::test]
#[traced_test]
async fn test_search_with_details_returns_query_translation_and_sort() {
    let mock_server = MockServer::start().await;

    let expected_translation = "\"cancer\"[MeSH Terms] OR \"cancer\"[All Fields]";

    Mock::given(method("GET"))
        .and(query_param("usehistory", "y"))
        .and(query_param("sort", "pub_date"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_history_and_translation(
                &["100", "200", "300"],
                1000,
                expected_translation,
            ),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let query = SearchQuery::new()
        .query("cancer")
        .sort(SortOrder::PublicationDate)
        .limit(10);

    let result = query.search_with_details(&client).await.unwrap();

    assert_eq!(result.pmids.len(), 3);
    assert_eq!(result.total_count, 1000);
    assert_eq!(
        result.query_translation.as_deref(),
        Some(expected_translation)
    );
    assert!(result.has_history());
}

#[tokio::test]
#[traced_test]
async fn test_search_with_history_and_options_sort_and_translation() {
    let mock_server = MockServer::start().await;

    let expected_translation = "\"vaccine\"[MeSH Terms] OR \"vaccine\"[All Fields]";

    Mock::given(method("GET"))
        .and(query_param("sort", "JournalName"))
        .and(query_param("usehistory", "y"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            esearch_json_response_with_history_and_translation(
                &["50", "51"],
                2,
                expected_translation,
            ),
        ))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .search_with_history_and_options("vaccine", 10, Some(&SortOrder::JournalName))
        .await
        .unwrap();

    assert_eq!(result.pmids.len(), 2);
    assert_eq!(
        result.query_translation.as_deref(),
        Some(expected_translation)
    );
}

// ================================================================================================
// Sort Order Unit Tests
// ================================================================================================

#[test]
fn test_search_query_sort_builder() {
    let query = SearchQuery::new()
        .query("test")
        .sort(SortOrder::PublicationDate);

    assert_eq!(query.get_sort(), Some(&SortOrder::PublicationDate));
    assert_eq!(query.build(), "test");
    // Sort should not affect the query string itself
}

#[test]
fn test_search_query_sort_override() {
    let query = SearchQuery::new()
        .query("test")
        .sort(SortOrder::Relevance)
        .sort(SortOrder::FirstAuthor);

    assert_eq!(query.get_sort(), Some(&SortOrder::FirstAuthor));
}

#[test]
fn test_search_query_no_sort_by_default() {
    let query = SearchQuery::new().query("test");
    assert_eq!(query.get_sort(), None);
}
