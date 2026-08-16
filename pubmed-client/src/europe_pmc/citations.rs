//! Europe PMC `citations` endpoint operations (page-number pagination).

use tracing::instrument;

use pubmed_parser::europe_pmc::{
    EuropePmcCitation, EuropePmcCitationList, parse_citations_response,
};

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;
use super::paged::PagedList;

/// Path segment of the `citations` list endpoint.
const SEGMENT: &str = "citations";

impl PagedList for EuropePmcCitationList {
    type Item = EuropePmcCitation;

    fn hit_count(&self) -> u64 {
        self.hit_count
    }

    fn into_items(self) -> Vec<Self::Item> {
        self.citations
    }
}

impl EuropePmcClient {
    /// Fetch a single page of the citation list (citing articles) for a record.
    #[instrument(skip(self), fields(id = %id, page, page_size))]
    pub async fn get_citations_page(
        &self,
        id: &EuropePmcId,
        page: u32,
        page_size: u32,
    ) -> Result<EuropePmcCitationList> {
        self.get_list_page(id, SEGMENT, page, page_size, parse_citations_response)
            .await
    }

    /// Fetch all citing articles for a record, following page numbers until exhausted.
    #[instrument(skip(self), fields(id = %id))]
    pub async fn get_citations(&self, id: &EuropePmcId) -> Result<Vec<EuropePmcCitation>> {
        self.collect_list_pages(id, SEGMENT, parse_citations_response)
            .await
    }
}
