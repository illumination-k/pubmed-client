//! Integration tests for the Europe PMC client using mocked HTTP responses.
//!
//! These exercise the client end-to-end (URL construction, pagination loops,
//! JSON/JATS parsing, and the non-PMC full-text guard) without real network
//! access, using wiremock.

use pubmed_client::europe_pmc::{EuropePmcClient, EuropePmcId, EuropePmcSource};
use tracing_test::traced_test;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(mock_server: &MockServer) -> EuropePmcClient {
    EuropePmcClient::new().with_base_url(mock_server.uri())
}

fn json_response(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(body.to_string())
        .insert_header("content-type", "application/json")
}

#[tokio::test]
#[traced_test]
async fn test_search_follows_cursor_across_pages() {
    let mock_server = MockServer::start().await;

    // First page (cursorMark=*) returns 2 results and advances the cursor.
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("cursorMark", "*"))
        .respond_with(json_response(
            r#"{
                "hitCount": 3,
                "nextCursorMark": "PAGE2",
                "resultList": {"result": [
                    {"id": "1", "source": "MED", "title": "First", "pubYear": "2020"},
                    {"id": "2", "source": "PMC", "pmcid": "PMC2", "title": "Second"}
                ]}
            }"#,
        ))
        .mount(&mock_server)
        .await;

    // Second page (cursorMark=PAGE2) returns 1 result, cursor stops advancing.
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("cursorMark", "PAGE2"))
        .respond_with(json_response(
            r#"{
                "hitCount": 3,
                "nextCursorMark": "PAGE2",
                "resultList": {"result": [
                    {"id": "3", "source": "PPR", "title": "Third"}
                ]}
            }"#,
        ))
        .mount(&mock_server)
        .await;

    let results = client(&mock_server)
        .search("cancer", 100)
        .await
        .expect("search should succeed");

    assert_eq!(results.len(), 3, "should collect across both pages");
    assert_eq!(results[0].id, "1");
    assert_eq!(results[1].pmcid.as_deref(), Some("PMC2"));
    assert_eq!(results[2].source, "PPR");
}

#[tokio::test]
#[traced_test]
async fn test_search_respects_limit() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(json_response(
            r#"{
                "hitCount": 50,
                "nextCursorMark": "NEXT",
                "resultList": {"result": [
                    {"id": "1", "source": "MED"},
                    {"id": "2", "source": "MED"},
                    {"id": "3", "source": "MED"}
                ]}
            }"#,
        ))
        .mount(&mock_server)
        .await;

    let results = client(&mock_server)
        .search("x", 2)
        .await
        .expect("search should succeed");
    assert_eq!(results.len(), 2, "should truncate to the requested limit");
}

/// Minimal but structurally valid JATS, matching what Europe PMC serves from
/// `fullTextXML`. Parsed by the same `parse_pmc_xml` used for NCBI PMC.
const JATS_FULL_TEXT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<article article-type="research-article">
  <front>
    <journal-meta>
      <journal-title-group><journal-title>Test Journal</journal-title></journal-title-group>
    </journal-meta>
    <article-meta>
      <article-id pub-id-type="pmcid">PMC10618641</article-id>
      <article-id pub-id-type="doi">10.1000/test</article-id>
      <title-group><article-title>A Europe PMC test article</article-title></title-group>
    </article-meta>
  </front>
  <body>
    <sec><title>Introduction</title><p>Some body text.</p></sec>
  </body>
</article>"#;

#[tokio::test]
#[traced_test]
async fn test_fetch_full_text_parses_jats() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/PMC/PMC10618641/fullTextXML"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(JATS_FULL_TEXT.to_string())
                .insert_header("content-type", "application/xml"),
        )
        .mount(&mock_server)
        .await;

    let id = EuropePmcId::pmc("PMC10618641").unwrap();
    let article = client(&mock_server)
        .fetch_full_text(&id)
        .await
        .expect("full text should parse");
    assert_eq!(article.title(), Some("A Europe PMC test article"));
}

#[tokio::test]
#[traced_test]
async fn test_fetch_full_text_rejects_non_pmc_source() {
    let mock_server = MockServer::start().await;
    // No mock needed: the guard short-circuits before any request.
    let id = EuropePmcId::new(EuropePmcSource::Med, "12345");
    let err = client(&mock_server)
        .fetch_full_text(&id)
        .await
        .expect_err("MED source has no PmcArticle full text");
    assert!(
        err.to_string().to_lowercase().contains("not available"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
#[traced_test]
async fn test_get_references_paginates() {
    let mock_server = MockServer::start().await;

    // hitCount 2 but pageSize is large, so a single full page ends pagination.
    Mock::given(method("GET"))
        .and(path("/PMC/PMC10618641/references"))
        .respond_with(json_response(
            r#"{
                "hitCount": 2,
                "referenceList": {"reference": [
                    {"id": 100, "title": "Ref one", "pubYear": 2010, "pmid": "100"},
                    {"title": "Ref two", "pubYear": 1999}
                ]}
            }"#,
        ))
        .mount(&mock_server)
        .await;

    let id = EuropePmcId::pmc("PMC10618641").unwrap();
    let refs = client(&mock_server)
        .get_references(&id)
        .await
        .expect("references should fetch");
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].pub_year.as_deref(), Some("2010"));
    assert_eq!(refs[0].id.as_deref(), Some("100"));
}

#[tokio::test]
#[traced_test]
async fn test_get_citations() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/MED/12345/citations"))
        .respond_with(json_response(
            r#"{
                "hitCount": 1,
                "citationList": {"citation": [
                    {"id": "999", "source": "MED", "title": "Citing", "citedByCount": 4}
                ]}
            }"#,
        ))
        .mount(&mock_server)
        .await;

    let id = EuropePmcId::med("12345");
    let citations = client(&mock_server)
        .get_citations(&id)
        .await
        .expect("citations should fetch");
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].cited_by_count.as_deref(), Some("4"));
}

#[tokio::test]
#[traced_test]
async fn test_get_database_links() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/MED/12345/databaseLinks"))
        .respond_with(json_response(
            r#"{
                "hitCount": 1,
                "dbCrossReferenceList": {"dbCrossReference": [
                    {"dbName": "UNIPROT", "dbCount": 1,
                     "dbCrossReferenceInfo": [{"info1": "P12345"}]}
                ]}
            }"#,
        ))
        .mount(&mock_server)
        .await;

    let id = EuropePmcId::med("12345");
    let links = client(&mock_server)
        .get_database_links(&id)
        .await
        .expect("database links should fetch");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].db_name.as_deref(), Some("UNIPROT"));
    assert_eq!(links[0].info[0].info1.as_deref(), Some("P12345"));
}
