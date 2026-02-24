//! ELink API operations for cross-referencing between NCBI databases

use crate::error::Result;
use crate::pubmed::models::{Citations, PmcLinks, RelatedArticles};
use crate::pubmed::responses::ELinkResponse;
use tracing::{debug, info, instrument};

use super::PubMedClient;

impl PubMedClient {
    /// Get related articles for given PMIDs
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to find related articles for
    ///
    /// # Returns
    ///
    /// Returns a `Result<RelatedArticles>` containing related article information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let related = client.get_related_articles(&[31978945]).await?;
    ///     println!("Found {} related articles", related.related_pmids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn get_related_articles(&self, pmids: &[u32]) -> Result<RelatedArticles> {
        if pmids.is_empty() {
            return Ok(RelatedArticles {
                source_pmids: Vec::new(),
                related_pmids: Vec::new(),
                link_type: "pubmed_pubmed".to_string(),
            });
        }

        let elink_response = self.elink_request(pmids, "pubmed", "pubmed_pubmed").await?;

        let mut all_related_pmids = Vec::new();

        for linkset in elink_response.linksets {
            if let Some(linkset_dbs) = linkset.linkset_dbs {
                for linkset_db in linkset_dbs {
                    if linkset_db.link_name == "pubmed_pubmed" {
                        for link_id in linkset_db.links {
                            if let Ok(pmid) = link_id.parse::<u32>() {
                                all_related_pmids.push(pmid);
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates and original PMIDs
        all_related_pmids.sort_unstable();
        all_related_pmids.dedup();
        all_related_pmids.retain(|&pmid| !pmids.contains(&pmid));

        info!(
            source_count = pmids.len(),
            related_count = all_related_pmids.len(),
            "Related articles retrieved successfully"
        );

        Ok(RelatedArticles {
            source_pmids: pmids.to_vec(),
            related_pmids: all_related_pmids,
            link_type: "pubmed_pubmed".to_string(),
        })
    }

    /// Get PMC links for given PMIDs (full-text availability)
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to check for PMC availability
    ///
    /// # Returns
    ///
    /// Returns a `Result<PmcLinks>` containing PMC IDs with full text available
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let pmc_links = client.get_pmc_links(&[31978945]).await?;
    ///     println!("Found {} PMC articles", pmc_links.pmc_ids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn get_pmc_links(&self, pmids: &[u32]) -> Result<PmcLinks> {
        if pmids.is_empty() {
            return Ok(PmcLinks {
                source_pmids: Vec::new(),
                pmc_ids: Vec::new(),
            });
        }

        let elink_response = self.elink_request(pmids, "pmc", "pubmed_pmc").await?;

        let mut pmc_ids = Vec::new();

        for linkset in elink_response.linksets {
            if let Some(linkset_dbs) = linkset.linkset_dbs {
                for linkset_db in linkset_dbs {
                    if linkset_db.link_name == "pubmed_pmc" && linkset_db.db_to == "pmc" {
                        pmc_ids.extend(linkset_db.links);
                    }
                }
            }
        }

        // Remove duplicates
        pmc_ids.sort();
        pmc_ids.dedup();

        info!(
            source_count = pmids.len(),
            pmc_count = pmc_ids.len(),
            "PMC links retrieved successfully"
        );

        Ok(PmcLinks {
            source_pmids: pmids.to_vec(),
            pmc_ids,
        })
    }

    /// Get citing articles for given PMIDs
    ///
    /// This method retrieves articles that cite the specified PMIDs from the PubMed database.
    /// The citation count returned represents only citations within the PubMed database
    /// (peer-reviewed journal articles indexed in PubMed).
    ///
    /// # Important Note on Citation Counts
    ///
    /// The citation count from this method may be **lower** than counts from other sources like
    /// Google Scholar, Web of Science, or scite.ai because:
    ///
    /// - **PubMed citations** (this method): Only includes peer-reviewed articles in PubMed
    /// - **Google Scholar/scite.ai**: Includes preprints, books, conference proceedings, and other sources
    ///
    /// For example, PMID 31978945 shows:
    /// - PubMed (this API): ~14,000 citations (PubMed database only)
    /// - scite.ai: ~23,000 citations (broader sources)
    ///
    /// This is expected behavior - this method provides accurate PubMed-specific citation data.
    ///
    /// # Arguments
    ///
    /// * `pmids` - List of PubMed IDs to find citing articles for
    ///
    /// # Returns
    ///
    /// Returns a `Result<Citations>` containing citing article information
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::PubMedClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PubMedClient::new();
    ///     let citations = client.get_citations(&[31978945]).await?;
    ///     println!("Found {} citing articles in PubMed", citations.citing_pmids.len());
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), fields(pmids_count = pmids.len()))]
    pub async fn get_citations(&self, pmids: &[u32]) -> Result<Citations> {
        if pmids.is_empty() {
            return Ok(Citations {
                source_pmids: Vec::new(),
                citing_pmids: Vec::new(),
                link_type: "pubmed_pubmed_citedin".to_string(),
            });
        }

        let elink_response = self
            .elink_request(pmids, "pubmed", "pubmed_pubmed_citedin")
            .await?;

        let mut citing_pmids = Vec::new();

        for linkset in elink_response.linksets {
            if let Some(linkset_dbs) = linkset.linkset_dbs {
                for linkset_db in linkset_dbs {
                    if linkset_db.link_name == "pubmed_pubmed_citedin" {
                        for link_id in linkset_db.links {
                            if let Ok(pmid) = link_id.parse::<u32>() {
                                citing_pmids.push(pmid);
                            }
                        }
                    }
                }
            }
        }

        // Remove duplicates
        citing_pmids.sort_unstable();
        citing_pmids.dedup();

        info!(
            source_count = pmids.len(),
            citing_count = citing_pmids.len(),
            "Citations retrieved successfully"
        );

        Ok(Citations {
            source_pmids: pmids.to_vec(),
            citing_pmids,
            link_type: "pubmed_pubmed_citedin".to_string(),
        })
    }

    /// Internal helper method for ELink API requests
    pub(crate) async fn elink_request(
        &self,
        pmids: &[u32],
        target_db: &str,
        link_name: &str,
    ) -> Result<ELinkResponse> {
        // Convert PMIDs to strings and join with commas
        let id_list: Vec<String> = pmids.iter().map(|id| id.to_string()).collect();
        let ids = id_list.join(",");

        // Build URL - API parameters will be added by make_request
        let url = format!(
            "{}/elink.fcgi?dbfrom=pubmed&db={}&id={}&linkname={}&retmode=json",
            self.base_url,
            urlencoding::encode(target_db),
            urlencoding::encode(&ids),
            urlencoding::encode(link_name)
        );

        debug!("Making ELink API request");
        let response = self.make_request(&url).await?;

        let elink_response: ELinkResponse = response.json().await?;
        Ok(elink_response)
    }
}
