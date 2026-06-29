//! Europe PMC `references` endpoint response parsing.

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::models::EuropePmcReference;

/// Parsed response from the Europe PMC `references` endpoint (one page).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(from = "RawReferencesResponse")]
pub struct EuropePmcReferenceList {
    /// Total number of references for the article (across all pages).
    pub hit_count: u64,
    /// The references on this page.
    pub references: Vec<EuropePmcReference>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawReferencesResponse {
    #[serde(default)]
    hit_count: u64,
    #[serde(default)]
    reference_list: RawReferenceList,
}

#[derive(Deserialize, Default)]
struct RawReferenceList {
    #[serde(default)]
    reference: Vec<EuropePmcReference>,
}

impl From<RawReferencesResponse> for EuropePmcReferenceList {
    fn from(raw: RawReferencesResponse) -> Self {
        Self {
            hit_count: raw.hit_count,
            references: raw.reference_list.reference,
        }
    }
}

/// Parse a Europe PMC `references` JSON response.
pub fn parse_references_response(json: &str) -> Result<EuropePmcReferenceList> {
    Ok(serde_json::from_str(json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_references() {
        let json = r#"{
            "hitCount": 2,
            "referenceList": {
                "reference": [
                    {
                        "id": 12345,
                        "source": "MED",
                        "citationType": "JOURNAL ARTICLE",
                        "title": "Cited work one",
                        "authorString": "Doe J.",
                        "journalAbbreviation": "Nature",
                        "pubYear": 2010,
                        "volume": "5",
                        "issue": "2",
                        "pageInfo": "100-110",
                        "pmid": "12345",
                        "doi": "10.1/abc"
                    },
                    {
                        "title": "Unmatched reference",
                        "pubYear": 1999
                    }
                ]
            }
        }"#;

        let resp = parse_references_response(json).unwrap();
        assert_eq!(resp.hit_count, 2);
        assert_eq!(resp.references.len(), 2);
        // pubYear arrives as a JSON number but is normalized to a string.
        assert_eq!(resp.references[0].pub_year.as_deref(), Some("2010"));
        assert_eq!(resp.references[0].id.as_deref(), Some("12345"));
        assert_eq!(resp.references[0].pmid.as_deref(), Some("12345"));
        assert_eq!(resp.references[1].pub_year.as_deref(), Some("1999"));
        assert!(resp.references[1].id.is_none());
    }
}
