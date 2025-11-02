use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESearchResult {
    pub esearchresult: ESearchData,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESearchData {
    #[serde(default, rename = "ERROR")]
    pub error: Option<String>,
    #[serde(default)]
    pub count: Option<String>,
    #[serde(default)]
    pub retmax: Option<String>,
    #[serde(default)]
    pub retstart: Option<String>,
    #[serde(default)]
    pub idlist: Vec<String>,
}

// EInfo API response structures
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EInfoResponse {
    #[serde(rename = "einforesult")]
    pub einfo_result: EInfoResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EInfoResult {
    #[serde(rename = "dblist", default)]
    pub db_list: Option<Vec<String>>,
    #[serde(rename = "dbinfo", default)]
    pub db_info: Option<Vec<EInfoDbInfo>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EInfoDbInfo {
    #[serde(rename = "dbname")]
    pub db_name: String,
    #[serde(rename = "menuname")]
    pub menu_name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "dbbuild")]
    pub db_build: Option<String>,
    #[serde(rename = "count")]
    pub count: Option<String>,
    #[serde(rename = "lastupdate")]
    pub last_update: Option<String>,
    #[serde(rename = "fieldlist")]
    pub field_list: Option<Vec<EInfoField>>,
    #[serde(rename = "linklist")]
    pub link_list: Option<Vec<EInfoLink>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EInfoField {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "fullname")]
    pub full_name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "termcount")]
    pub term_count: Option<String>,
    #[serde(rename = "isdate")]
    pub is_date: Option<String>,
    #[serde(rename = "isnumerical")]
    pub is_numerical: Option<String>,
    #[serde(rename = "singletoken")]
    pub single_token: Option<String>,
    #[serde(rename = "hierarchy")]
    pub hierarchy: Option<String>,
    #[serde(rename = "ishidden")]
    pub is_hidden: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct EInfoLink {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "menu")]
    pub menu: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "dbto")]
    pub db_to: String,
}

// ELink API response structures
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ELinkResponse {
    #[serde(rename = "linksets")]
    pub linksets: Vec<ELinkSet>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ELinkSet {
    #[serde(rename = "dbfrom")]
    pub db_from: String,
    #[serde(rename = "ids")]
    pub ids: Vec<String>,
    #[serde(rename = "linksetdbs", default)]
    pub linkset_dbs: Option<Vec<ELinkSetDb>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ELinkSetDb {
    #[serde(rename = "dbto")]
    pub db_to: String,
    #[serde(rename = "linkname")]
    pub link_name: String,
    #[serde(rename = "links")]
    pub links: Vec<String>,
}
