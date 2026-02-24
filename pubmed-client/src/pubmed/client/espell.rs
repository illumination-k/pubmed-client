//! ESpell API operations for spell-checking search terms

use crate::error::{PubMedError, Result};
use crate::pubmed::models::{SpellCheckResult, SpelledQuerySegment};
use tracing::{debug, info, instrument};

use super::PubMedClient;

impl PubMedClient {
    /// Check spelling of a search term using the ESpell API
    ///
    /// Provides spelling suggestions for terms within a single text query.
    /// Useful as a preprocessing step before executing actual searches to improve
    /// search accuracy.
    ///
    /// # Arguments
    ///
    /// * `term` - The search term to spell-check
    ///
    /// # Returns
    ///
    /// Returns a `Result<SpellCheckResult>` containing the original query,
    /// corrected query, and detailed information about which terms were corrected.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let result = client.spell_check("asthmaa OR alergies").await?;
    ///
    ///     println!("Original: {}", result.query);
    ///     println!("Corrected: {}", result.corrected_query);
    ///
    ///     if result.has_corrections() {
    ///         println!("Replacements: {:?}", result.replacements());
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(term = %term))]
    pub async fn spell_check(&self, term: &str) -> Result<SpellCheckResult> {
        self.spell_check_db(term, "pubmed").await
    }

    /// Check spelling of a search term against a specific database using the ESpell API
    ///
    /// Spelling suggestions are database-specific, so use the same database you plan to search.
    ///
    /// # Arguments
    ///
    /// * `term` - The search term to spell-check
    /// * `db` - The NCBI database to check against (e.g., "pubmed", "pmc")
    ///
    /// # Returns
    ///
    /// Returns a `Result<SpellCheckResult>` containing spelling suggestions
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let result = client.spell_check_db("fiberblast cell grwth", "pmc").await?;
    ///     println!("Corrected: {}", result.corrected_query);
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(term = %term, db = %db))]
    pub async fn spell_check_db(&self, term: &str, db: &str) -> Result<SpellCheckResult> {
        let term = term.trim();
        if term.is_empty() {
            return Err(PubMedError::InvalidQuery(
                "Search term cannot be empty".to_string(),
            ));
        }

        let db = db.trim();
        if db.is_empty() {
            return Err(PubMedError::ApiError {
                status: 400,
                message: "Database name cannot be empty".to_string(),
            });
        }

        let url = format!(
            "{}/espell.fcgi?db={}&term={}",
            self.base_url,
            urlencoding::encode(db),
            urlencoding::encode(term)
        );

        debug!(term = %term, db = %db, "Making ESpell API request");
        let response = self.make_request(&url).await?;
        let xml_text = response.text().await?;

        let result = Self::parse_espell_response(&xml_text, term, db)?;

        info!(
            term = %term,
            corrected = %result.corrected_query,
            has_corrections = result.has_corrections(),
            "ESpell completed"
        );

        Ok(result)
    }

    /// Parse ESpell XML response into SpellCheckResult
    pub(crate) fn parse_espell_response(
        xml: &str,
        query_term: &str,
        db: &str,
    ) -> Result<SpellCheckResult> {
        use crate::common::xml_utils::extract_text_between;

        // Check for error
        let error = extract_text_between(xml, "<ERROR>", "</ERROR>");
        if let Some(error_msg) = error {
            if !error_msg.is_empty() {
                return Err(PubMedError::ApiError {
                    status: 200,
                    message: format!("NCBI ESpell API error: {}", error_msg),
                });
            }
        }

        let database = extract_text_between(xml, "<Database>", "</Database>")
            .unwrap_or_else(|| db.to_string());

        let query = extract_text_between(xml, "<Query>", "</Query>")
            .unwrap_or_else(|| query_term.to_string());

        let corrected_query =
            extract_text_between(xml, "<CorrectedQuery>", "</CorrectedQuery>").unwrap_or_default();

        // Parse SpelledQuery segments
        let spelled_query = if let Some(spelled_content) =
            extract_text_between(xml, "<SpelledQuery>", "</SpelledQuery>")
        {
            Self::parse_spelled_query_segments(&spelled_content)
        } else {
            Vec::new()
        };

        Ok(SpellCheckResult {
            database,
            query,
            corrected_query,
            spelled_query,
        })
    }

    /// Parse the interleaved <Original> and <Replaced> elements from SpelledQuery
    fn parse_spelled_query_segments(content: &str) -> Vec<SpelledQuerySegment> {
        let mut segments = Vec::new();
        let mut pos = 0;

        while pos < content.len() {
            let orig_pos = content[pos..].find("<Original>");
            let repl_pos = content[pos..].find("<Replaced>");

            match (orig_pos, repl_pos) {
                (Some(o), Some(r)) if o <= r => {
                    // <Original> comes first
                    let abs_start = pos + o;
                    if let Some(end_offset) = content[abs_start..].find("</Original>") {
                        let text_start = abs_start + "<Original>".len();
                        let text_end = abs_start + end_offset;
                        segments.push(SpelledQuerySegment::Original(
                            content[text_start..text_end].to_string(),
                        ));
                        pos = text_end + "</Original>".len();
                    } else {
                        break;
                    }
                }
                (Some(_), Some(r)) => {
                    // <Replaced> comes first
                    let abs_start = pos + r;
                    if let Some(end_offset) = content[abs_start..].find("</Replaced>") {
                        let text_start = abs_start + "<Replaced>".len();
                        let text_end = abs_start + end_offset;
                        segments.push(SpelledQuerySegment::Replaced(
                            content[text_start..text_end].to_string(),
                        ));
                        pos = text_end + "</Replaced>".len();
                    } else {
                        break;
                    }
                }
                (Some(o), None) => {
                    // Only <Original> remaining
                    let abs_start = pos + o;
                    if let Some(end_offset) = content[abs_start..].find("</Original>") {
                        let text_start = abs_start + "<Original>".len();
                        let text_end = abs_start + end_offset;
                        segments.push(SpelledQuerySegment::Original(
                            content[text_start..text_end].to_string(),
                        ));
                        pos = text_end + "</Original>".len();
                    } else {
                        break;
                    }
                }
                (None, Some(r)) => {
                    // Only <Replaced> remaining
                    let abs_start = pos + r;
                    if let Some(end_offset) = content[abs_start..].find("</Replaced>") {
                        let text_start = abs_start + "<Replaced>".len();
                        let text_end = abs_start + end_offset;
                        segments.push(SpelledQuerySegment::Replaced(
                            content[text_start..text_end].to_string(),
                        ));
                        pos = text_end + "</Replaced>".len();
                    } else {
                        break;
                    }
                }
                (None, None) => break,
            }
        }

        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_espell_response_with_corrections() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<eSpellResult>
  <Database>pubmed</Database>
  <Query>asthmaa OR alergies</Query>
  <CorrectedQuery>asthma or allergies</CorrectedQuery>
  <SpelledQuery>
    <Original></Original>
    <Replaced>asthma</Replaced>
    <Original> OR </Original>
    <Replaced>allergies</Replaced>
  </SpelledQuery>
  <ERROR/>
</eSpellResult>"#;

        let result =
            PubMedClient::parse_espell_response(xml, "asthmaa OR alergies", "pubmed").unwrap();
        assert_eq!(result.database, "pubmed");
        assert_eq!(result.query, "asthmaa OR alergies");
        assert_eq!(result.corrected_query, "asthma or allergies");
        assert!(result.has_corrections());

        let replacements = result.replacements();
        assert_eq!(replacements.len(), 2);
        assert_eq!(replacements[0], "asthma");
        assert_eq!(replacements[1], "allergies");

        assert_eq!(result.spelled_query.len(), 4);
        assert_eq!(
            result.spelled_query[0],
            SpelledQuerySegment::Original("".to_string())
        );
        assert_eq!(
            result.spelled_query[1],
            SpelledQuerySegment::Replaced("asthma".to_string())
        );
        assert_eq!(
            result.spelled_query[2],
            SpelledQuerySegment::Original(" OR ".to_string())
        );
        assert_eq!(
            result.spelled_query[3],
            SpelledQuerySegment::Replaced("allergies".to_string())
        );
    }

    #[test]
    fn test_parse_espell_response_no_corrections() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<eSpellResult>
  <Database>pubmed</Database>
  <Query>asthma</Query>
  <CorrectedQuery>asthma</CorrectedQuery>
  <SpelledQuery>
    <Original>asthma</Original>
  </SpelledQuery>
  <ERROR/>
</eSpellResult>"#;

        let result = PubMedClient::parse_espell_response(xml, "asthma", "pubmed").unwrap();
        assert_eq!(result.query, "asthma");
        assert_eq!(result.corrected_query, "asthma");
        assert!(!result.has_corrections());
        assert!(result.replacements().is_empty());
    }

    #[test]
    fn test_parse_espell_response_empty_corrected() {
        let xml = r#"<eSpellResult>
  <Database>pubmed</Database>
  <Query>xyznonexistent</Query>
  <CorrectedQuery></CorrectedQuery>
  <SpelledQuery/>
  <ERROR/>
</eSpellResult>"#;

        let result = PubMedClient::parse_espell_response(xml, "xyznonexistent", "pubmed").unwrap();
        assert_eq!(result.query, "xyznonexistent");
        assert_eq!(result.corrected_query, "");
    }

    #[test]
    fn test_parse_espell_response_pmc_database() {
        let xml = r#"<eSpellResult>
  <Database>pmc</Database>
  <Query>fiberblast</Query>
  <CorrectedQuery>fibroblast</CorrectedQuery>
  <SpelledQuery>
    <Replaced>fibroblast</Replaced>
  </SpelledQuery>
  <ERROR/>
</eSpellResult>"#;

        let result = PubMedClient::parse_espell_response(xml, "fiberblast", "pmc").unwrap();
        assert_eq!(result.database, "pmc");
        assert_eq!(result.corrected_query, "fibroblast");
        assert!(result.has_corrections());
    }

    #[test]
    fn test_spell_check_empty_term() {
        use tokio_test;
        let client = PubMedClient::new();
        let result = tokio_test::block_on(client.spell_check(""));
        assert!(result.is_err());
    }

    #[test]
    fn test_spell_check_whitespace_term() {
        use tokio_test;
        let client = PubMedClient::new();
        let result = tokio_test::block_on(client.spell_check("   "));
        assert!(result.is_err());
    }

    #[test]
    fn test_spell_check_db_empty_db() {
        use tokio_test;
        let client = PubMedClient::new();
        let result = tokio_test::block_on(client.spell_check_db("asthma", ""));
        assert!(result.is_err());
    }
}
