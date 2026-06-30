//! Europe PMC `databaseLinks` endpoint operations.

use tracing::instrument;

use pubmed_parser::europe_pmc::{
    EuropePmcDatabaseLink, EuropePmcDatabaseLinkList, parse_database_links_response,
};

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;
use super::references::DEFAULT_PAGE_SIZE;

impl EuropePmcClient {
    /// Fetch a single page of external database cross-references for a record.
    #[instrument(skip(self), fields(id = %id, page, page_size))]
    pub async fn get_database_links_page(
        &self,
        id: &EuropePmcId,
        page: u32,
        page_size: u32,
    ) -> Result<EuropePmcDatabaseLinkList> {
        let endpoint = format!("{}/{}/databaseLinks", id.source, id.id);
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
        Ok(parse_database_links_response(&text)?)
    }

    /// Fetch all external database cross-references for a record.
    #[instrument(skip(self), fields(id = %id))]
    pub async fn get_database_links(&self, id: &EuropePmcId) -> Result<Vec<EuropePmcDatabaseLink>> {
        let mut collected = Vec::new();
        let mut page = 1;
        loop {
            let list = self
                .get_database_links_page(id, page, DEFAULT_PAGE_SIZE)
                .await?;
            let count = list.links.len();
            collected.extend(list.links);
            if count < DEFAULT_PAGE_SIZE as usize || collected.len() as u64 >= list.hit_count {
                break;
            }
            page += 1;
        }
        Ok(collected)
    }
}
