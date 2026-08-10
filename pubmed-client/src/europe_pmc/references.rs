//! Europe PMC `references` endpoint operations (page-number pagination).

use tracing::instrument;

use pubmed_parser::europe_pmc::{
    EuropePmcReference, EuropePmcReferenceList, parse_references_response,
};

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;
use super::paged::PagedList;

/// Path segment of the `references` list endpoint.
const SEGMENT: &str = "references";

impl PagedList for EuropePmcReferenceList {
    type Item = EuropePmcReference;

    fn hit_count(&self) -> u64 {
        self.hit_count
    }

    fn into_items(self) -> Vec<Self::Item> {
        self.references
    }
}

impl EuropePmcClient {
    /// Fetch a single page of the reference list (works cited) for a record.
    #[instrument(skip(self), fields(id = %id, page, page_size))]
    pub async fn get_references_page(
        &self,
        id: &EuropePmcId,
        page: u32,
        page_size: u32,
    ) -> Result<EuropePmcReferenceList> {
        self.get_list_page(id, SEGMENT, page, page_size, parse_references_response)
            .await
    }

    /// Fetch all references for a record, following page numbers until exhausted.
    #[instrument(skip(self), fields(id = %id))]
    pub async fn get_references(&self, id: &EuropePmcId) -> Result<Vec<EuropePmcReference>> {
        self.collect_list_pages(id, SEGMENT, parse_references_response)
            .await
    }
}
