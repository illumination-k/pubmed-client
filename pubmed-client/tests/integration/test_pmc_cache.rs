use pubmed_client::{cache::CacheConfig, ClientConfig, PmcClient};
use std::time::Duration;
use wiremock::{
    matchers::{method, path_regex},
    Mock, MockServer, ResponseTemplate,
};

/// Test data for a simple PMC article
const PMC_XML_RESPONSE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE pmc-articleset PUBLIC "-//NLM//DTD ARTICLE SET 2.0//EN" "https://dtd.nlm.nih.gov/ncbi/pmc/articleset/nlm-articleset-2.0.dtd">
<pmc-articleset>
<article xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:mml="http://www.w3.org/1998/Math/MathML" article-type="research-article">
  <front>
    <journal-meta>
      <journal-title-group>
        <journal-title>Test Journal</journal-title>
      </journal-title-group>
    </journal-meta>
    <article-meta>
      <article-id pub-id-type="pmc">1234567</article-id>
      <title-group>
        <article-title>Test Article Title</article-title>
      </title-group>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Doe</surname>
            <given-names>John</given-names>
          </name>
        </contrib>
      </contrib-group>
      <abstract>
        <p>This is a test abstract.</p>
      </abstract>
    </article-meta>
  </front>
  <body>
    <sec>
      <title>Introduction</title>
      <p>This is the introduction section.</p>
    </sec>
  </body>
</article>
</pmc-articleset>"#;

#[tokio::test]
async fn test_pmc_cache_hit() {
    let mock_server = MockServer::start().await;

    // Set up mock to track number of calls
    let mock = Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PMC_XML_RESPONSE))
        .expect(1) // Should only be called once due to caching
        .mount_as_scoped(&mock_server)
        .await;

    // Create client with caching enabled
    let cache_config = CacheConfig {
        max_capacity: 100,
        time_to_live: Duration::from_secs(60),
    };

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_cache_config(cache_config);

    let client = PmcClient::with_config(config);

    // First fetch - should hit the API
    let article1 = client.fetch_full_text("PMC1234567").await.unwrap();
    assert_eq!(article1.title, "Test Article Title");

    // Second fetch - should be served from cache
    let article2 = client.fetch_full_text("PMC1234567").await.unwrap();
    assert_eq!(article2.title, "Test Article Title");

    // Verify both articles are identical
    assert_eq!(article1.pmcid, article2.pmcid);
    assert_eq!(article1.sections.len(), article2.sections.len());

    // The mock should have been called exactly once
    drop(mock);
}

#[tokio::test]
async fn test_pmc_cache_miss_different_ids() {
    let mock_server = MockServer::start().await;

    // Set up mock to track calls
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PMC_XML_RESPONSE))
        .expect(2) // Should be called twice for different PMCIDs
        .mount(&mock_server)
        .await;

    // Create client with caching enabled
    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_cache_config(CacheConfig::default());

    let client = PmcClient::with_config(config);

    // Fetch two different articles
    let _article1 = client.fetch_full_text("PMC1234567").await.unwrap();
    let _article2 = client.fetch_full_text("PMC7654321").await.unwrap();

    // Both should trigger API calls as they have different IDs
}

#[tokio::test]
async fn test_pmc_cache_clear() {
    let mock_server = MockServer::start().await;

    // Set up mock to track calls
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PMC_XML_RESPONSE))
        .expect(2) // Should be called twice due to cache clear
        .mount(&mock_server)
        .await;

    // Create client with caching enabled
    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_cache_config(CacheConfig::default());

    let client = PmcClient::with_config(config);

    // First fetch
    let _article1 = client.fetch_full_text("PMC1234567").await.unwrap();

    // Clear cache
    client.clear_cache().await;

    // Second fetch should hit API again
    let _article2 = client.fetch_full_text("PMC1234567").await.unwrap();
}

#[tokio::test]
async fn test_pmc_cache_stats() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PMC_XML_RESPONSE))
        .mount(&mock_server)
        .await;

    // Create client with caching enabled
    let cache_config = CacheConfig {
        max_capacity: 100,
        time_to_live: Duration::from_secs(60),
    };

    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_cache_config(cache_config);

    let client = PmcClient::with_config(config);

    // Check initial stats
    let count = client.cache_entry_count();
    assert_eq!(count, 0);

    // Fetch an article
    let _article = client.fetch_full_text("PMC1234567").await.unwrap();

    // Sync cache to ensure all operations are flushed
    client.sync_cache().await;

    // Check stats after fetch
    let count = client.cache_entry_count();
    assert_eq!(count, 1);

    // Fetch another article
    let _article2 = client.fetch_full_text("PMC7654321").await.unwrap();

    // Sync cache to ensure all operations are flushed
    client.sync_cache().await;

    // Check stats again
    let count = client.cache_entry_count();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_pmc_no_cache() {
    let mock_server = MockServer::start().await;

    // Set up mock to track calls
    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PMC_XML_RESPONSE))
        .expect(2) // Should be called twice since caching is disabled
        .mount(&mock_server)
        .await;

    // Create client without caching
    let config = ClientConfig::new().with_base_url(mock_server.uri());

    let client = PmcClient::with_config(config);

    // Fetch same article twice
    let _article1 = client.fetch_full_text("PMC1234567").await.unwrap();
    let _article2 = client.fetch_full_text("PMC1234567").await.unwrap();

    // Both should trigger API calls
    // Cache stats should be 0
    assert_eq!(client.cache_entry_count(), 0);
}

#[tokio::test]
async fn test_pmc_cache_normalization() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_regex(r"/efetch\.fcgi.*"))
        .respond_with(ResponseTemplate::new(200).set_body_string(PMC_XML_RESPONSE))
        .expect(1) // Should only be called once due to ID normalization
        .mount(&mock_server)
        .await;

    // Create client with caching enabled
    let config = ClientConfig::new()
        .with_base_url(mock_server.uri())
        .with_cache_config(CacheConfig::default());

    let client = PmcClient::with_config(config);

    // Fetch with different ID formats
    let article1 = client.fetch_full_text("1234567").await.unwrap();
    let article2 = client.fetch_full_text("PMC1234567").await.unwrap();

    // Should be the same article from cache
    assert_eq!(article1.pmcid, article2.pmcid);

    // Sync cache to ensure all operations are flushed
    client.sync_cache().await;

    // Cache should have only one entry
    let count = client.cache_entry_count();
    assert_eq!(count, 1);
}
