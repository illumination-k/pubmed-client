//! Europe PMC `search` endpoint response parsing.

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::models::EuropePmcResult;

/// Parsed response from the Europe PMC `search` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(from = "RawSearchResponse")]
pub struct EuropePmcSearchResponse {
    /// Total number of records matching the query (across all pages).
    pub hit_count: u64,
    /// Cursor mark to pass as `cursorMark` to fetch the next page, if any.
    /// Europe PMC keeps returning the same value once the last page is reached.
    pub next_cursor_mark: Option<String>,
    /// The records on this page.
    pub results: Vec<EuropePmcResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSearchResponse {
    #[serde(default)]
    hit_count: u64,
    #[serde(default)]
    next_cursor_mark: Option<String>,
    #[serde(default)]
    result_list: RawResultList,
}

#[derive(Deserialize, Default)]
struct RawResultList {
    #[serde(default)]
    result: Vec<EuropePmcResult>,
}

impl From<RawSearchResponse> for EuropePmcSearchResponse {
    fn from(raw: RawSearchResponse) -> Self {
        Self {
            hit_count: raw.hit_count,
            next_cursor_mark: raw.next_cursor_mark,
            results: raw.result_list.result,
        }
    }
}

/// Parse a Europe PMC `search` JSON response.
pub fn parse_search_response(json: &str) -> Result<EuropePmcSearchResponse> {
    Ok(serde_json::from_str(json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search_lite() {
        let json = r#"{
            "hitCount": 2,
            "nextCursorMark": "AoJ123",
            "resultList": {
                "result": [
                    {
                        "id": "33515491",
                        "source": "MED",
                        "pmid": "33515491",
                        "pmcid": "PMC7894017",
                        "doi": "10.1000/x",
                        "title": "First",
                        "authorString": "Smith J.",
                        "journalTitle": "Nature",
                        "pubYear": "2021",
                        "isOpenAccess": "Y"
                    },
                    {
                        "id": "PPR12345",
                        "source": "PPR",
                        "title": "A preprint",
                        "pubYear": "2022"
                    }
                ]
            }
        }"#;

        let resp = parse_search_response(json).unwrap();
        assert_eq!(resp.hit_count, 2);
        assert_eq!(resp.next_cursor_mark.as_deref(), Some("AoJ123"));
        assert_eq!(resp.results.len(), 2);
        assert_eq!(resp.results[0].pmcid.as_deref(), Some("PMC7894017"));
        assert_eq!(resp.results[0].pub_year.as_deref(), Some("2021"));
        assert_eq!(resp.results[1].source, "PPR");
    }

    #[test]
    fn test_parse_search_core_keeps_extra_fields() {
        let json = r#"{
            "hitCount": 1,
            "resultList": {
                "result": [
                    {
                        "id": "1",
                        "source": "MED",
                        "title": "Has extra",
                        "citedByCount": 42,
                        "inEPMC": "Y"
                    }
                ]
            }
        }"#;

        let resp = parse_search_response(json).unwrap();
        let result = &resp.results[0];
        assert_eq!(
            result.extra.get("citedByCount").and_then(|v| v.as_i64()),
            Some(42)
        );
        assert_eq!(
            result.extra.get("inEPMC").and_then(|v| v.as_str()),
            Some("Y")
        );
    }

    #[test]
    fn test_parse_empty_results() {
        let json = r#"{"hitCount": 0, "resultList": {"result": []}}"#;
        let resp = parse_search_response(json).unwrap();
        assert_eq!(resp.hit_count, 0);
        assert!(resp.results.is_empty());
        assert!(resp.next_cursor_mark.is_none());
    }
}
