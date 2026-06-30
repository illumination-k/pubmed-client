//! Europe PMC `references` endpoint operations (page-number pagination).

use tracing::instrument;

use pubmed_parser::europe_pmc::{
    EuropePmcReference, EuropePmcReferenceList, parse_references_response,
};

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;

/// Default page size for reference/citation pagination.
pub(crate) const DEFAULT_PAGE_SIZE: u32 = 100;

impl EuropePmcClient {
    /// Fetch a single page of the reference list (works cited) for a record.
    #[instrument(skip(self), fields(id = %id, page, page_size))]
    pub async fn get_references_page(
        &self,
        id: &EuropePmcId,
        page: u32,
        page_size: u32,
    ) -> Result<EuropePmcReferenceList> {
        let endpoint = format!("{}/{}/references", id.source, id.id);
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
        Ok(parse_references_response(&text)?)
    }

    /// Fetch all references for a record, following page numbers until exhausted.
    #[instrument(skip(self), fields(id = %id))]
    pub async fn get_references(&self, id: &EuropePmcId) -> Result<Vec<EuropePmcReference>> {
        let mut collected = Vec::new();
        let mut page = 1;
        loop {
            let list = self
                .get_references_page(id, page, DEFAULT_PAGE_SIZE)
                .await?;
            let count = list.references.len();
            collected.extend(list.references);
            if count < DEFAULT_PAGE_SIZE as usize || collected.len() as u64 >= list.hit_count {
                break;
            }
            page += 1;
        }
        Ok(collected)
    }
}
