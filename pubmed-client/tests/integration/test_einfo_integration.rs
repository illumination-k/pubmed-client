use pubmed_client::{Client, PubMedClient};
use tracing::{info, warn};

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

            info!(
                database_count = databases.len(),
                first_databases = ?&databases[..10.min(databases.len())],
                "Found databases"
            );
        }
        Err(e) => {
            // If we're offline or have rate limiting issues, just warn
            warn!(error = %e, "Could not fetch database list");
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
            info!(
                available_fields = ?&field_names[..10.min(field_names.len())],
                "Available fields found"
            );
            assert!(field_names.contains(&"TITL"), "Should have title field");
            assert!(field_names.contains(&"FULL"), "Should have author field");

            info!(
                description = %db_info.description,
                field_count = db_info.fields.len(),
                link_count = db_info.links.len(),
                "PubMed database information"
            );

            // Log first few fields
            for field in db_info.fields.iter().take(5) {
                info!(
                    field_name = %field.name,
                    field_full_name = %field.full_name,
                    "PubMed field"
                );
            }
        }
        Err(e) => {
            warn!(error = %e, "Could not fetch PubMed database info");
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

            info!(
                description = %db_info.description,
                field_count = db_info.fields.len(),
                link_count = db_info.links.len(),
                "PMC database information"
            );
        }
        Err(e) => {
            warn!(error = %e, "Could not fetch PMC database info");
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
            info!(
                database_count = databases.len(),
                "Combined client found databases"
            );
        }
        Err(e) => {
            warn!(error = %e, "Combined client database list failed");
        }
    }

    // Test specific database info through combined client
    match client.get_database_info("pubmed").await {
        Ok(db_info) => {
            assert_eq!(db_info.name, "pubmed");
            info!(
                field_count = db_info.fields.len(),
                "Combined client got PubMed info"
            );
        }
        Err(e) => {
            warn!(error = %e, "Combined client database info failed");
        }
    }
}
