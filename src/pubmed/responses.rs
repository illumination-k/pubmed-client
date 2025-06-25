use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESearchResult {
    pub esearchresult: ESearchData,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESearchData {
    pub idlist: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESummaryResult {
    pub result: ESummaryResultData,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESummaryResultData {
    pub uids: Vec<String>,
    #[serde(flatten)]
    pub articles: std::collections::HashMap<String, ESummaryData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ESummaryData {
    pub title: String,
    pub authors: Vec<AuthorData>,
    pub fulljournalname: String,
    pub pubdate: String,
    pub elocationid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AuthorData {
    pub name: String,
}
