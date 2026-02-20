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
    /// WebEnv session identifier for history server
    #[serde(default)]
    pub webenv: Option<String>,
    /// Query key for history server
    #[serde(default, rename = "querykey")]
    pub query_key: Option<String>,
    /// How PubMed interpreted and translated the search query
    #[serde(default)]
    pub querytranslation: Option<String>,
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

// ESummary API response structures

/// ESummary returns a JSON object with "result" containing "uids" array and per-UID objects.
/// We use serde_json::Value to handle the dynamic per-UID keys, then parse manually.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESummaryResponse {
    pub result: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESummaryAuthor {
    pub name: String,
    #[serde(default)]
    pub authtype: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESummaryArticleId {
    pub idtype: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESummaryDocSum {
    pub uid: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, rename = "sorttitle")]
    pub sort_title: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub authors: Vec<ESummaryAuthor>,
    #[serde(default)]
    pub pubdate: String,
    #[serde(default)]
    pub epubdate: String,
    #[serde(default)]
    pub volume: String,
    #[serde(default)]
    pub issue: String,
    #[serde(default)]
    pub pages: String,
    #[serde(default)]
    pub lang: Vec<String>,
    #[serde(default)]
    pub issn: String,
    #[serde(default)]
    pub essn: String,
    #[serde(default)]
    pub pubtype: Vec<String>,
    #[serde(default)]
    pub articleids: Vec<ESummaryArticleId>,
    #[serde(default)]
    pub fulljournalname: String,
    #[serde(default)]
    pub sortpubdate: String,
    #[serde(default)]
    pub pmcrefcount: u64,
    #[serde(default)]
    pub recordstatus: String,
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
