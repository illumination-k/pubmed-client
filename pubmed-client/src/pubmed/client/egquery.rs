//! EGQuery API operations for querying across all NCBI databases

use crate::error::{PubMedError, Result};
use crate::pubmed::models::{DatabaseCount, GlobalQueryResults};
use tracing::{debug, info, instrument};

use super::PubMedClient;

impl PubMedClient {
    /// Query all NCBI databases for record counts using the EGQuery API
    ///
    /// Returns the number of records matching the query in each Entrez database.
    /// Useful for exploratory searches and understanding data distribution across databases.
    ///
    /// # Arguments
    ///
    /// * `term` - Search query string
    ///
    /// # Returns
    ///
    /// Returns a `Result<GlobalQueryResults>` containing counts per database
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let results = client.global_query("asthma").await?;
    ///     println!("Query: {}", results.term);
    ///     for db in results.non_zero() {
    ///         println!("  {}: {} records", db.menu_name, db.count);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn global_query(&self, term: &str) -> Result<GlobalQueryResults> {
        let term = term.trim();
        if term.is_empty() {
            return Err(PubMedError::InvalidQuery(
                "Search term cannot be empty".to_string(),
            ));
        }

        let url = format!(
            "{}/egquery.fcgi?term={}",
            self.base_url,
            urlencoding::encode(term)
        );

        debug!(term = %term, "Making EGQuery API request");
        let response = self.make_request(&url).await?;
        let xml_text = response.text().await?;

        // Parse XML response using string-based extraction (consistent with existing xml_utils)
        let results = Self::parse_egquery_response(&xml_text, term)?;

        info!(
            term = %term,
            database_count = results.results.len(),
            non_zero_count = results.non_zero().len(),
            "EGQuery completed"
        );

        Ok(results)
    }

    /// Parse EGQuery XML response into GlobalQueryResults
    pub(crate) fn parse_egquery_response(
        xml: &str,
        query_term: &str,
    ) -> Result<GlobalQueryResults> {
        use crate::common::xml_utils::{extract_all_text_between, extract_text_between};

        // Extract the term from response, fallback to the query term
        let term = extract_text_between(xml, "<Term>", "</Term>")
            .unwrap_or_else(|| query_term.to_string());

        // Extract all ResultItem blocks
        let result_items = extract_all_text_between(xml, "<ResultItem>", "</ResultItem>");

        let mut results = Vec::new();
        for item in &result_items {
            let db_name = extract_text_between(item, "<DbName>", "</DbName>").unwrap_or_default();
            let menu_name =
                extract_text_between(item, "<MenuName>", "</MenuName>").unwrap_or_default();
            let count_str = extract_text_between(item, "<Count>", "</Count>").unwrap_or_default();
            let status = extract_text_between(item, "<Status>", "</Status>").unwrap_or_default();

            let count = count_str.parse::<u64>().unwrap_or(0);

            results.push(DatabaseCount {
                db_name,
                menu_name,
                count,
                status,
            });
        }

        Ok(GlobalQueryResults { term, results })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_egquery_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Result>
  <Term>asthma</Term>
  <eGQueryResult>
    <ResultItem>
      <DbName>pubmed</DbName>
      <MenuName>PubMed</MenuName>
      <Count>234567</Count>
      <Status>Ok</Status>
    </ResultItem>
    <ResultItem>
      <DbName>pmc</DbName>
      <MenuName>PMC</MenuName>
      <Count>89012</Count>
      <Status>Ok</Status>
    </ResultItem>
    <ResultItem>
      <DbName>mesh</DbName>
      <MenuName>MeSH</MenuName>
      <Count>0</Count>
      <Status>Ok</Status>
    </ResultItem>
  </eGQueryResult>
</Result>"#;

        let result = PubMedClient::parse_egquery_response(xml, "asthma").unwrap();
        assert_eq!(result.term, "asthma");
        assert_eq!(result.results.len(), 3);

        assert_eq!(result.results[0].db_name, "pubmed");
        assert_eq!(result.results[0].menu_name, "PubMed");
        assert_eq!(result.results[0].count, 234567);
        assert_eq!(result.results[0].status, "Ok");

        assert_eq!(result.results[1].db_name, "pmc");
        assert_eq!(result.results[1].count, 89012);

        // Test helper methods
        let non_zero = result.non_zero();
        assert_eq!(non_zero.len(), 2); // pubmed and pmc, not mesh

        assert_eq!(result.count_for("pubmed"), Some(234567));
        assert_eq!(result.count_for("pmc"), Some(89012));
        assert_eq!(result.count_for("mesh"), Some(0));
        assert_eq!(result.count_for("nonexistent"), None);
    }

    #[test]
    fn test_parse_egquery_response_empty() {
        let xml = r#"<Result><Term>test</Term><eGQueryResult></eGQueryResult></Result>"#;
        let result = PubMedClient::parse_egquery_response(xml, "test").unwrap();
        assert_eq!(result.term, "test");
        assert!(result.results.is_empty());
    }

    #[test]
    fn test_parse_egquery_response_error_status() {
        let xml = r#"<Result>
  <Term>test</Term>
  <eGQueryResult>
    <ResultItem>
      <DbName>pubmed</DbName>
      <MenuName>PubMed</MenuName>
      <Count>100</Count>
      <Status>Ok</Status>
    </ResultItem>
    <ResultItem>
      <DbName>snp</DbName>
      <MenuName>SNP</MenuName>
      <Count>0</Count>
      <Status>Term or Database is not found</Status>
    </ResultItem>
  </eGQueryResult>
</Result>"#;
        let result = PubMedClient::parse_egquery_response(xml, "test").unwrap();
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[1].status, "Term or Database is not found");
    }

    #[test]
    fn test_global_query_empty_term() {
        use tokio_test;
        let client = PubMedClient::new();
        let result = tokio_test::block_on(client.global_query(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_global_query_whitespace_term() {
        use tokio_test;
        let client = PubMedClient::new();
        let result = tokio_test::block_on(client.global_query("   "));
        assert!(result.is_err());
    }
}
