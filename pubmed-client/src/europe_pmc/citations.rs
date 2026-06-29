//! Europe PMC `citations` endpoint operations (page-number pagination).

use tracing::instrument;

use pubmed_parser::europe_pmc::{
    EuropePmcCitation, EuropePmcCitationList, parse_citations_response,
};

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;
use super::references::DEFAULT_PAGE_SIZE;

impl EuropePmcClient {
    /// Fetch a single page of the citation list (citing articles) for a record.
    #[instrument(skip(self), fields(id = %id, page, page_size))]
    pub async fn get_citations_page(
        &self,
        id: &EuropePmcId,
        page: u32,
        page_size: u32,
    ) -> Result<EuropePmcCitationList> {
        let endpoint = format!("{}/{}/citations", id.source, id.id);
        let page = page.to_string();
        let page_size = page_size.to_string();
        let response = self
            .executor()
            .get_endpoint(
                &self.base_url,
                &endpoint,
                &[
                    ("format", "json"),
                    ("page", page.as_str()),
                    ("pageSize", page_size.as_str()),
                ],
            )
            .await?;
        let text = response.text().await?;
        Ok(parse_citations_response(&text)?)
    }

    /// Fetch all citing articles for a record, following page numbers until exhausted.
    #[instrument(skip(self), fields(id = %id))]
    pub async fn get_citations(&self, id: &EuropePmcId) -> Result<Vec<EuropePmcCitation>> {
        let mut collected = Vec::new();
        let mut page = 1;
        loop {
            let list = self.get_citations_page(id, page, DEFAULT_PAGE_SIZE).await?;
            let count = list.citations.len();
            collected.extend(list.citations);
            if count < DEFAULT_PAGE_SIZE as usize || collected.len() as u64 >= list.hit_count {
                break;
            }
            page += 1;
        }
        Ok(collected)
    }
}
