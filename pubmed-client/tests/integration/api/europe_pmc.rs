//! Europe PMC REST API integration tests (real network).
//!
//! **IMPORTANT**: These tests are only run when:
//! 1. The `integration-tests` feature is enabled
//! 2. The `PUBMED_REAL_API_TESTS` environment variable is set
//!
//! ```bash
//! PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test api_europe_pmc
//! ```

#[path = "../common/mod.rs"]
mod common;

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use tracing::info;
    use tracing_test::traced_test;

    use pubmed_client::europe_pmc::{EuropePmcId, EuropePmcSource};

    use crate::common::integration_test_utils::{
        create_test_europe_pmc_client, should_run_real_api_tests,
    };

    #[tokio::test]
    #[traced_test]
    async fn test_search_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_europe_pmc_client();
        let results = client
            .search("malaria vaccine", 5)
            .await
            .expect("search should succeed");

        assert!(!results.is_empty(), "expected at least one result");
        assert!(results.len() <= 5);
        assert!(results.iter().all(|r| !r.source.is_empty()));
    }

    #[tokio::test]
    #[traced_test]
    async fn test_fetch_full_text_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_europe_pmc_client();
        // PMC3258128 is the canonical Europe PMC fullTextXML example.
        let id = EuropePmcId::pmc("PMC3258128").unwrap();
        let article = client
            .fetch_full_text(&id)
            .await
            .expect("full text should fetch and parse");
        assert!(article.title().is_some());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_references_and_citations_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_europe_pmc_client();
        let id = EuropePmcId::new(EuropePmcSource::Med, "23245604");

        let references = client
            .get_references(&id)
            .await
            .expect("references should fetch");
        assert!(!references.is_empty(), "expected at least one reference");

        let citations = client
            .get_citations(&id)
            .await
            .expect("citations should fetch");
        // Citation count can legitimately be zero for some articles; just assert
        // the request succeeded and any returned entries are well-formed.
        assert!(
            citations
                .iter()
                .all(|c| c.id.is_some() || c.title.is_some())
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn test_database_links_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_europe_pmc_client();
        let id = EuropePmcId::new(EuropePmcSource::Med, "23245604");
        let links = client
            .get_database_links(&id)
            .await
            .expect("database links should fetch");
        // Some records have no external DB links; assert structure if present.
        assert!(links.iter().all(|l| l.db_name.is_some()));
    }
}
