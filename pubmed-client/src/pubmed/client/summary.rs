//! ESummary API operations for fetching lightweight article metadata

use crate::common::PubMedId;
use crate::error::{ParseError, PubMedError, Result};
use crate::pubmed::models::ArticleSummary;
use crate::pubmed::query::SortOrder;
use crate::pubmed::responses::{ESummaryDocSum, ESummaryResponse};
use tracing::{debug, info, instrument, warn};

use super::PubMedClient;

impl PubMedClient {
    /// Fetch lightweight article summaries by PMIDs using the ESummary API
    ///
    /// Returns basic metadata (title, authors, journal, dates, DOI) without
    /// abstracts, MeSH terms, or chemical lists. Faster than `fetch_articles()`
    /// when you only need bibliographic overview data.
    ///
    /// # Arguments
    ///
    /// * `pmids` - Slice of PubMed IDs as strings
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<ArticleSummary>>` containing lightweight article metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let summaries = client.fetch_summaries(&["31978945", "33515491"]).await?;
    ///     for summary in &summaries {
    ///         println!("{}: {} ({})", summary.pmid, summary.title, summary.pub_date);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn fetch_summaries(&self, pmids: &[&str]) -> Result<Vec<ArticleSummary>> {
        if pmids.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all PMIDs upfront
        let validated: Vec<u32> = pmids
            .iter()
            .map(|pmid| {
                PubMedId::parse(pmid)
                    .map(|p| p.as_u32())
                    .map_err(PubMedError::from)
            })
            .collect::<Result<Vec<_>>>()?;

        const BATCH_SIZE: usize = 200;

        let mut all_summaries = Vec::with_capacity(pmids.len());

        for chunk in validated.chunks(BATCH_SIZE) {
            let id_list: String = chunk
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let url = format!(
                "{}/esummary.fcgi?db=pubmed&id={}&retmode=json",
                self.base_url, id_list
            );

            debug!(
                batch_size = chunk.len(),
                "Making batch ESummary API request"
            );
            let response = self.make_request(&url).await?;
            let json_text = response.text().await?;

            if json_text.trim().is_empty() {
                continue;
            }

            let summaries = Self::parse_esummary_response(&json_text)?;
            info!(
                requested = chunk.len(),
                parsed = summaries.len(),
                "ESummary batch completed"
            );
            all_summaries.extend(summaries);
        }

        Ok(all_summaries)
    }

    /// Fetch a single article summary by PMID using the ESummary API
    ///
    /// # Arguments
    ///
    /// * `pmid` - PubMed ID as a string
    ///
    /// # Returns
    ///
    /// Returns a `Result<ArticleSummary>` containing lightweight article metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let summary = client.fetch_summary("31978945").await?;
    ///     println!("{}: {}", summary.pmid, summary.title);
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmid = %pmid))]
    pub async fn fetch_summary(&self, pmid: &str) -> Result<ArticleSummary> {
        let mut summaries = self.fetch_summaries(&[pmid]).await?;

        if summaries.len() == 1 {
            Ok(summaries.remove(0))
        } else {
            let idx = summaries.iter().position(|s| s.pmid == pmid);
            match idx {
                Some(i) => Ok(summaries.remove(i)),
                None => Err(ParseError::ArticleNotFound {
                    pmid: pmid.to_string(),
                }
                .into()),
            }
        }
    }

    /// Search and fetch lightweight summaries in a single operation
    ///
    /// Combines `search_articles()` and `fetch_summaries()`. Use this when you
    /// only need basic metadata (title, authors, journal, dates) and want faster
    /// retrieval than `search_and_fetch()`.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of articles to fetch
    ///
    /// # Returns
    ///
    /// Returns a `Result<Vec<ArticleSummary>>` containing lightweight article metadata
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let summaries = client.search_and_fetch_summaries("covid-19 treatment", 20, None).await?;
    ///     for summary in &summaries {
    ///         println!("{}: {}", summary.pmid, summary.title);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_and_fetch_summaries(
        &self,
        query: &str,
        limit: usize,
        sort: Option<&SortOrder>,
    ) -> Result<Vec<ArticleSummary>> {
        let pmids = self.search_articles(query, limit, sort).await?;

        let pmid_refs: Vec<&str> = pmids.iter().map(|s| s.as_str()).collect();
        self.fetch_summaries(&pmid_refs).await
    }

    /// Parse ESummary JSON response into ArticleSummary objects
    pub(crate) fn parse_esummary_response(json_text: &str) -> Result<Vec<ArticleSummary>> {
        let response: ESummaryResponse =
            serde_json::from_str(json_text).map_err(|e| PubMedError::from(ParseError::from(e)))?;

        let result = &response.result;

        // Get the list of UIDs
        let uids = result
            .get("uids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut summaries = Vec::with_capacity(uids.len());

        for uid in &uids {
            let Some(doc_value) = result.get(uid) else {
                warn!(uid = %uid, "UID not found in ESummary response");
                continue;
            };

            // Check for error in individual document
            if doc_value.get("error").is_some() {
                warn!(uid = %uid, "ESummary returned error for UID");
                continue;
            }

            let doc: ESummaryDocSum = match serde_json::from_value(doc_value.clone()) {
                Ok(d) => d,
                Err(e) => {
                    warn!(uid = %uid, error = %e, "Failed to parse ESummary document");
                    continue;
                }
            };

            // Extract DOI and PMC ID from articleids
            let mut doi = None;
            let mut pmc_id = None;
            for aid in &doc.articleids {
                match aid.idtype.as_str() {
                    "doi" => {
                        if !aid.value.is_empty() {
                            doi = Some(aid.value.clone());
                        }
                    }
                    "pmc" => {
                        if !aid.value.is_empty() {
                            pmc_id = Some(aid.value.clone());
                        }
                    }
                    _ => {}
                }
            }

            let author_names: Vec<String> = doc.authors.iter().map(|a| a.name.clone()).collect();

            summaries.push(ArticleSummary {
                pmid: doc.uid,
                title: doc.title,
                authors: author_names,
                journal: doc.source,
                full_journal_name: doc.fulljournalname,
                pub_date: doc.pubdate,
                epub_date: doc.epubdate,
                doi,
                pmc_id,
                volume: doc.volume,
                issue: doc.issue,
                pages: doc.pages,
                languages: doc.lang,
                pub_types: doc.pubtype,
                issn: doc.issn,
                essn: doc.essn,
                sort_pub_date: doc.sortpubdate,
                pmc_ref_count: doc.pmcrefcount,
                record_status: doc.recordstatus,
            });
        }

        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_esummary_response_basic() {
        let json = r#"{"result":{"uids":["31978945"],"31978945":{"uid":"31978945","pubdate":"2020 Feb","epubdate":"2020 Jan 24","source":"N Engl J Med","authors":[{"name":"Zhu N","authtype":"Author","clusterid":""},{"name":"Zhang D","authtype":"Author","clusterid":""}],"title":"A Novel Coronavirus from Patients with Pneumonia in China, 2019.","sorttitle":"novel coronavirus","volume":"382","issue":"8","pages":"727-733","lang":["eng"],"issn":"0028-4793","essn":"1533-4406","pubtype":["Journal Article"],"articleids":[{"idtype":"pubmed","idtypen":1,"value":"31978945"},{"idtype":"doi","idtypen":3,"value":"10.1056/NEJMoa2001017"},{"idtype":"pmc","idtypen":8,"value":"PMC7092803"}],"fulljournalname":"The New England journal of medicine","sortpubdate":"2020/02/20 00:00","pmcrefcount":14123,"recordstatus":"PubMed - indexed for MEDLINE"}}}"#;

        let summaries = PubMedClient::parse_esummary_response(json).unwrap();
        assert_eq!(summaries.len(), 1);

        let s = &summaries[0];
        assert_eq!(s.pmid, "31978945");
        assert_eq!(
            s.title,
            "A Novel Coronavirus from Patients with Pneumonia in China, 2019."
        );
        assert_eq!(s.authors, vec!["Zhu N", "Zhang D"]);
        assert_eq!(s.journal, "N Engl J Med");
        assert_eq!(s.full_journal_name, "The New England journal of medicine");
        assert_eq!(s.pub_date, "2020 Feb");
        assert_eq!(s.epub_date, "2020 Jan 24");
        assert_eq!(s.doi.as_deref(), Some("10.1056/NEJMoa2001017"));
        assert_eq!(s.pmc_id.as_deref(), Some("PMC7092803"));
        assert_eq!(s.volume, "382");
        assert_eq!(s.issue, "8");
        assert_eq!(s.pages, "727-733");
        assert_eq!(s.languages, vec!["eng"]);
        assert_eq!(s.pub_types, vec!["Journal Article"]);
        assert_eq!(s.issn, "0028-4793");
        assert_eq!(s.essn, "1533-4406");
        assert_eq!(s.sort_pub_date, "2020/02/20 00:00");
        assert_eq!(s.pmc_ref_count, 14123);
        assert_eq!(s.record_status, "PubMed - indexed for MEDLINE");
    }

    #[test]
    fn test_parse_esummary_response_multiple_uids() {
        let json = r#"{"result":{"uids":["31978945","33515491"],"31978945":{"uid":"31978945","pubdate":"2020 Feb","epubdate":"","source":"N Engl J Med","authors":[{"name":"Zhu N","authtype":"Author","clusterid":""}],"title":"Article One","volume":"382","issue":"8","pages":"727-733","lang":["eng"],"issn":"","essn":"","pubtype":[],"articleids":[],"fulljournalname":"N Engl J Med","sortpubdate":"","pmcrefcount":0,"recordstatus":""},"33515491":{"uid":"33515491","pubdate":"2021 Jan","epubdate":"","source":"Science","authors":[{"name":"Smith J","authtype":"Author","clusterid":""}],"title":"Article Two","volume":"371","issue":"6526","pages":"120-125","lang":["eng"],"issn":"","essn":"","pubtype":[],"articleids":[{"idtype":"doi","idtypen":3,"value":"10.1126/science.abc123"}],"fulljournalname":"Science","sortpubdate":"","pmcrefcount":100,"recordstatus":""}}}"#;

        let summaries = PubMedClient::parse_esummary_response(json).unwrap();
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].pmid, "31978945");
        assert_eq!(summaries[0].title, "Article One");
        assert_eq!(summaries[1].pmid, "33515491");
        assert_eq!(summaries[1].title, "Article Two");
        assert_eq!(summaries[1].doi.as_deref(), Some("10.1126/science.abc123"));
    }

    #[test]
    fn test_parse_esummary_response_empty() {
        let json = r#"{"result": {"uids": []}}"#;
        let summaries = PubMedClient::parse_esummary_response(json).unwrap();
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_parse_esummary_response_with_error_uid() {
        let json = r#"{"result":{"uids":["99999999999"],"99999999999":{"uid":"99999999999","error":"cannot get document summary"}}}"#;

        let summaries = PubMedClient::parse_esummary_response(json).unwrap();
        assert!(summaries.is_empty());
    }

    #[test]
    fn test_parse_esummary_response_no_doi_no_pmc() {
        let json = r#"{"result":{"uids":["12345678"],"12345678":{"uid":"12345678","pubdate":"2020","epubdate":"","source":"Some Journal","authors":[],"title":"Test Article","volume":"","issue":"","pages":"","lang":[],"issn":"","essn":"","pubtype":[],"articleids":[{"idtype":"pubmed","idtypen":1,"value":"12345678"}],"fulljournalname":"Some Journal","sortpubdate":"","pmcrefcount":0,"recordstatus":""}}}"#;

        let summaries = PubMedClient::parse_esummary_response(json).unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].doi.is_none());
        assert!(summaries[0].pmc_id.is_none());
        assert!(summaries[0].authors.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_summaries_empty_input() {
        let client = PubMedClient::new();
        let result = client.fetch_summaries(&[]).await;
        assert!(result.is_ok());
        assert!(
            result
                .expect("empty input should return empty summaries")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn test_fetch_summaries_invalid_pmid() {
        let client = PubMedClient::new();
        let result = client.fetch_summaries(&["not_a_number"]).await;
        assert!(result.is_err());
    }
}
