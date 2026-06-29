//! Europe PMC `search` endpoint operations (cursor-based pagination).

use tracing::{debug, instrument};

use pubmed_parser::europe_pmc::{EuropePmcResult, EuropePmcSearchResponse, parse_search_response};

use crate::error::Result;

use super::client::EuropePmcClient;

/// The level of detail returned by the Europe PMC `search` endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultType {
    /// Identifiers only.
    IdList,
    /// Core bibliographic fields (default).
    Lite,
    /// Full metadata including abstracts, MeSH, full author/affiliation data.
    Core,
}

impl ResultType {
    fn as_str(&self) -> &'static str {
        match self {
            ResultType::IdList => "idlist",
            ResultType::Lite => "lite",
            ResultType::Core => "core",
        }
    }
}

/// Options controlling a single Europe PMC `search` request.
#[derive(Debug, Clone)]
pub struct EuropePmcSearchOptions {
    /// Level of detail to return.
    pub result_type: ResultType,
    /// Number of results per page (Europe PMC caps this at 1000).
    pub page_size: u32,
    /// Cursor mark for the page to fetch. Use `"*"` for the first page.
    pub cursor_mark: String,
    /// Optional sort expression (e.g. `"P_PDATE_D desc"`, `"CITED desc"`).
    pub sort: Option<String>,
}

impl Default for EuropePmcSearchOptions {
    fn default() -> Self {
        Self {
            result_type: ResultType::Lite,
            page_size: 25,
            cursor_mark: "*".to_string(),
            sort: None,
        }
    }
}

impl EuropePmcClient {
    /// Search Europe PMC and return up to `limit` lite results.
    ///
    /// Convenience wrapper over [`EuropePmcClient::search_all`] using
    /// [`ResultType::Lite`]. For cursor control or `core` detail, use
    /// [`EuropePmcClient::search_page`] / [`EuropePmcClient::search_all`].
    #[instrument(skip(self), fields(query = %query, limit))]
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<EuropePmcResult>> {
        let opts = EuropePmcSearchOptions {
            page_size: limit.clamp(1, 1000) as u32,
            ..Default::default()
        };
        self.search_all(query, limit, &opts).await
    }

    /// Fetch a single page of search results.
    ///
    /// The returned [`EuropePmcSearchResponse::next_cursor_mark`] is the cursor
    /// to pass back via `opts.cursor_mark` to fetch the following page.
    #[instrument(skip(self, opts), fields(query = %query, cursor = %opts.cursor_mark))]
    pub async fn search_page(
        &self,
        query: &str,
        opts: &EuropePmcSearchOptions,
    ) -> Result<EuropePmcSearchResponse> {
        let page_size = opts.page_size.to_string();
        let mut params: Vec<(&str, &str)> = vec![
            ("query", query),
            ("format", "json"),
            ("resultType", opts.result_type.as_str()),
            ("pageSize", page_size.as_str()),
            ("cursorMark", opts.cursor_mark.as_str()),
        ];
        if let Some(sort) = &opts.sort {
            params.push(("sort", sort.as_str()));
        }

        let response = self
            .executor()
            .get_endpoint(&self.base_url, "search", &params)
            .await?;
        let text = response.text().await?;
        Ok(parse_search_response(&text)?)
    }

    /// Fetch search results across pages until `max_results` is reached or the
    /// result set is exhausted.
    ///
    /// Follows the `nextCursorMark` chain. Europe PMC signals the end of results
    /// by returning the same cursor it was given, so the loop also stops when the
    /// cursor stops advancing.
    #[instrument(skip(self, opts), fields(query = %query, max_results))]
    pub async fn search_all(
        &self,
        query: &str,
        max_results: usize,
        opts: &EuropePmcSearchOptions,
    ) -> Result<Vec<EuropePmcResult>> {
        let mut collected: Vec<EuropePmcResult> = Vec::new();
        let mut cursor = opts.cursor_mark.clone();

        while collected.len() < max_results {
            let page_opts = EuropePmcSearchOptions {
                cursor_mark: cursor.clone(),
                ..opts.clone()
            };
            let page = self.search_page(query, &page_opts).await?;

            if page.results.is_empty() {
                break;
            }
            collected.extend(page.results);

            match page.next_cursor_mark {
                // Cursor stopped advancing => last page reached.
                Some(next) if next != cursor => cursor = next,
                _ => break,
            }
        }

        collected.truncate(max_results);
        debug!(returned = collected.len(), "Europe PMC search_all complete");
        Ok(collected)
    }
}
