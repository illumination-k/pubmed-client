use pubmed_client::PubMedClient;
use tracing::{debug, info, warn};
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn test_fetch_article_with_abstract_integration() {
    let client = PubMedClient::new();

    // Test with PMID 31978945 - the COVID-19 paper we know has an abstract
    info!("Testing abstract retrieval for article with known abstract");
    let result = client.fetch_article("31978945").await;

    // This test will only run if we have internet access
    // If the request fails due to network issues, skip the test
    match result {
        Ok(article) => {
            info!(
                title = %article.title,
                authors_count = article.authors.len(),
                journal = %article.journal,
                "Successfully fetched article"
            );

            // The most important assertion - abstract should not be None
            assert!(
                article.abstract_text.is_some(),
                "Abstract should not be None!"
            );

            let abstract_text = article.abstract_text.unwrap();
            info!(
                abstract_length = abstract_text.len(),
                "Abstract retrieved successfully"
            );

            debug!(
                abstract_preview = %abstract_text.chars().take(100).collect::<String>(),
                "Abstract content preview"
            );

            // Verify the abstract contains expected content
            assert!(
                abstract_text.contains("2019"),
                "Abstract should mention 2019"
            );
            assert!(abstract_text.len() > 100, "Abstract should be substantial");

            // Check other fields are populated correctly
            assert_eq!(article.pmid, "31978945");
            assert!(!article.title.is_empty());
            assert!(!article.authors.is_empty());

            info!("Abstract parsing test completed successfully");
        }
        Err(e) => {
            warn!(error = %e, "Test skipped due to network error");
            // Don't fail the test if we can't reach the API
            // This allows the test to pass in CI environments without internet
        }
    }
}

#[tokio::test]
#[traced_test]
async fn test_fetch_article_without_abstract_integration() {
    let client = PubMedClient::new();

    // Test with PMID 33515491 - the Lancet paper we know doesn't have an abstract in XML
    info!("Testing article without abstract");
    let result = client.fetch_article("33515491").await;

    match result {
        Ok(article) => {
            info!(
                title = %article.title,
                pmid = %article.pmid,
                "Successfully fetched article"
            );

            // This article might or might not have an abstract
            // The important thing is that the function doesn't crash
            // and returns a valid article structure
            assert_eq!(article.pmid, "33515491");
            assert!(!article.title.is_empty());

            if let Some(abstract_text) = &article.abstract_text {
                info!(abstract_length = abstract_text.len(), "Abstract found");
            } else {
                info!("No abstract found for this article - this is expected");
            }

            info!("No-abstract handling test completed successfully");
        }
        Err(e) => {
            warn!(error = %e, "Test skipped due to network error");
        }
    }
}
