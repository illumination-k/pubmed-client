//! Shared page-number pagination for the Europe PMC list endpoints.
//!
//! `references`, `citations` and `databaseLinks` are the same endpoint shape:
//! `/{source}/{id}/{segment}?format=json&page=N&pageSize=M`, answered by a JSON
//! object carrying a total `hitCount` and one page of items. Only the path
//! segment, the response type and its item field differ, so the request and the
//! page-walking loop live here once and each endpoint module supplies the three
//! things that actually vary.

use pubmed_parser::Result as ParseResult;

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;

/// Default page size for reference / citation / database-link pagination.
pub(crate) const DEFAULT_PAGE_SIZE: u32 = 100;

/// One page of a page-numbered Europe PMC list response.
///
/// Implemented by the `*List` models in [`pubmed_parser::europe_pmc`] so
/// [`EuropePmcClient::collect_list_pages`] can walk any of them.
pub(crate) trait PagedList: Sized {
    /// The element type collected across pages.
    type Item;

    /// Total number of items across all pages, as reported by the API.
    fn hit_count(&self) -> u64;

    /// Consume the page and yield its items.
    fn into_items(self) -> Vec<Self::Item>;
}

impl EuropePmcClient {
    /// Fetch one page of a `/{source}/{id}/{segment}` list endpoint.
    pub(crate) async fn get_list_page<T>(
        &self,
        id: &EuropePmcId,
        segment: &str,
        page: u32,
        page_size: u32,
        parse: fn(&str) -> ParseResult<T>,
    ) -> Result<T> {
        let endpoint = format!("{}/{}/{}", id.source, id.id, segment);
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
        Ok(parse(&text)?)
    }

    /// Walk a list endpoint from page 1 until it is exhausted.
    ///
    /// Stops on a short page (fewer items than [`DEFAULT_PAGE_SIZE`]) or once
    /// the reported `hitCount` has been collected, so a server that keeps
    /// returning full pages past its own total cannot loop forever.
    pub(crate) async fn collect_list_pages<T: PagedList>(
        &self,
        id: &EuropePmcId,
        segment: &str,
        parse: fn(&str) -> ParseResult<T>,
    ) -> Result<Vec<T::Item>> {
        let mut collected = Vec::new();
        let mut page = 1;
        loop {
            let list = self
                .get_list_page(id, segment, page, DEFAULT_PAGE_SIZE, parse)
                .await?;
            let hit_count = list.hit_count();
            let items = list.into_items();
            let count = items.len();
            collected.extend(items);
            if count < DEFAULT_PAGE_SIZE as usize || collected.len() as u64 >= hit_count {
                break;
            }
            page += 1;
        }
        Ok(collected)
    }
}
