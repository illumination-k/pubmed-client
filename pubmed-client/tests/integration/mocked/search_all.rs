//! Integration tests for `PubMedClient::search_all` streaming pagination.
//!
//! These tests exercise the history-server-backed streaming search against a
//! mocked NCBI endpoint (wiremock) so the pagination state machine can be
//! verified without network access. They focus on the risky boundaries where
//! an off-by-one in the offset/index bookkeeping would silently drop or
//! duplicate articles:
//!
//! * empty first page (search matches nothing)
//! * error mid-stream (a later batch fails)
//! * result count exactly on a batch boundary
//! * final partial batch

use futures_util::StreamExt;
use pubmed_client::{ClientConfig, PubMedClient, Result};
use std::pin::pin;
use tracing_test::traced_test;
use wiremock::matchers::{method, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a client pointing at a mock server.
fn create_mock_client(mock_server: &MockServer) -> PubMedClient {
    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_rate_limit(100.0); // High rate limit for tests

    PubMedClient::with_config(config)
}

/// Build an ESearch (usehistory=y) JSON body with the given result count and
/// returned PMIDs. Always includes a WebEnv/query_key so streaming can proceed.
fn esearch_body(count: usize, pmids: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "esearchresult": {
            "count": count.to_string(),
            "retmax": pmids.len().to_string(),
            "retstart": "0",
            "idlist": pmids,
            "webenv": "MCID_search_all_test",
            "querykey": "1",
        }
    })
}

/// Build an EFetch PubmedArticleSet XML body containing one article per PMID.
fn efetch_body(pmids: &[&str]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" ?>\n<PubmedArticleSet>\n");
    for pmid in pmids {
        xml.push_str(&format!(
            r#"    <PubmedArticle>
        <MedlineCitation>
            <PMID Version="1">{pmid}</PMID>
            <Article>
                <Journal><Title>Test Journal</Title></Journal>
                <ArticleTitle>Article {pmid}</ArticleTitle>
                <AuthorList>
                    <Author><LastName>Doe</LastName><ForeName>Jane</ForeName></Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType>Journal Article</PublicationType>
                </PublicationTypeList>
            </Article>
        </MedlineCitation>
    </PubmedArticle>
"#
        ));
    }
    xml.push_str("</PubmedArticleSet>");
    xml
}

/// Mount the ESearch mock returning the given count/PMIDs.
async fn mount_esearch(mock_server: &MockServer, count: usize, pmids: &[&str]) {
    Mock::given(method("GET"))
        .and(path_regex(r"/esearch\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(esearch_body(count, pmids))
                .insert_header("content-type", "application/json"),
        )
        .mount(mock_server)
        .await;
}

/// Mount an EFetch mock scoped to a specific `retstart` offset so each batch
/// returns a distinct set of articles.
async fn mount_efetch_at(mock_server: &MockServer, retstart: usize, pmids: &[&str]) {
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .and(query_param("retstart", retstart.to_string()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(efetch_body(pmids))
                .insert_header("content-type", "application/xml"),
        )
        .mount(mock_server)
        .await;
}

/// Collect a stream of `Result<PubMedArticle>` into the successfully-yielded
/// PMIDs and a flag indicating whether any error was yielded.
async fn collect_pmids(
    stream: impl futures_util::Stream<Item = Result<pubmed_client::PubMedArticle>>,
) -> (Vec<String>, bool) {
    let mut stream = pin!(stream);
    let mut pmids = Vec::new();
    let mut saw_error = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(article) => pmids.push(article.pmid),
            Err(_) => saw_error = true,
        }
    }
    (pmids, saw_error)
}

/// Empty first page: the search matches nothing, so the stream yields no items
/// (and, in particular, never performs an EFetch).
#[tokio::test]
#[traced_test]
async fn test_search_all_empty_first_page() {
    let mock_server = MockServer::start().await;
    mount_esearch(&mock_server, 0, &[]).await;

    let client = create_mock_client(&mock_server);

    let (pmids, saw_error) = collect_pmids(client.search_all("no matches", 10)).await;

    assert!(pmids.is_empty(), "empty search should yield no articles");
    assert!(!saw_error, "empty search should not yield an error");

    // No EFetch request should be made when there are no results.
    let requests = mock_server.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .all(|r| !r.url.path().contains("efetch.fcgi")),
        "no EFetch request should be made for an empty result set"
    );
}

/// Missing WebEnv: the search returns PMIDs but no history session, so the
/// stream cannot paginate and yields a single error.
#[tokio::test]
#[traced_test]
async fn test_search_all_missing_webenv_yields_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/esearch\.fcgi.*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "esearchresult": {
                        "count": "2",
                        "retmax": "2",
                        "retstart": "0",
                        "idlist": ["1", "2"],
                        // no webenv / querykey
                    }
                }))
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    let (pmids, saw_error) = collect_pmids(client.search_all("no webenv", 10)).await;

    assert!(pmids.is_empty(), "no articles should be yielded");
    assert!(saw_error, "missing WebEnv should yield an error");
}

/// Single batch: all results fit in the first fetch. Verifies no phantom second
/// fetch occurs once the batch is exhausted.
#[tokio::test]
#[traced_test]
async fn test_search_all_single_batch() {
    let mock_server = MockServer::start().await;
    mount_esearch(&mock_server, 2, &["11", "22"]).await;
    mount_efetch_at(&mock_server, 0, &["11", "22"]).await;

    let client = create_mock_client(&mock_server);

    let (pmids, saw_error) = collect_pmids(client.search_all("two results", 10)).await;

    assert!(!saw_error, "no error expected");
    assert_eq!(pmids, vec!["11".to_string(), "22".to_string()]);
}

/// Result count exactly on a batch boundary: two full batches of `batch_size`,
/// then the stream stops without an extra fetch or any duplicate article.
#[tokio::test]
#[traced_test]
async fn test_search_all_exact_batch_boundary() {
    let mock_server = MockServer::start().await;
    // total = 4, batch_size = 2 → exactly two full batches.
    mount_esearch(&mock_server, 4, &["1", "2"]).await;
    mount_efetch_at(&mock_server, 0, &["1", "2"]).await;
    mount_efetch_at(&mock_server, 2, &["3", "4"]).await;

    let client = create_mock_client(&mock_server);

    let (pmids, saw_error) = collect_pmids(client.search_all("four results", 2)).await;

    assert!(!saw_error, "no error expected");
    assert_eq!(
        pmids,
        vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string()
        ],
        "all four articles should be yielded exactly once with no duplicates"
    );

    // Exactly two EFetch requests: at offset 0 and offset 2. There must be no
    // fetch at offset 4 (which would be past the boundary).
    let requests = mock_server.received_requests().await.unwrap();
    let efetch_count = requests
        .iter()
        .filter(|r| r.url.path().contains("efetch.fcgi"))
        .count();
    assert_eq!(
        efetch_count, 2,
        "should fetch exactly two batches at the boundary, not a third"
    );
}

/// Final partial batch: the last batch is smaller than `batch_size`. Verifies
/// the trailing records are all yielded and the stream terminates cleanly.
#[tokio::test]
#[traced_test]
async fn test_search_all_final_partial_batch() {
    let mock_server = MockServer::start().await;
    // total = 3, batch_size = 2 → full batch of 2, then partial batch of 1.
    mount_esearch(&mock_server, 3, &["1", "2"]).await;
    mount_efetch_at(&mock_server, 0, &["1", "2"]).await;
    mount_efetch_at(&mock_server, 2, &["3"]).await;

    let client = create_mock_client(&mock_server);

    let (pmids, saw_error) = collect_pmids(client.search_all("three results", 2)).await;

    assert!(!saw_error, "no error expected");
    assert_eq!(
        pmids,
        vec!["1".to_string(), "2".to_string(), "3".to_string()],
        "the trailing partial batch should be yielded"
    );
}

/// Empty later batch: the server reports more results than it actually returns.
/// When a subsequent fetch comes back empty, the stream must stop rather than
/// loop or panic.
#[tokio::test]
#[traced_test]
async fn test_search_all_empty_later_batch_terminates() {
    let mock_server = MockServer::start().await;
    // total claims 4, but the second batch (offset 2) comes back empty.
    mount_esearch(&mock_server, 4, &["1", "2"]).await;
    mount_efetch_at(&mock_server, 0, &["1", "2"]).await;
    mount_efetch_at(&mock_server, 2, &[]).await;

    let client = create_mock_client(&mock_server);

    let (pmids, saw_error) = collect_pmids(client.search_all("short server", 2)).await;

    assert!(!saw_error, "an empty batch is not an error");
    assert_eq!(
        pmids,
        vec!["1".to_string(), "2".to_string()],
        "only the articles actually returned should be yielded"
    );
}

/// Error mid-stream: the first batch succeeds, but a later batch fails with a
/// server error. The already-yielded articles are preserved and the failure is
/// surfaced as an error item.
#[tokio::test]
#[traced_test]
async fn test_search_all_error_mid_stream() {
    let mock_server = MockServer::start().await;
    mount_esearch(&mock_server, 4, &["1", "2"]).await;
    mount_efetch_at(&mock_server, 0, &["1", "2"]).await;

    // Second batch (offset 2) fails.
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .and(query_param("retstart", "2"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = create_mock_client(&mock_server);

    // Draining the stream to completion here also asserts that it terminates
    // after the error (rather than looping): the `while` loop only exits when
    // the stream yields `None`.
    let mut stream = pin!(client.search_all("error mid stream", 2));
    let mut oks = Vec::new();
    let mut errors = 0;
    while let Some(item) = stream.next().await {
        match item {
            Ok(article) => oks.push(article.pmid),
            Err(_) => errors += 1,
        }
    }

    assert_eq!(
        oks,
        vec!["1".to_string(), "2".to_string()],
        "articles from the successful first batch should be preserved"
    );
    assert_eq!(errors, 1, "the mid-stream failure should be yielded once");
}
