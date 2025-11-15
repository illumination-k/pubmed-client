//! Parser coverage test - fetches 100+ articles and validates parser robustness

use pubmed_client::{ClientConfig, PubMedClient};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug)]
struct ParserTestResult {
    pmid: String,
    success: bool,
    error_message: Option<String>,
    title: Option<String>,
}

/// Test parser coverage with diverse queries to find edge cases
#[tokio::test]
#[ignore] // Run explicitly with: cargo test test_parser_coverage -- --ignored --nocapture
async fn test_parser_coverage() {
    // Create client with rate limiting
    let config = ClientConfig::new()
        .with_email("test@example.com")
        .with_tool("pubmed-client-parser-coverage-test")
        .with_rate_limit(2.0); // Conservative rate limiting

    let client = PubMedClient::with_config(config);

    // Diverse search queries to get variety of article types
    let queries = vec![
        "COVID-19[ti] AND 2020[pdat]",                    // Recent pandemic research
        "cancer therapy[tiab]",                            // Cancer research
        "CRISPR[ti]",                                      // Gene editing
        "machine learning[ti] AND medicine[tiab]",         // AI in medicine
        "vaccine[ti] AND clinical trial[pt]",              // Clinical trials
        "diabetes[ti] AND 2015:2020[pdat]",                // Chronic disease
        "microbiome[ti]",                                  // Microbiome research
        "Alzheimer[ti]",                                   // Neurology
        "heart failure[ti]",                               // Cardiology
        "immunotherapy[ti] AND cancer[tiab]",              // Immunotherapy
        "RNA[ti] AND sequencing[tiab]",                    // Genomics
        "artificial intelligence[ti] AND radiology[tiab]", // Medical imaging
        "stem cell[ti]",                                   // Regenerative medicine
        "antibiotic resistance[ti]",                       // Infectious disease
        "mental health[ti] AND adolescents[tiab]",         // Psychiatry
    ];

    let mut all_results = Vec::new();
    let mut pmids_to_test: Vec<String> = Vec::new();

    println!("=== Collecting PMIDs from diverse queries ===\n");

    // Collect PMIDs from each query
    for query in &queries {
        println!("Searching: {}", query);
        match client.search_articles(query, 10).await {
            Ok(pmids) => {
                let count = pmids.len();
                pmids_to_test.extend(pmids);
                println!("  ✓ Found {} articles", count);
            }
            Err(e) => {
                println!("  ✗ Search failed: {}", e);
            }
        }
        sleep(Duration::from_millis(350)).await; // Rate limiting
    }

    // Deduplicate PMIDs
    pmids_to_test.sort();
    pmids_to_test.dedup();

    let total_pmids = pmids_to_test.len();
    println!("\n=== Total unique PMIDs collected: {} ===\n", total_pmids);

    println!("=== Testing parser on each article ===\n");

    // Test parser on each PMID
    for (idx, pmid) in pmids_to_test.iter().enumerate() {
        print!("[{:3}/{}] Testing PMID {}... ", idx + 1, total_pmids, pmid);

        match client.fetch_article(pmid).await {
            Ok(article) => {
                // Parser succeeded
                let title = article.title.clone();
                println!("✓ Success");
                all_results.push(ParserTestResult {
                    pmid: pmid.clone(),
                    success: true,
                    error_message: None,
                    title: Some(title),
                });
            }
            Err(e) => {
                // Parser or fetch failed
                println!("✗ Error: {}", e);
                all_results.push(ParserTestResult {
                    pmid: pmid.clone(),
                    success: false,
                    error_message: Some(e.to_string()),
                    title: None,
                });
            }
        }

        // Rate limiting
        if (idx + 1) % 3 == 0 {
            sleep(Duration::from_millis(1000)).await;
        } else {
            sleep(Duration::from_millis(350)).await;
        }
    }

    // Analyze results
    println!("\n=== Parser Coverage Test Results ===\n");

    let successful = all_results.iter().filter(|r| r.success).count();
    let failed = all_results.iter().filter(|r| !r.success).count();
    let success_rate = (successful as f64 / total_pmids as f64) * 100.0;

    println!("Total articles tested: {}", total_pmids);
    println!("Successfully parsed: {}", successful);
    println!("Failed to parse: {}", failed);
    println!("Success rate: {:.2}%", success_rate);

    // Group errors by type
    if failed > 0 {
        println!("\n=== Failed Articles ===\n");

        let mut error_groups: HashMap<String, Vec<String>> = HashMap::new();

        for result in all_results.iter().filter(|r| !r.success) {
            let error_key = result
                .error_message
                .as_ref()
                .map(|e| {
                    // Extract error type from message
                    if e.contains("XML") {
                        "XML parsing error".to_string()
                    } else if e.contains("No article") {
                        "No article returned".to_string()
                    } else if e.contains("Network") || e.contains("HTTP") {
                        "Network error".to_string()
                    } else {
                        e.lines().next().unwrap_or("Unknown error").to_string()
                    }
                })
                .unwrap_or_else(|| "Unknown error".to_string());

            error_groups
                .entry(error_key)
                .or_insert_with(Vec::new)
                .push(result.pmid.clone());
        }

        for (error_type, pmids) in error_groups.iter() {
            println!("Error type: {}", error_type);
            println!("  Count: {}", pmids.len());
            println!("  PMIDs: {}", pmids.join(", "));
            println!();
        }
    }

    // List some successful parses
    println!("=== Sample Successful Parses ===\n");
    for result in all_results.iter().filter(|r| r.success).take(10) {
        if let Some(title) = &result.title {
            println!("PMID {}: {}", result.pmid, title);
        }
    }

    // Assert reasonable success rate
    assert!(
        success_rate >= 90.0,
        "Parser success rate ({:.2}%) is below acceptable threshold (90%)",
        success_rate
    );
}
