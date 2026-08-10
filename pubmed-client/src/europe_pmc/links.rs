//! Europe PMC `databaseLinks` endpoint operations.

use tracing::instrument;

use pubmed_parser::europe_pmc::{
    EuropePmcDatabaseLink, EuropePmcDatabaseLinkList, parse_database_links_response,
};

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;
use super::paged::PagedList;

/// Path segment of the `databaseLinks` list endpoint.
const SEGMENT: &str = "databaseLinks";

impl PagedList for EuropePmcDatabaseLinkList {
    type Item = EuropePmcDatabaseLink;

    fn hit_count(&self) -> u64 {
        self.hit_count
    }

    fn into_items(self) -> Vec<Self::Item> {
        self.links
    }
}

impl EuropePmcClient {
    /// Fetch a single page of external database cross-references for a record.
    #[instrument(skip(self), fields(id = %id, page, page_size))]
    pub async fn get_database_links_page(
        &self,
        id: &EuropePmcId,
        page: u32,
        page_size: u32,
    ) -> Result<EuropePmcDatabaseLinkList> {
        self.get_list_page(id, SEGMENT, page, page_size, parse_database_links_response)
            .await
    }

    /// Fetch all external database cross-references for a record.
    #[instrument(skip(self), fields(id = %id))]
    pub async fn get_database_links(&self, id: &EuropePmcId) -> Result<Vec<EuropePmcDatabaseLink>> {
        self.collect_list_pages(id, SEGMENT, parse_database_links_response)
            .await
    }
}
