//! Mocked integration tests for the PubTator3 client.
//!
//! These verify request shaping (paths, query parameters) and response parsing
//! against captured fixtures, without making real network calls.

use std::fs;

use tracing_test::traced_test;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pubmed_client::{EntityType, PubTatorClient};

/// Resolve a workspace-root-relative fixture path from either the workspace
/// root or the crate root (tests may run from either directory).
#[allow(clippy::expect_used)]
fn read_fixture(relative: &str) -> String {
    let candidates = [relative.to_string(), format!("../{relative}")];
    candidates
        .iter()
        .find_map(|candidate| fs::read_to_string(candidate).ok())
        .expect("fixture should be readable from one of the candidate paths")
}

fn mock_client(server: &MockServer) -> PubTatorClient {
    PubTatorClient::new().with_base_url(server.uri())
}

#[tokio::test]
#[traced_test]
async fn test_export_annotations_parses_documents() {
    let body = read_fixture("test_data/pubtator/biocjson_two_abstracts.json");
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/publications/export/biocjson"))
        .and(query_param("pmids", "29355051,28483577"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let response = mock_client(&server)
        .export_annotations(&["29355051", "28483577"])
        .await
        .expect("export should succeed");

    assert_eq!(response.documents.len(), 2);
    let doc = response.document("29355051").expect("document present");
    let chemicals: Vec<_> = doc
        .annotations_of_type(EntityType::Chemical)
        .map(|a| a.text.as_str())
        .collect();
    assert!(chemicals.contains(&"Doxorubicin"));
}

#[tokio::test]
#[traced_test]
async fn test_export_full_text_sets_full_param() {
    let body = read_fixture("test_data/pubtator/biocjson_two_abstracts.json");
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/publications/export/biocjson"))
        .and(query_param("full", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let response = mock_client(&server)
        .export_full_text_annotations(&["29355051"])
        .await
        .expect("full-text export should succeed");
    assert!(!response.documents.is_empty());
}

#[tokio::test]
#[traced_test]
async fn test_find_entity_parses_matches() {
    let body = read_fixture("test_data/pubtator/autocomplete_covid.json");
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/entity/autocomplete/"))
        .and(query_param("query", "covid-19"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let matches = mock_client(&server)
        .find_entity("covid-19")
        .await
        .expect("entity lookup should succeed");

    assert!(!matches.is_empty());
    assert_eq!(matches[0].entity_type(), EntityType::Disease);
}

#[tokio::test]
async fn test_empty_pmids_skips_request() {
    // No mock mounted: an empty input must not issue any HTTP request.
    let server = MockServer::start().await;
    let response = mock_client(&server)
        .export_annotations(&[])
        .await
        .expect("empty input should short-circuit");
    assert!(response.documents.is_empty());
}
