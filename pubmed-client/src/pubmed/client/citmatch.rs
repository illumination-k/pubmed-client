//! ECitMatch API operations for matching citations to PMIDs

use crate::error::Result;
use crate::pubmed::models::{CitationMatch, CitationMatchStatus, CitationMatches, CitationQuery};
use tracing::{debug, info, instrument};

use super::PubMedClient;

impl PubMedClient {
    /// Match citations to PMIDs using the ECitMatch API
    ///
    /// This method takes citation information (journal, year, volume, page, author)
    /// and returns the corresponding PMIDs. Useful for identifying PMIDs from
    /// reference lists.
    ///
    /// # Arguments
    ///
    /// * `citations` - List of citation queries to match
    ///
    /// # Returns
    ///
    /// Returns a `Result<CitationMatches>` containing match results for each citation
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::{PubMedClient, pubmed::CitationQuery};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let citations = vec![
    ///         CitationQuery::new(
    ///             "proc natl acad sci u s a", "1991", "88", "3248", "mann bj", "Art1",
    ///         ),
    ///         CitationQuery::new(
    ///             "science", "1987", "235", "182", "palmenberg ac", "Art2",
    ///         ),
    ///     ];
    ///     let results = client.match_citations(&citations).await?;
    ///     for m in &results.matches {
    ///         println!("{}: {:?} ({:?})", m.key, m.pmid, m.status);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(citations_count = citations.len()))]
    pub async fn match_citations(&self, citations: &[CitationQuery]) -> Result<CitationMatches> {
        if citations.is_empty() {
            return Ok(CitationMatches {
                matches: Vec::new(),
            });
        }

        // Build bdata parameter: citations separated by %0D (carriage return)
        let bdata: String = citations
            .iter()
            .map(|c| c.to_bdata())
            .collect::<Vec<_>>()
            .join("%0D");

        let url = format!(
            "{}/ecitmatch.cgi?db=pubmed&retmode=xml&bdata={}",
            self.base_url, bdata
        );

        debug!(
            citations_count = citations.len(),
            "Making ECitMatch API request"
        );
        let response = self.make_request(&url).await?;
        let text = response.text().await?;

        // Parse pipe-delimited response
        let matches = Self::parse_ecitmatch_response(&text);

        info!(
            citations_count = citations.len(),
            matched_count = matches
                .iter()
                .filter(|m| m.status == CitationMatchStatus::Found)
                .count(),
            "ECitMatch completed"
        );

        Ok(CitationMatches { matches })
    }

    /// Parse ECitMatch pipe-delimited response into CitationMatch results
    pub(crate) fn parse_ecitmatch_response(text: &str) -> Vec<CitationMatch> {
        let mut matches = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 7 {
                let pmid_str = parts[6].trim();
                let (pmid, status) = if pmid_str.is_empty() {
                    (None, CitationMatchStatus::NotFound)
                } else if pmid_str.eq_ignore_ascii_case("AMBIGUOUS") {
                    (None, CitationMatchStatus::Ambiguous)
                } else {
                    (Some(pmid_str.to_string()), CitationMatchStatus::Found)
                };

                matches.push(CitationMatch {
                    journal: parts[0].replace('+', " "),
                    year: parts[1].to_string(),
                    volume: parts[2].to_string(),
                    first_page: parts[3].to_string(),
                    author_name: parts[4].replace('+', " "),
                    key: parts[5].to_string(),
                    pmid,
                    status,
                });
            }
        }

        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ecitmatch_response_found() {
        let response = "proc natl acad sci u s a|1991|88|3248|mann bj|Art1|2014248\n";
        let matches = PubMedClient::parse_ecitmatch_response(response);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].journal, "proc natl acad sci u s a");
        assert_eq!(matches[0].year, "1991");
        assert_eq!(matches[0].volume, "88");
        assert_eq!(matches[0].first_page, "3248");
        assert_eq!(matches[0].author_name, "mann bj");
        assert_eq!(matches[0].key, "Art1");
        assert_eq!(matches[0].pmid, Some("2014248".to_string()));
        assert_eq!(matches[0].status, CitationMatchStatus::Found);
    }

    #[test]
    fn test_parse_ecitmatch_response_not_found() {
        let response = "fake journal|2000|1|1|nobody|ref1|\n";
        let matches = PubMedClient::parse_ecitmatch_response(response);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pmid, None);
        assert_eq!(matches[0].status, CitationMatchStatus::NotFound);
    }

    #[test]
    fn test_parse_ecitmatch_response_ambiguous() {
        let response = "some journal|2000|1|1|smith|ref1|AMBIGUOUS\n";
        let matches = PubMedClient::parse_ecitmatch_response(response);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pmid, None);
        assert_eq!(matches[0].status, CitationMatchStatus::Ambiguous);
    }

    #[test]
    fn test_parse_ecitmatch_response_multiple() {
        let response = concat!(
            "proc natl acad sci u s a|1991|88|3248|mann bj|Art1|2014248\n",
            "science|1987|235|182|palmenberg ac|Art2|3026048\n",
        );
        let matches = PubMedClient::parse_ecitmatch_response(response);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].pmid, Some("2014248".to_string()));
        assert_eq!(matches[1].pmid, Some("3026048".to_string()));
    }

    #[test]
    fn test_parse_ecitmatch_response_empty() {
        let matches = PubMedClient::parse_ecitmatch_response("");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_parse_ecitmatch_response_plus_to_space() {
        let response = "proc+natl+acad+sci|1991|88|3248|mann+bj|Art1|2014248\n";
        let matches = PubMedClient::parse_ecitmatch_response(response);

        assert_eq!(matches[0].journal, "proc natl acad sci");
        assert_eq!(matches[0].author_name, "mann bj");
    }

    #[test]
    fn test_citation_query_to_bdata() {
        let query = CitationQuery::new(
            "proc natl acad sci u s a",
            "1991",
            "88",
            "3248",
            "mann bj",
            "Art1",
        );
        let bdata = query.to_bdata();
        assert_eq!(bdata, "proc+natl+acad+sci+u+s+a|1991|88|3248|mann+bj|Art1|");
    }

    #[test]
    fn test_empty_citations_match() {
        use tokio_test;
        let client = PubMedClient::new();
        let result = tokio_test::block_on(client.match_citations(&[]));
        assert!(result.is_ok());
        assert!(result.unwrap().matches.is_empty());
    }
}
