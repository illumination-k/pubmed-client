//! Comprehensive PubMed E-utilities API integration tests
//!
//! These tests make actual network calls to NCBI E-utilities API to verify
//! real-world functionality and catch API changes.
//!
//! **IMPORTANT**: These tests are only run when:
//! 1. The `integration-tests` feature is enabled
//! 2. The `PUBMED_REAL_API_TESTS` environment variable is set
//!
//! To run these tests:
//! ```bash
//! PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pubmed_api_tests
//! ```
//!
//! With API key for higher rate limits:
//! ```bash
//! PUBMED_REAL_API_TESTS=1 NCBI_API_KEY=your_key cargo test --features integration-tests --test pubmed_api_tests
//! ```

#[path = "../common/mod.rs"]
mod common;

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use std::time::{Duration, Instant};
    use tokio::time::sleep;
    use tracing::{debug, info, warn};
    use tracing_test::traced_test;

    use pubmed_client::{ClientConfig, PubMedError, SearchQuery};

    // Import test utilities
    use crate::common::integration_test_utils::{
        create_test_client, create_test_pubmed_client, should_run_real_api_tests, TEST_PMIDS,
        TEST_PMIDS_STR, TEST_SEARCH_QUERIES,
    };

    /// Test basic article search functionality with real API
    #[tokio::test]
    #[traced_test]
    async fn test_search_articles_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PubMed article search integration");

        let client = create_test_pubmed_client();

        for query in TEST_SEARCH_QUERIES.iter().take(3) {
            info!(query = query, "Testing search query");

            let start_time = Instant::now();
            match client.search_articles(query, 10).await {
                Ok(pmids) => {
                    let duration = start_time.elapsed();
                    info!(
                        query = query,
                        results_count = pmids.len(),
                        duration_ms = duration.as_millis(),
                        "Search successful"
                    );

                    assert!(!pmids.is_empty(), "Search should return results");
                    assert!(pmids.len() <= 10, "Should not exceed requested limit");

                    // Verify PMID format
                    for pmid in pmids.iter().take(3) {
                        assert!(
                            pmid.parse::<u32>().is_ok(),
                            "PMID should be numeric: {pmid}"
                        );
                        debug!(pmid = pmid, "Retrieved PMID");
                    }
                }
                Err(e) => {
                    warn!(query = query, error = %e, "Search failed");
                    panic!("Search should succeed for query: {query}");
                }
            }

            // Respectful delay between searches
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Test fetching specific articles by PMID
    #[tokio::test]
    #[traced_test]
    async fn test_fetch_articles_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing PubMed article fetching integration");

        let client = create_test_pubmed_client();

        for pmid in TEST_PMIDS_STR.iter().take(3) {
            info!(pmid = pmid, "Testing article fetch");

            let start_time = Instant::now();
            match client.fetch_article(pmid).await {
                Ok(article) => {
                    let duration = start_time.elapsed();
                    info!(
                        pmid = pmid,
                        title_length = article.title.len(),
                        authors_count = article.authors.len(),
                        has_abstract = article.abstract_text.is_some(),
                        duration_ms = duration.as_millis(),
                        "Article fetch successful"
                    );

                    // Verify required fields
                    assert!(!article.title.is_empty(), "Article should have title");
                    assert!(!article.journal.is_empty(), "Article should have journal");
                    assert!(!article.authors.is_empty(), "Article should have authors");
                    assert!(
                        !article.pub_date.is_empty(),
                        "Article should have publication date"
                    );

                    // Verify PMID matches
                    assert_eq!(article.pmid, *pmid);

                    // Log title preview for verification
                    let title_preview = if article.title.len() > 100 {
                        let preview = &article.title[..100];
                        format!("{preview}...")
                    } else {
                        article.title.clone()
                    };
                    debug!(pmid = pmid, title_preview = %title_preview, "Article title");
                }
                Err(e) => {
                    warn!(pmid = pmid, error = %e, "Article fetch failed");
                    panic!("Fetch should succeed for PMID: {pmid}");
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Test search and fetch workflow (end-to-end)
    #[tokio::test]
    #[traced_test]
    async fn test_search_and_fetch_workflow_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing search and fetch workflow integration");

        let client = create_test_pubmed_client();
        let query = "COVID-19[Title] AND 2023[PDAT]";

        // Step 1: Search for articles
        info!(query = query, "Step 1: Searching for articles");

        let pmids = match client.search_articles(query, 5).await {
            Ok(pmids) => {
                info!(results_count = pmids.len(), "Search completed successfully");
                pmids
            }
            Err(e) => {
                warn!(error = %e, "Search failed");
                panic!("Search should succeed");
            }
        };

        assert!(!pmids.is_empty(), "Search should find articles");

        // Step 2: Fetch detailed article data
        info!("Step 2: Fetching article details");

        let mut articles_fetched = 0;
        let mut articles_with_abstracts = 0;

        for (i, pmid) in pmids.iter().take(3).enumerate() {
            info!(pmid = pmid, article_number = i + 1, "Fetching article");

            match client.fetch_article(pmid).await {
                Ok(article) => {
                    articles_fetched += 1;

                    // Verify data quality
                    assert!(!article.title.is_empty(), "Article should have title");
                    assert!(!article.authors.is_empty(), "Article should have authors");

                    if article.abstract_text.is_some() {
                        articles_with_abstracts += 1;
                    }

                    debug!(
                        pmid = pmid,
                        title_length = article.title.len(),
                        authors_count = article.authors.len(),
                        "Article details verified"
                    );
                }
                Err(e) => {
                    warn!(pmid = pmid, error = %e, "Failed to fetch article");
                }
            }

            sleep(Duration::from_millis(100)).await;
        }

        info!(
            total_pmids = pmids.len(),
            articles_fetched = articles_fetched,
            articles_with_abstracts = articles_with_abstracts,
            "Workflow completed"
        );

        assert!(articles_fetched > 0, "Should fetch at least one article");
    }

    /// Test ELink API functionality for related articles
    #[tokio::test]
    #[traced_test]
    async fn test_elink_related_articles_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing ELink related articles integration");

        let client = create_test_pubmed_client();
        let test_pmids = &TEST_PMIDS[0..2]; // Use first 2 PMIDs

        match client.get_related_articles(test_pmids).await {
            Ok(related) => {
                info!(
                    input_pmids = test_pmids.len(),
                    related_articles_count = related.related_pmids.len(),
                    "Related articles retrieved successfully"
                );

                // Verify results
                assert!(
                    !related.related_pmids.is_empty(),
                    "Should find related articles"
                );

                // Verify PMIDs are valid
                for pmid in related.related_pmids.iter().take(5) {
                    assert!(*pmid > 0, "Related PMID should be positive: {pmid}");
                }

                debug!(
                    first_few_related = ?related.related_pmids.iter().take(3).collect::<Vec<_>>(),
                    "Sample related PMIDs"
                );
            }
            Err(e) => {
                warn!(error = %e, "ELink related articles failed");
                panic!("ELink should succeed for known PMIDs");
            }
        }
    }

    /// Test ELink API functionality for PMC links
    #[tokio::test]
    #[traced_test]
    async fn test_elink_pmc_links_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing ELink PMC links integration");

        let client = create_test_pubmed_client();
        let test_pmids = &TEST_PMIDS[0..3]; // Use first 3 PMIDs

        match client.get_pmc_links(test_pmids).await {
            Ok(pmc_links) => {
                info!(
                    input_pmids = test_pmids.len(),
                    pmc_links_count = pmc_links.pmc_ids.len(),
                    "PMC links retrieved successfully"
                );

                // PMC links might be empty if articles don't have full text
                if !pmc_links.pmc_ids.is_empty() {
                    for pmc_id in pmc_links.pmc_ids.iter().take(3) {
                        assert!(
                            pmc_id.starts_with("PMC"),
                            "PMC ID should start with PMC: {pmc_id}"
                        );
                    }

                    debug!(
                        pmc_ids = ?pmc_links.pmc_ids.iter().take(3).collect::<Vec<_>>(),
                        "Sample PMC IDs"
                    );
                } else {
                    info!("No PMC links found for test PMIDs (expected for some articles)");
                }
            }
            Err(e) => {
                warn!(error = %e, "ELink PMC links failed");
                panic!("ELink should succeed even if no PMC links found");
            }
        }
    }

    /// Test ELink API functionality for citations
    #[tokio::test]
    #[traced_test]
    async fn test_elink_citations_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing ELink citations integration");

        let client = create_test_pubmed_client();
        let test_pmids = &TEST_PMIDS[0..2]; // Use first 2 PMIDs

        match client.get_citations(test_pmids).await {
            Ok(citations) => {
                info!(
                    input_pmids = test_pmids.len(),
                    citations_count = citations.citing_pmids.len(),
                    "Citations retrieved successfully"
                );

                // Citations might be empty for newer articles
                if !citations.citing_pmids.is_empty() {
                    for pmid in citations.citing_pmids.iter().take(5) {
                        assert!(*pmid > 0, "Citation PMID should be positive: {pmid}");
                    }

                    debug!(
                        citing_pmids = ?citations.citing_pmids.iter().take(3).collect::<Vec<_>>(),
                        "Sample citing PMIDs"
                    );
                } else {
                    info!("No citations found for test PMIDs (expected for newer articles)");
                }
            }
            Err(e) => {
                warn!(error = %e, "ELink citations failed");
                panic!("ELink should succeed even if no citations found");
            }
        }
    }

    /// Test query builder with advanced search functionality
    #[tokio::test]
    #[traced_test]
    async fn test_advanced_search_query_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing advanced search query integration");

        let client = create_test_pubmed_client();

        // Test complex query with date range and filters
        let query = SearchQuery::new()
            .mesh_term("CRISPR")
            .title_contains("gene editing")
            .date_range(2020, Some(2023))
            .build();

        info!(query = %query, "Testing advanced query");

        match client.search_articles(&query, 15).await {
            Ok(pmids) => {
                info!(
                    query = %query,
                    results_count = pmids.len(),
                    "Advanced search successful"
                );

                assert!(!pmids.is_empty(), "Advanced search should return results");
                assert!(pmids.len() <= 15, "Should not exceed requested limit");

                // Fetch one article to verify search quality
                if let Ok(article) = client.fetch_article(&pmids[0]).await {
                    let title_lower = article.title.to_lowercase();
                    let abstract_lower = article
                        .abstract_text
                        .as_ref()
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default();

                    let has_crispr =
                        title_lower.contains("crispr") || abstract_lower.contains("crispr");

                    let has_gene_editing = title_lower.contains("gene editing")
                        || title_lower.contains("editing")
                        || abstract_lower.contains("gene editing")
                        || abstract_lower.contains("editing");

                    info!(
                        pmid = pmids[0],
                        has_crispr = has_crispr,
                        has_gene_editing = has_gene_editing,
                        "Search result verification"
                    );

                    // At least one of the terms should match (search might be fuzzy)
                    assert!(
                        has_crispr || has_gene_editing,
                        "Search result should match query terms"
                    );
                }
            }
            Err(e) => {
                warn!(query = %query, error = %e, "Advanced search failed");
                panic!("Advanced search should succeed");
            }
        }
    }

    /// Test performance and rate limiting under load
    #[tokio::test]
    #[traced_test]
    async fn test_performance_and_rate_limiting_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing performance and rate limiting integration");

        let client = create_test_pubmed_client();
        let start_time = Instant::now();

        // Make several requests to test rate limiting
        let mut successful_requests = 0;
        let mut rate_limited_requests = 0;

        for i in 0..8 {
            let query = format!("test query {i}");

            match client.search_articles(&query, 3).await {
                Ok(pmids) => {
                    successful_requests += 1;
                    debug!(
                        request_number = i + 1,
                        results_count = pmids.len(),
                        "Request successful"
                    );
                }
                Err(PubMedError::RateLimitExceeded) => {
                    rate_limited_requests += 1;
                    debug!(request_number = i + 1, "Rate limit enforced by client");
                }
                Err(e) => {
                    warn!(request_number = i + 1, error = %e, "Request failed");
                }
            }
        }

        let total_duration = start_time.elapsed();

        info!(
            total_requests = 8,
            successful_requests = successful_requests,
            rate_limited_requests = rate_limited_requests,
            total_duration_ms = total_duration.as_millis(),
            "Performance test completed"
        );

        // Verify rate limiting is working (should take reasonable time)
        let min_expected_duration = Duration::from_millis(2000); // At least 2 seconds for 8 requests
        assert!(
            total_duration >= min_expected_duration,
            "Requests completed too quickly - rate limiting may not be working"
        );

        // Most requests should succeed
        assert!(
            successful_requests >= 6,
            "Most requests should succeed with proper rate limiting"
        );
    }

    /// Test MeSH term extraction functionality
    #[tokio::test]
    #[traced_test]
    async fn test_mesh_terms_integration() {
        if !should_run_real_api_tests() {
            info!(
                "Skipping real API test - enable with PUBMED_REAL_API_TESTS=1 and --features integration-tests"
            );
            return;
        }

        info!("Testing MeSH terms integration");

        let client = create_test_pubmed_client();

        // Use known PMID that should have MeSH terms
        let pmid = TEST_PMIDS_STR[0];

        match client.fetch_article(pmid).await {
            Ok(article) => {
                let mesh_terms = article.get_all_mesh_terms();
                info!(
                    pmid = pmid,
                    mesh_count = mesh_terms.len(),
                    "Article with MeSH terms retrieved"
                );

                if !mesh_terms.is_empty() {
                    // Verify MeSH term structure
                    for (i, mesh_term) in mesh_terms.iter().take(3).enumerate() {
                        assert!(!mesh_term.is_empty(), "MeSH descriptor should not be empty");

                        debug!(
                            mesh_index = i,
                            descriptor = %mesh_term,
                            "MeSH term details"
                        );
                    }

                    info!(
                        sample_mesh_terms = ?mesh_terms.iter().take(3).collect::<Vec<_>>(),
                        "Sample MeSH terms"
                    );
                } else {
                    info!("No MeSH terms found for this article (possible for some articles)");
                }
            }
            Err(e) => {
                warn!(pmid = pmid, error = %e, "Failed to fetch article for MeSH test");
                panic!("Should be able to fetch article for MeSH test");
            }
        }
    }

    // ==================================================================================
    // Abstract fetch tests (from test_integration_abstract.rs)
    // ==================================================================================

    #[tokio::test]
    #[traced_test]
    async fn test_fetch_article_with_abstract_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        info!("Testing abstract retrieval for article with known abstract");
        let article = client.fetch_article("31978945").await.unwrap();

        assert!(
            article.abstract_text.is_some(),
            "Abstract should not be None!"
        );

        let abstract_text = article.abstract_text.unwrap();
        info!(
            abstract_length = abstract_text.len(),
            "Abstract retrieved successfully"
        );

        assert!(
            abstract_text.contains("2019"),
            "Abstract should mention 2019"
        );
        assert!(abstract_text.len() > 100, "Abstract should be substantial");
        assert_eq!(article.pmid, "31978945");
        assert!(!article.title.is_empty());
        assert!(!article.authors.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_fetch_article_without_abstract_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        info!("Testing article without abstract");
        let article = client.fetch_article("33515491").await.unwrap();

        assert_eq!(article.pmid, "33515491");
        assert!(!article.title.is_empty());

        if let Some(abstract_text) = &article.abstract_text {
            info!(abstract_length = abstract_text.len(), "Abstract found");
        } else {
            info!("No abstract found for this article - this is expected");
        }
    }

    // ==================================================================================
    // EInfo API tests (from test_einfo_integration.rs)
    // ==================================================================================

    #[tokio::test]
    #[traced_test]
    async fn test_get_database_list_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        let databases = client.get_database_list().await.unwrap();
        assert!(!databases.is_empty(), "Database list should not be empty");
        assert!(
            databases.contains(&"pubmed".to_string()),
            "Should contain pubmed database"
        );
        assert!(
            databases.contains(&"pmc".to_string()),
            "Should contain pmc database"
        );

        info!(
            database_count = databases.len(),
            first_databases = ?&databases[..10.min(databases.len())],
            "Found databases"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_pubmed_database_info_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        let db_info = client.get_database_info("pubmed").await.unwrap();
        assert_eq!(db_info.name, "pubmed");
        assert!(
            !db_info.description.is_empty(),
            "Description should not be empty"
        );
        assert!(!db_info.fields.is_empty(), "Should have search fields");

        let field_names: Vec<&str> = db_info.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(field_names.contains(&"TITL"), "Should have title field");
        assert!(field_names.contains(&"FULL"), "Should have author field");

        info!(
            description = %db_info.description,
            field_count = db_info.fields.len(),
            link_count = db_info.links.len(),
            "PubMed database information"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_pmc_database_info_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();

        let db_info = client.get_database_info("pmc").await.unwrap();
        assert_eq!(db_info.name, "pmc");
        assert!(
            !db_info.description.is_empty(),
            "Description should not be empty"
        );

        info!(
            description = %db_info.description,
            field_count = db_info.fields.len(),
            link_count = db_info.links.len(),
            "PMC database information"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn test_combined_client_einfo() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_client();

        let databases = client.get_database_list().await.unwrap();
        assert!(!databases.is_empty(), "Database list should not be empty");
        info!(
            database_count = databases.len(),
            "Combined client found databases"
        );

        let db_info = client.get_database_info("pubmed").await.unwrap();
        assert_eq!(db_info.name, "pubmed");
        info!(
            field_count = db_info.fields.len(),
            "Combined client got PubMed info"
        );
    }

    // ==================================================================================
    // ELink API tests (from test_elink_integration.rs)
    // ==================================================================================

    #[tokio::test]
    #[traced_test]
    async fn test_get_related_articles_direct_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let test_pmids = vec![31978945];

        let related = client.get_related_articles(&test_pmids).await.unwrap();
        assert_eq!(related.source_pmids, test_pmids);
        assert_eq!(related.link_type, "pubmed_pubmed");
        info!(
            related_count = related.related_pmids.len(),
            pmid = test_pmids[0],
            "Found related articles"
        );

        for &pmid in &related.related_pmids {
            assert!(
                !test_pmids.contains(&pmid),
                "Related articles should not include source PMID"
            );
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_pmc_links_direct_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let test_pmids = vec![31978945, 33515491];

        let pmc_links = client.get_pmc_links(&test_pmids).await.unwrap();
        assert_eq!(pmc_links.source_pmids, test_pmids);
        info!(
            pmc_count = pmc_links.pmc_ids.len(),
            pmid_count = test_pmids.len(),
            "Found PMC articles"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn test_get_citations_direct_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let test_pmids = vec![31978945];

        let citations = client.get_citations(&test_pmids).await.unwrap();
        assert_eq!(citations.source_pmids, test_pmids);
        assert_eq!(citations.link_type, "pubmed_pubmed_citedin");
        info!(
            citing_count = citations.citing_pmids.len(),
            pmid = test_pmids[0],
            "Found citing articles"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn test_empty_pmids_handling() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let empty_pmids: Vec<u32> = vec![];

        let related = client.get_related_articles(&empty_pmids).await.unwrap();
        assert!(related.source_pmids.is_empty());
        assert!(related.related_pmids.is_empty());
        assert_eq!(related.link_type, "pubmed_pubmed");

        let pmc_links = client.get_pmc_links(&empty_pmids).await.unwrap();
        assert!(pmc_links.source_pmids.is_empty());
        assert!(pmc_links.pmc_ids.is_empty());

        let citations = client.get_citations(&empty_pmids).await.unwrap();
        assert!(citations.source_pmids.is_empty());
        assert!(citations.citing_pmids.is_empty());
        assert_eq!(citations.link_type, "pubmed_pubmed_citedin");
    }

    #[tokio::test]
    #[traced_test]
    async fn test_elink_methods_through_combined_client() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_client();
        let test_pmids = vec![31978945];

        let related = client.get_related_articles(&test_pmids).await.unwrap();
        info!(
            related_count = related.related_pmids.len(),
            "Combined client found related articles"
        );

        let pmc_links = client.get_pmc_links(&test_pmids).await.unwrap();
        info!(
            pmc_count = pmc_links.pmc_ids.len(),
            "Combined client found PMC links"
        );

        let citations = client.get_citations(&test_pmids).await.unwrap();
        info!(
            citing_count = citations.citing_pmids.len(),
            "Combined client found citations"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn test_multiple_pmids_handling() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let multiple_pmids = vec![31978945, 33515491, 32960547];

        let related = client.get_related_articles(&multiple_pmids).await.unwrap();
        assert_eq!(related.source_pmids, multiple_pmids);
        info!(
            related_count = related.related_pmids.len(),
            source_count = multiple_pmids.len(),
            "Multiple PMIDs: Found related articles"
        );

        for &source_pmid in &multiple_pmids {
            assert!(
                !related.related_pmids.contains(&source_pmid),
                "Related articles should not contain source PMIDs"
            );
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_elink_deduplication() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let client = create_test_pubmed_client();
        let duplicate_pmids = vec![31978945, 31978945, 31978945];

        let related = client.get_related_articles(&duplicate_pmids).await.unwrap();
        assert_eq!(related.source_pmids, duplicate_pmids);

        let mut sorted_related = related.related_pmids.clone();
        sorted_related.sort_unstable();
        let original_len = sorted_related.len();
        sorted_related.dedup();
        assert_eq!(
            original_len,
            sorted_related.len(),
            "Related PMIDs should already be deduplicated"
        );
    }

    // ==================================================================================
    // MeSH search tests (from test_mesh_integration.rs)
    // ==================================================================================

    #[tokio::test]
    #[traced_test]
    async fn test_mesh_search_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let config = ClientConfig::new().with_rate_limit(1.0);
        let client = pubmed_client::PubMedClient::with_config(config);

        let articles = SearchQuery::new()
            .mesh_major_topic("COVID-19")
            .mesh_subheading("prevention & control")
            .published_after(2023)
            .limit(5)
            .search_and_fetch(&client)
            .await
            .unwrap();

        assert!(!articles.is_empty());

        for article in &articles {
            info!(pmid = %article.pmid, title = %article.title, "Found article");
            if article.mesh_headings.is_some() {
                let mesh_terms = article.get_all_mesh_terms();
                info!(mesh_terms = %mesh_terms.join(", "), "Article MeSH terms");

                let major_terms = article.get_major_mesh_terms();
                info!(major_topics = %major_terms.join(", "), "Major topics");
            }
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn test_chemical_search_integration() {
        if !should_run_real_api_tests() {
            info!("Skipping real API test - enable with PUBMED_REAL_API_TESTS=1");
            return;
        }

        let config = ClientConfig::new().with_rate_limit(1.0);
        let client = pubmed_client::PubMedClient::with_config(config);

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
            info!(pmid = %article.pmid, title = %article.title, "Found article");

            let chemicals = article.get_chemical_names();
            if !chemicals.is_empty() {
                info!(chemicals = %chemicals.join(", "), "Article chemicals");
            }

            let qualifiers = article.get_mesh_qualifiers("Diabetes Mellitus, Type 2");
            if !qualifiers.is_empty() {
                info!(diabetes_qualifiers = %qualifiers.join(", "), "Diabetes qualifiers");
            }
        }
    }
}

// Placeholder module for non-integration builds
#[cfg(not(feature = "integration-tests"))]
mod placeholder {
    //! Integration tests are only available with the `integration-tests` feature.
    //!
    //! To run these tests:
    //! ```bash
    //! cargo test --features integration-tests --test pubmed_api_tests
    //! ```
}
