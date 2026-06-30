//! Europe PMC `citations` endpoint response parsing.

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::models::EuropePmcCitation;

/// Parsed response from the Europe PMC `citations` endpoint (one page).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(from = "RawCitationsResponse")]
pub struct EuropePmcCitationList {
    /// Total number of citing articles (across all pages).
    pub hit_count: u64,
    /// The citing articles on this page.
    pub citations: Vec<EuropePmcCitation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCitationsResponse {
    #[serde(default)]
    hit_count: u64,
    #[serde(default)]
    citation_list: RawCitationList,
}

#[derive(Deserialize, Default)]
struct RawCitationList {
    #[serde(default)]
    citation: Vec<EuropePmcCitation>,
}

impl From<RawCitationsResponse> for EuropePmcCitationList {
    fn from(raw: RawCitationsResponse) -> Self {
        Self {
            hit_count: raw.hit_count,
            citations: raw.citation_list.citation,
        }
    }
}

/// Parse a Europe PMC `citations` JSON response.
pub fn parse_citations_response(json: &str) -> Result<EuropePmcCitationList> {
    Ok(serde_json::from_str(json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_citations() {
        let json = r#"{
            "hitCount": 1,
            "citationList": {
                "citation": [
                    {
                        "id": "99999",
                        "source": "MED",
                        "citationType": "JOURNAL ARTICLE",
                        "title": "A citing article",
                        "authorString": "Roe R.",
                        "journalAbbreviation": "Cell",
                        "pubYear": 2020,
                        "volume": "1",
                        "issue": "1",
                        "pageInfo": "1-9",
                        "citedByCount": 7
                    }
                ]
            }
        }"#;

        let resp = parse_citations_response(json).unwrap();
        assert_eq!(resp.hit_count, 1);
        assert_eq!(resp.citations.len(), 1);
        assert_eq!(resp.citations[0].id.as_deref(), Some("99999"));
        assert_eq!(resp.citations[0].pub_year.as_deref(), Some("2020"));
        assert_eq!(resp.citations[0].cited_by_count.as_deref(), Some("7"));
    }
}
