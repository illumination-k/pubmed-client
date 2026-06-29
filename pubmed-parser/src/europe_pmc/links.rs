//! Europe PMC `databaseLinks` endpoint response parsing.

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::models::EuropePmcDatabaseLink;

/// Parsed response from the Europe PMC `databaseLinks` endpoint (one page).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(from = "RawDatabaseLinksResponse")]
pub struct EuropePmcDatabaseLinkList {
    /// Total number of cross-reference groups (across all pages).
    pub hit_count: u64,
    /// Cross-reference groups, one per external database.
    pub links: Vec<EuropePmcDatabaseLink>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDatabaseLinksResponse {
    #[serde(default)]
    hit_count: u64,
    #[serde(default)]
    db_cross_reference_list: RawDbCrossReferenceList,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawDbCrossReferenceList {
    #[serde(default)]
    db_cross_reference: Vec<EuropePmcDatabaseLink>,
}

impl From<RawDatabaseLinksResponse> for EuropePmcDatabaseLinkList {
    fn from(raw: RawDatabaseLinksResponse) -> Self {
        Self {
            hit_count: raw.hit_count,
            links: raw.db_cross_reference_list.db_cross_reference,
        }
    }
}

/// Parse a Europe PMC `databaseLinks` JSON response.
pub fn parse_database_links_response(json: &str) -> Result<EuropePmcDatabaseLinkList> {
    Ok(serde_json::from_str(json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_database_links() {
        let json = r#"{
            "hitCount": 1,
            "dbCrossReferenceList": {
                "dbCrossReference": [
                    {
                        "dbName": "UNIPROT",
                        "dbCount": 2,
                        "dbCrossReferenceInfo": [
                            {"info1": "P12345", "info2": "PROT1_HUMAN"},
                            {"info1": "Q67890"}
                        ]
                    }
                ]
            }
        }"#;

        let resp = parse_database_links_response(json).unwrap();
        assert_eq!(resp.hit_count, 1);
        assert_eq!(resp.links.len(), 1);
        let link = &resp.links[0];
        assert_eq!(link.db_name.as_deref(), Some("UNIPROT"));
        assert_eq!(link.db_count, Some(2));
        assert_eq!(link.info.len(), 2);
        assert_eq!(link.info[0].info1.as_deref(), Some("P12345"));
        assert_eq!(link.info[0].info2.as_deref(), Some("PROT1_HUMAN"));
        assert_eq!(link.info[1].info1.as_deref(), Some("Q67890"));
    }
}
