use pubmed_client_rs::{Client, PubMedClient};

#[tokio::test]
async fn test_get_related_articles_integration() {
    let client = PubMedClient::new();

    // Use a well-known PMID that should have related articles
    let test_pmids = vec![31978945];

    match client.get_related_articles(&test_pmids).await {
        Ok(related) => {
            assert_eq!(related.source_pmids, test_pmids);
            assert_eq!(related.link_type, "pubmed_pubmed");
            println!(
                "Found {} related articles for PMID {}",
                related.related_pmids.len(),
                test_pmids[0]
            );

            // Related PMIDs should not contain the original PMID
            for &pmid in &related.related_pmids {
                assert!(
                    !test_pmids.contains(&pmid),
                    "Related articles should not include source PMID"
                );
            }
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch related articles: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_pmc_links_integration() {
    let client = PubMedClient::new();

    // Use PMIDs that are likely to have PMC full text
    let test_pmids = vec![31978945, 33515491];

    match client.get_pmc_links(&test_pmids).await {
        Ok(pmc_links) => {
            assert_eq!(pmc_links.source_pmids, test_pmids);
            println!(
                "Found {} PMC articles for {} PMIDs",
                pmc_links.pmc_ids.len(),
                test_pmids.len()
            );

            // Print PMC IDs if found
            if !pmc_links.pmc_ids.is_empty() {
                println!(
                    "PMC IDs: {:?}",
                    &pmc_links.pmc_ids[..5.min(pmc_links.pmc_ids.len())]
                );
            }
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch PMC links: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_citations_integration() {
    let client = PubMedClient::new();

    // Use a well-known PMID that should have citing articles
    let test_pmids = vec![31978945];

    match client.get_citations(&test_pmids).await {
        Ok(citations) => {
            assert_eq!(citations.source_pmids, test_pmids);
            assert_eq!(citations.link_type, "pubmed_pubmed_citedin");
            println!(
                "Found {} citing articles for PMID {}",
                citations.citing_pmids.len(),
                test_pmids[0]
            );
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch citations: {}", e);
        }
    }
}

#[tokio::test]
async fn test_empty_pmids_handling() {
    let client = PubMedClient::new();

    // Test empty input handling
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
async fn test_elink_methods_through_combined_client() {
    let client = Client::new();

    let test_pmids = vec![31978945];

    // Test related articles through combined client
    match client.get_related_articles(&test_pmids).await {
        Ok(related) => {
            println!(
                "Combined client: Found {} related articles",
                related.related_pmids.len()
            );
        }
        Err(e) => {
            eprintln!("Warning: Combined client related articles failed: {}", e);
        }
    }

    // Test PMC links through combined client
    match client.get_pmc_links(&test_pmids).await {
        Ok(pmc_links) => {
            println!(
                "Combined client: Found {} PMC links",
                pmc_links.pmc_ids.len()
            );
        }
        Err(e) => {
            eprintln!("Warning: Combined client PMC links failed: {}", e);
        }
    }

    // Test citations through combined client
    match client.get_citations(&test_pmids).await {
        Ok(citations) => {
            println!(
                "Combined client: Found {} citations",
                citations.citing_pmids.len()
            );
        }
        Err(e) => {
            eprintln!("Warning: Combined client citations failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_multiple_pmids_handling() {
    let client = PubMedClient::new();

    // Test with multiple PMIDs
    let multiple_pmids = vec![31978945, 33515491, 32960547];

    match client.get_related_articles(&multiple_pmids).await {
        Ok(related) => {
            assert_eq!(related.source_pmids, multiple_pmids);
            println!(
                "Multiple PMIDs: Found {} related articles for {} source PMIDs",
                related.related_pmids.len(),
                multiple_pmids.len()
            );

            // Ensure no source PMIDs are in the related results
            for &source_pmid in &multiple_pmids {
                assert!(
                    !related.related_pmids.contains(&source_pmid),
                    "Related articles should not contain source PMIDs"
                );
            }
        }
        Err(e) => {
            eprintln!("Warning: Multiple PMIDs related articles failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_elink_deduplication() {
    let client = PubMedClient::new();

    // Test with duplicate PMIDs to ensure deduplication works
    let duplicate_pmids = vec![31978945, 31978945, 31978945];

    match client.get_related_articles(&duplicate_pmids).await {
        Ok(related) => {
            // Source PMIDs should still contain duplicates (as provided)
            assert_eq!(related.source_pmids, duplicate_pmids);

            // Related PMIDs should be deduplicated
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
        Err(e) => {
            eprintln!("Warning: Duplicate PMIDs test failed: {}", e);
        }
    }
}
