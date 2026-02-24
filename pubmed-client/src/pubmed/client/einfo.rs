//! EInfo API operations for retrieving NCBI database information

use crate::error::{PubMedError, Result};
use crate::pubmed::models::{DatabaseInfo, FieldInfo, LinkInfo};
use crate::pubmed::responses::EInfoResponse;
use tracing::{debug, info, instrument};

use super::PubMedClient;

impl PubMedClient {
    /// Get list of all available NCBI databases
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<String>>` containing names of all available databases
    ///
    /// # Errors
    ///
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let databases = client.get_database_list().await?;
    ///     println!("Available databases: {:?}", databases);
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn get_database_list(&self) -> Result<Vec<String>> {
        // Build URL - API parameters will be added by make_request
        let url = format!("{}/einfo.fcgi?retmode=json", self.base_url);

        debug!("Making EInfo API request for database list");
        let response = self.make_request(&url).await?;

        let einfo_response: EInfoResponse = response.json().await?;

        let db_list = einfo_response.einfo_result.db_list.unwrap_or_default();

        info!(
            databases_found = db_list.len(),
            "Database list retrieved successfully"
        );

        Ok(db_list)
    }

    /// Get detailed information about a specific database
    ///
    /// # Arguments
    ///
    /// * `database` - Name of the database (e.g., "pubmed", "pmc", "books")
    ///
    /// # Returns
    ///
    /// Returns a `Result<DatabaseInfo>` containing detailed database information
    ///
    /// # Errors
    ///
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::JsonError` - If JSON parsing fails
    /// * `PubMedError::ApiError` - If the database doesn't exist
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let db_info = client.get_database_info("pubmed").await?;
    ///     println!("Database: {}", db_info.name);
    ///     println!("Description: {}", db_info.description);
    ///     println!("Fields: {}", db_info.fields.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(database = %database))]
    pub async fn get_database_info(&self, database: &str) -> Result<DatabaseInfo> {
        if database.trim().is_empty() {
            return Err(PubMedError::ApiError {
                status: 400,
                message: "Database name cannot be empty".to_string(),
            });
        }

        // Build URL - API parameters will be added by make_request
        let url = format!(
            "{}/einfo.fcgi?db={}&retmode=json",
            self.base_url,
            urlencoding::encode(database)
        );

        debug!("Making EInfo API request for database details");
        let response = self.make_request(&url).await?;

        let einfo_response: EInfoResponse = response.json().await?;

        let db_info_list =
            einfo_response
                .einfo_result
                .db_info
                .ok_or_else(|| PubMedError::ApiError {
                    status: 404,
                    message: format!("Database '{database}' not found or no information available"),
                })?;

        let db_info = db_info_list
            .into_iter()
            .next()
            .ok_or_else(|| PubMedError::ApiError {
                status: 404,
                message: format!("Database '{database}' information not found"),
            })?;

        // Convert internal response to public model
        let fields = db_info
            .field_list
            .unwrap_or_default()
            .into_iter()
            .map(|field| FieldInfo {
                name: field.name,
                full_name: field.full_name,
                description: field.description,
                term_count: field.term_count.and_then(|s| s.parse().ok()),
                is_date: field.is_date.as_deref() == Some("Y"),
                is_numerical: field.is_numerical.as_deref() == Some("Y"),
                single_token: field.single_token.as_deref() == Some("Y"),
                hierarchy: field.hierarchy.as_deref() == Some("Y"),
                is_hidden: field.is_hidden.as_deref() == Some("Y"),
            })
            .collect();

        let links = db_info
            .link_list
            .unwrap_or_default()
            .into_iter()
            .map(|link| LinkInfo {
                name: link.name,
                menu: link.menu,
                description: link.description,
                target_db: link.db_to,
            })
            .collect();

        let database_info = DatabaseInfo {
            name: db_info.db_name,
            menu_name: db_info.menu_name,
            description: db_info.description,
            build: db_info.db_build,
            count: db_info.count.and_then(|s| s.parse().ok()),
            last_update: db_info.last_update,
            fields,
            links,
        };

        info!(
            fields_count = database_info.fields.len(),
            links_count = database_info.links.len(),
            "Database information retrieved successfully"
        );

        Ok(database_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ClientConfig;

    #[test]
    fn test_empty_database_name_validation() {
        use tokio_test;

        let config = ClientConfig::new();
        let client = PubMedClient::with_config(config);

        let result = tokio_test::block_on(client.get_database_info(""));
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }

    #[test]
    fn test_whitespace_database_name_validation() {
        use tokio_test;

        let config = ClientConfig::new();
        let client = PubMedClient::with_config(config);

        let result = tokio_test::block_on(client.get_database_info("   "));
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }
}
