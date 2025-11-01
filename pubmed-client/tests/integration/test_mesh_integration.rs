use pubmed_client::pubmed::SearchQuery;
use pubmed_client::{ClientConfig, PubMedClient};
use tracing::info;

#[tokio::test]
#[ignore] // This is an integration test that requires network access
async fn test_mesh_search_integration() {
    // Create client with rate limiting for testing
    let config = ClientConfig::new().with_rate_limit(1.0);
    let client = PubMedClient::with_config(config);

    // Search for articles with specific MeSH terms
    let articles = SearchQuery::new()
        .mesh_major_topic("COVID-19")
        .mesh_subheading("prevention & control")
        .published_after(2023)
        .limit(5)
        .search_and_fetch(&client)
        .await
        .unwrap();

    assert!(!articles.is_empty());

    // Verify that fetched articles have MeSH terms
    for article in &articles {
        info!(
            pmid = %article.pmid,
            title = %article.title,
            "Found article"
        );
        if let Some(_mesh_headings) = &article.mesh_headings {
            let mesh_terms = article.get_all_mesh_terms();
            info!(
                mesh_terms = %mesh_terms.join(", "),
                "Article MeSH terms"
            );

            // Check if COVID-19 is a major topic
            let major_terms = article.get_major_mesh_terms();
            info!(
                major_topics = %major_terms.join(", "),
                "Major topics"
            );
        }
    }
}

#[tokio::test]
#[ignore] // This is an integration test that requires network access
async fn test_chemical_search_integration() {
    let config = ClientConfig::new().with_rate_limit(1.0);
    let client = PubMedClient::with_config(config);

    // Search for articles about metformin
    let articles = SearchQuery::new()
        .mesh_term("Metformin")
        .mesh_major_topic("Diabetes Mellitus, Type 2")
        .published_after(2022)
        .limit(3)
        .search_and_fetch(&client)
        .await
        .unwrap();

    assert!(!articles.is_empty());

    for article in &articles {
        info!(
            pmid = %article.pmid,
            title = %article.title,
            "Found article"
        );

        // Check chemicals
        let chemicals = article.get_chemical_names();
        if !chemicals.is_empty() {
            info!(
                chemicals = %chemicals.join(", "),
                "Article chemicals"
            );
        }

        // Check MeSH qualifiers for diabetes
        let qualifiers = article.get_mesh_qualifiers("Diabetes Mellitus, Type 2");
        if !qualifiers.is_empty() {
            info!(
                diabetes_qualifiers = %qualifiers.join(", "),
                "Diabetes qualifiers"
            );
        }
    }
}
