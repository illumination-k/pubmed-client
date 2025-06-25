use pubmed_client_rs::{Client, PubMedClient};

#[tokio::test]
async fn test_get_database_list_integration() {
    let client = PubMedClient::new();

    match client.get_database_list().await {
        Ok(databases) => {
            assert!(!databases.is_empty(), "Database list should not be empty");

            // Check for common databases
            assert!(
                databases.contains(&"pubmed".to_string()),
                "Should contain pubmed database"
            );
            assert!(
                databases.contains(&"pmc".to_string()),
                "Should contain pmc database"
            );

            println!("Found {} databases", databases.len());
            println!(
                "First 10 databases: {:?}",
                &databases[..10.min(databases.len())]
            );
        }
        Err(e) => {
            // If we're offline or have rate limiting issues, just warn
            eprintln!("Warning: Could not fetch database list: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_pubmed_database_info_integration() {
    let client = PubMedClient::new();

    match client.get_database_info("pubmed").await {
        Ok(db_info) => {
            assert_eq!(db_info.name, "pubmed");
            assert!(
                !db_info.description.is_empty(),
                "Description should not be empty"
            );
            assert!(!db_info.fields.is_empty(), "Should have search fields");

            // Check for common PubMed fields
            let field_names: Vec<&str> = db_info.fields.iter().map(|f| f.name.as_str()).collect();
            println!(
                "Available fields: {:?}",
                &field_names[..10.min(field_names.len())]
            );
            assert!(field_names.contains(&"TITL"), "Should have title field");
            assert!(field_names.contains(&"FULL"), "Should have author field");

            println!("PubMed database:");
            println!("  Description: {}", db_info.description);
            println!("  Fields: {}", db_info.fields.len());
            println!("  Links: {}", db_info.links.len());

            // Print first few fields
            for field in db_info.fields.iter().take(5) {
                println!("  Field: {} - {}", field.name, field.full_name);
            }
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch PubMed database info: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_pmc_database_info_integration() {
    let client = PubMedClient::new();

    match client.get_database_info("pmc").await {
        Ok(db_info) => {
            assert_eq!(db_info.name, "pmc");
            assert!(
                !db_info.description.is_empty(),
                "Description should not be empty"
            );

            println!("PMC database:");
            println!("  Description: {}", db_info.description);
            println!("  Fields: {}", db_info.fields.len());
            println!("  Links: {}", db_info.links.len());
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch PMC database info: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_invalid_database_info() {
    let client = PubMedClient::new();

    let result = client.get_database_info("nonexistent_database").await;
    assert!(result.is_err(), "Should return error for invalid database");
}

#[tokio::test]
async fn test_get_empty_database_name() {
    let client = PubMedClient::new();

    let result = client.get_database_info("").await;
    assert!(
        result.is_err(),
        "Should return error for empty database name"
    );
}

#[tokio::test]
async fn test_combined_client_einfo() {
    let client = Client::new();

    // Test database list through combined client
    match client.get_database_list().await {
        Ok(databases) => {
            assert!(!databases.is_empty(), "Database list should not be empty");
            println!("Combined client found {} databases", databases.len());
        }
        Err(e) => {
            eprintln!("Warning: Combined client database list failed: {}", e);
        }
    }

    // Test specific database info through combined client
    match client.get_database_info("pubmed").await {
        Ok(db_info) => {
            assert_eq!(db_info.name, "pubmed");
            println!(
                "Combined client got PubMed info with {} fields",
                db_info.fields.len()
            );
        }
        Err(e) => {
            eprintln!("Warning: Combined client database info failed: {}", e);
        }
    }
}
