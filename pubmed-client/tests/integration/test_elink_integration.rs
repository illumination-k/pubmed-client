use pubmed_client::{Client, PubMedClient};
use tracing::{info, warn};

#[tokio::test]
async fn test_get_related_articles_integration() {
    let client = PubMedClient::new();

    // Use a well-known PMID that should have related articles
    let test_pmids = vec![31978945];

    match client.get_related_articles(&test_pmids).await {
        Ok(related) => {
            assert_eq!(related.source_pmids, test_pmids);
            assert_eq!(related.link_type, "pubmed_pubmed");
            info!(
                related_count = related.related_pmids.len(),
                pmid = test_pmids[0],
                "Found related articles"
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
            warn!(error = %e, "Could not fetch related articles");
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
            info!(
                pmc_count = pmc_links.pmc_ids.len(),
                pmid_count = test_pmids.len(),
                "Found PMC articles"
            );

            // Print PMC IDs if found
            if !pmc_links.pmc_ids.is_empty() {
                info!(
                    pmc_ids = ?&pmc_links.pmc_ids[..5.min(pmc_links.pmc_ids.len())],
                    "Found PMC IDs"
                );
            }
        }
        Err(e) => {
            warn!(error = %e, "Could not fetch PMC links");
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
            info!(
                citing_count = citations.citing_pmids.len(),
                pmid = test_pmids[0],
                "Found citing articles"
            );
        }
        Err(e) => {
            warn!(error = %e, "Could not fetch citations");
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
            info!(
                related_count = related.related_pmids.len(),
                "Combined client found related articles"
            );
        }
        Err(e) => {
            warn!(error = %e, "Combined client related articles failed");
        }
    }

    // Test PMC links through combined client
    match client.get_pmc_links(&test_pmids).await {
        Ok(pmc_links) => {
            info!(
                pmc_count = pmc_links.pmc_ids.len(),
                "Combined client found PMC links"
            );
        }
        Err(e) => {
            warn!(error = %e, "Combined client PMC links failed");
        }
    }

    // Test citations through combined client
    match client.get_citations(&test_pmids).await {
        Ok(citations) => {
            info!(
                citing_count = citations.citing_pmids.len(),
                "Combined client found citations"
            );
        }
        Err(e) => {
            warn!(error = %e, "Combined client citations failed");
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
            info!(
                related_count = related.related_pmids.len(),
                source_count = multiple_pmids.len(),
                "Multiple PMIDs: Found related articles"
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
            warn!(error = %e, "Multiple PMIDs related articles failed");
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
            warn!(error = %e, "Duplicate PMIDs test failed");
        }
    }
}
