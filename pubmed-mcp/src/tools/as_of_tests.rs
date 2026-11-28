//! One test per row of the `--as-of` behaviour table, driven through the real
//! tool functions against stand-ins for E-utilities and Europe PMC.
//!
//! The stand-ins answer from responses recorded from the live services
//! (`tests/fixtures/as_of/`) for three PubMed records, and — the point of the
//! exercise — they answer *whatever they are asked*. A tool that forgets to
//! apply the bound gets the later record back and the test sees it.
//!
//! | constant    | created    | Entrez date | publication date | Europe PMC first index | as of 2020-02-01 |
//! |-------------|------------|-------------|------------------|------------------------|------------------|
//! | `OLD`       | 1991-04-15 | 1991-04-15  | 1991 Apr 15      | 2009-12-24             | visible          |
//! | `EPUB`      | 2020-01-25 | 2020-01-25  | 2020 Feb 20      | 2020-01-28             | visible          |
//! | `LATER`     | 2021-01-30 | 2021-01-31  | 2021 Feb 6       | 2021-01-31             | hidden           |
//! | `BACKDATED` | 2021-10-25 | 2020-01-01  | 2020 Sep 21      | (PubMed tests only)    | hidden           |
//! | `UNKNOWN`   | —          | —           | —                | —                      | does not exist   |
//!
//! `EPUB` is the epub-ahead-of-print case: its issue date is after the cutoff
//! but it was in PubMed before it. `BACKDATED` is the opposite trap: published
//! and added to PubMed after the cutoff, but with an Entrez date set back to
//! before it.

use std::collections::HashSet;
use std::sync::Arc;

use pubmed_client::{Client, ClientConfig, EuropePmcClient};
use rmcp::handler::server::wrapper::{Json, Parameters};
use serde_json::{Value, json};
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::PubMedServer;

const AS_OF: &str = "2020-02-01";

const OLD: &str = "2014248";
const EPUB: &str = "31978945";
const LATER: &str = "33515491";
const BACKDATED: &str = "34695337";
const UNKNOWN: &str = "999999999";

const PMC_EPUB: &str = "PMC7092803";
const PMC_LATER: &str = "PMC7906746";
const PMC_UNKNOWN: &str = "PMC999999999";

const ESUMMARY: &str = include_str!("../../tests/fixtures/as_of/esummary.json");
const EUROPE_PMC_RECORDS: &str = include_str!("../../tests/fixtures/as_of/europe_pmc_search.json");
const EFETCH: [(&str, &str); 4] = [
    (
        OLD,
        include_str!("../../tests/fixtures/as_of/efetch_2014248.xml"),
    ),
    (
        EPUB,
        include_str!("../../tests/fixtures/as_of/efetch_31978945.xml"),
    ),
    (
        LATER,
        include_str!("../../tests/fixtures/as_of/efetch_33515491.xml"),
    ),
    (
        BACKDATED,
        include_str!("../../tests/fixtures/as_of/efetch_34695337.xml"),
    ),
];

/// `(pmid, create date, Entrez date)` of the fixtures: the `[crdt]` and
/// `[edat]` indexes of the ESearch and ELink stand-ins.
const RECORD_DATES: [(&str, &str, &str); 4] = [
    (BACKDATED, "2021/10/25", "2020/01/01"),
    (LATER, "2021/01/30", "2021/01/31"),
    (EPUB, "2020/01/25", "2020/01/25"),
    (OLD, "1991/04/15", "1991/04/15"),
];

/// The date bound of an ESearch / ELink request as a predicate over PMIDs,
/// honoured the way NCBI honours it: only when all three parameters are there.
fn date_bound(request: &Request) -> impl Fn(&str) -> bool {
    let bound = match (
        param(request, "datetype"),
        param(request, "mindate"),
        param(request, "maxdate"),
    ) {
        (Some(date_type), Some(min), Some(max)) => Some((date_type, min, max)),
        _ => None,
    };
    move |pmid: &str| {
        let Some((date_type, min, max)) = &bound else {
            return true;
        };
        RECORD_DATES
            .iter()
            .find(|(id, _, _)| *id == pmid)
            .is_some_and(|(_, created, entered)| {
                let date = if date_type == "crdt" {
                    created
                } else {
                    entered
                };
                min.as_str() <= *date && *date <= max.as_str()
            })
    }
}

fn param(request: &Request, name: &str) -> Option<String> {
    request
        .url
        .query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

fn ids(request: &Request) -> Vec<String> {
    param(request, "id")
        .unwrap_or_default()
        .split(',')
        .map(str::to_string)
        .collect()
}

fn json_response(body: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

// ---------------------------------------------------------------- E-utilities

/// ESearch: every fixture matches every term, within the date bound.
fn esearch(request: &Request) -> ResponseTemplate {
    let in_bound = date_bound(request);
    let hits: Vec<&str> = RECORD_DATES
        .iter()
        .map(|(pmid, _, _)| *pmid)
        .filter(|pmid| in_bound(pmid))
        .collect();
    json_response(json!({ "esearchresult": {
        "count": hits.len().to_string(),
        "retmax": hits.len().to_string(),
        "retstart": "0",
        "idlist": hits,
    }}))
}

fn esummary(request: &Request) -> ResponseTemplate {
    let recorded: Value = serde_json::from_str(ESUMMARY).unwrap();
    let uids = ids(request);
    let mut result = serde_json::Map::new();
    for uid in &uids {
        let doc = recorded.get(uid).cloned().unwrap_or_else(|| {
            let mut missing = recorded[UNKNOWN].clone();
            missing["uid"] = json!(uid);
            missing
        });
        result.insert(uid.clone(), doc);
    }
    result.insert("uids".to_string(), json!(uids));
    json_response(json!({ "result": result }))
}

/// A PMC article as small as the parser accepts, tied to its PubMed record.
fn pmc_xml(pmcid: &str, pmid: Option<&str>) -> String {
    let pmid = pmid
        .map(|pmid| format!(r#"<article-id pub-id-type="pmid">{pmid}</article-id>"#))
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0"?><pmc-articleset><article><front><journal-meta><journal-title-group><journal-title>Journal</journal-title></journal-title-group></journal-meta><article-meta>{pmid}<article-id pub-id-type="pmc">{pmcid}</article-id><title-group><article-title>Full text of {pmcid}</article-title></title-group></article-meta></front><body><sec><title>Results</title><p>Body of {pmcid}.</p></sec></body></article></pmc-articleset>"#
    )
}

fn pmc_article(pmcid: &str) -> Option<String> {
    match pmcid {
        PMC_EPUB => Some(pmc_xml(PMC_EPUB, Some(EPUB))),
        PMC_LATER => Some(pmc_xml(PMC_LATER, Some(LATER))),
        // In PMC but with no PubMed record to date it by.
        "PMC1" => Some(pmc_xml("PMC1", None)),
        _ => None,
    }
}

fn efetch(request: &Request) -> ResponseTemplate {
    if param(request, "db").as_deref() == Some("pmc") {
        let id = ids(request).remove(0);
        // Recorded shape of PMC's answer for an id it does not have.
        let body = pmc_article(&id).unwrap_or_else(|| {
            format!(
                r#"<?xml version="1.0"  ?><pmc-articleset><error id="{id}">The following PMCID is not available: {id}</error></pmc-articleset>"#
            )
        });
        return ResponseTemplate::new(200).set_body_string(body);
    }
    let articles: String = ids(request)
        .iter()
        .filter_map(|id| EFETCH.iter().find(|(pmid, _)| pmid == id))
        .map(|(_, xml)| *xml)
        .collect();
    ResponseTemplate::new(200).set_body_string(format!(
        r#"<?xml version="1.0" ?><PubmedArticleSet>{articles}</PubmedArticleSet>"#
    ))
}

/// ELink: every fixture is related to every other, the three later records
/// cite `OLD`, and `LATER` and `BACKDATED` cite `EPUB`; `EPUB` and `LATER`
/// have PMC versions. Links are narrowed by the date bound, when one is sent.
fn elink(request: &Request) -> ResponseTemplate {
    let sources = ids(request);
    let known = |pmid: &str| RECORD_DATES.iter().any(|(id, _, _)| *id == pmid);
    let has = |pmid: &str| sources.iter().any(|id| id == pmid);
    let in_bound = date_bound(request);
    let db = param(request, "db").unwrap_or_default();
    let link_name = param(request, "linkname").unwrap_or_else(|| format!("pubmed_{db}"));

    let links: Vec<&str> = match link_name.as_str() {
        "pubmed_pubmed" if sources.iter().any(|id| known(id)) => {
            vec![OLD, EPUB, LATER, BACKDATED]
        }
        "pubmed_pubmed_citedin" => [
            (OLD, vec![EPUB, LATER, BACKDATED]),
            (EPUB, vec![LATER, BACKDATED]),
        ]
        .into_iter()
        .filter(|(cited, _)| has(cited))
        .flat_map(|(_, citing)| citing)
        .collect(),
        "pubmed_pmc" => {
            return elink_response(
                &sources,
                &db,
                &link_name,
                [(EPUB, "7092803"), (LATER, "7906746")]
                    .into_iter()
                    .filter(|(pmid, _)| has(pmid))
                    .map(|(_, pmc)| pmc)
                    .collect(),
            );
        }
        _ => Vec::new(),
    };
    let links = links.into_iter().filter(|pmid| in_bound(pmid)).collect();
    elink_response(&sources, &db, &link_name, links)
}

fn elink_response(
    sources: &[String],
    db: &str,
    link_name: &str,
    links: Vec<&str>,
) -> ResponseTemplate {
    let mut linkset = json!({ "dbfrom": "pubmed", "ids": sources });
    if !links.is_empty() {
        linkset["linksetdbs"] = json!([{ "dbto": db, "linkname": link_name, "links": links }]);
    }
    json_response(json!({ "linksets": [linkset] }))
}

fn ecitmatch(_: &Request) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_string(format!(
        "n engl j med|2020|382|727|zhu n|epub|{EPUB}\nlancet|2021|397|452|sabino ec|later|{LATER}\nnowhere|1900|1|1|nobody|unknown|\n"
    ))
}

// ----------------------------------------------------------------- Europe PMC

fn europe_pmc_records() -> Vec<Value> {
    serde_json::from_str(EUROPE_PMC_RECORDS).unwrap()
}

/// Europe PMC `search`. A `CITES:` query selects the records citing the named
/// one (the same two the `citations` endpoint lists), a query naming ids
/// selects those records, and any other query matches all three. With
/// `honour_bound` unset the service ignores `FIRST_IDATE`, standing in for a
/// bound that failed to take effect.
fn europe_pmc_search(request: &Request, honour_bound: bool) -> ResponseTemplate {
    let query = param(request, "query").unwrap_or_default();
    let citing = query.split_once("CITES:").map(|(_, rest)| {
        let cited = rest.split('_').next().unwrap_or_default();
        if [OLD, EPUB, LATER].contains(&cited) {
            vec![EPUB, LATER]
        } else {
            Vec::new()
        }
    });
    let by_id = citing.is_some() || query.contains("EXT_ID:") || query.contains("PMCID:");
    let bound = query
        .split_once("FIRST_IDATE:[* TO ")
        .map(|(_, rest)| rest[..10].to_string())
        .filter(|_| honour_bound);

    let results: Vec<Value> = europe_pmc_records()
        .into_iter()
        .filter(|record| {
            let named = match &citing {
                Some(citing) => citing.contains(&record["id"].as_str().unwrap()),
                None => ["id", "pmcid"]
                    .iter()
                    .any(|field| record[field].as_str().is_some_and(|id| query.contains(id))),
            };
            let in_bound = bound
                .as_deref()
                .is_none_or(|bound| record["firstIndexDate"].as_str().unwrap() <= bound);
            (named || !by_id) && in_bound
        })
        .collect();
    json_response(json!({
        "hitCount": results.len(),
        "resultList": { "result": results },
    }))
}

fn graph_entry(pmid: &str) -> Value {
    let record = europe_pmc_records()
        .into_iter()
        .find(|record| record["id"] == pmid)
        .unwrap();
    json!({
        "id": pmid,
        "source": "MED",
        "title": record["title"],
        "authorString": record["authorString"],
        "journalAbbreviation": record["journalTitle"],
        "pubYear": record["pubYear"],
        "citedByCount": record["citedByCount"],
    })
}

/// The `/MED/{id}/...` list endpoints. Every known record has entries, the
/// hidden one included, so a tool that reports on a hidden record shows it.
fn europe_pmc_list(request: &Request) -> ResponseTemplate {
    let segments: Vec<&str> = request.url.path().trim_matches('/').split('/').collect();
    let (id, endpoint) = (segments[1], segments[2]);
    let known = [OLD, EPUB, LATER].contains(&id);

    let (list, item, entries) = match endpoint {
        "citations" if known => (
            "citationList",
            "citation",
            vec![graph_entry(EPUB), graph_entry(LATER)],
        ),
        "references" if known => (
            "referenceList",
            "reference",
            vec![
                graph_entry(OLD),
                graph_entry(LATER),
                // A reference Europe PMC could not match to a record.
                json!({ "title": "An unmatched book chapter", "pubYear": 1987 }),
            ],
        ),
        "databaseLinks" if known => (
            "dbCrossReferenceList",
            "dbCrossReference",
            vec![json!({
                "dbName": "EMBL",
                "dbCount": 1,
                "dbCrossReferenceInfo": [{ "info1": format!("ACC{id}") }],
            })],
        ),
        _ => return json_response(json!({ "hitCount": 0 })),
    };
    json_response(json!({ "hitCount": entries.len(), list: { item: entries } }))
}

fn europe_pmc_full_text(request: &Request) -> ResponseTemplate {
    let pmcid = request.url.path().trim_matches('/').split('/').next();
    match pmcid.and_then(pmc_article) {
        Some(xml) => ResponseTemplate::new(200).set_body_string(xml),
        None => ResponseTemplate::new(404),
    }
}

// -------------------------------------------------------------------- harness

type Responder = fn(&Request) -> ResponseTemplate;

struct Services {
    mock: MockServer,
}

impl Services {
    async fn start() -> Self {
        Self::start_with(true).await
    }

    async fn start_with(europe_pmc_honours_bound: bool) -> Self {
        let mock = MockServer::start().await;
        let routes: [(&str, Responder); 5] = [
            ("/esearch.fcgi", esearch),
            ("/esummary.fcgi", esummary),
            ("/efetch.fcgi", efetch),
            ("/elink.fcgi", elink),
            ("/ecitmatch.cgi", ecitmatch),
        ];
        for (route, respond) in routes {
            Mock::given(method("GET"))
                .and(path(route))
                .respond_with(respond)
                .mount(&mock)
                .await;
        }
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(move |request: &Request| {
                europe_pmc_search(request, europe_pmc_honours_bound)
            })
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                "^/[A-Z]+/[^/]+/(citations|references|databaseLinks)$",
            ))
            .respond_with(europe_pmc_list)
            .mount(&mock)
            .await;
        Mock::given(method("GET"))
            .and(path_regex("^/[^/]+/fullTextXML$"))
            .respond_with(europe_pmc_full_text)
            .mount(&mock)
            .await;
        Self { mock }
    }

    fn server(&self, as_of: Option<&str>) -> PubMedServer {
        let config = ClientConfig::new()
            .with_base_url(self.mock.uri())
            .with_rate_limit(1000.0);
        let mut client = Client::with_config(config.clone());
        client.europe_pmc = EuropePmcClient::with_config(config).with_base_url(self.mock.uri());
        PubMedServer::with_options(
            Arc::new(client),
            None,
            as_of.map(|date| date.parse().unwrap()),
        )
    }

    /// The server under test: bounded at [`AS_OF`].
    fn bounded(&self) -> PubMedServer {
        self.server(Some(AS_OF))
    }

    /// Every `(path, query)` the tools sent, for asserting what was *asked*.
    async fn requests(&self) -> Vec<(String, String)> {
        self.mock
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| {
                let query: Vec<String> = request
                    .url
                    .query_pairs()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect();
                (request.url.path().to_string(), query.join("&"))
            })
            .collect()
    }

    /// Whether any request to `route` mentioned `needle`.
    async fn asked(&self, route: &str, needle: &str) -> bool {
        self.requests()
            .await
            .iter()
            .any(|(path, query)| path.ends_with(route) && query.contains(needle))
    }
}

/// Call a tool function with JSON arguments; answer with its JSON output, or
/// the error message it failed with.
macro_rules! call {
    ($server:expr, $tool:path, $arguments:tt) => {
        $tool(
            &$server,
            Parameters(serde_json::from_value(json!($arguments)).expect("valid arguments")),
        )
        .await
        .map(|Json(output)| serde_json::to_value(output).expect("serializable output"))
        .map_err(|error| error.message.to_string())
    };
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected an array, got {value}"))
        .iter()
        .map(|item| match item {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect()
}

fn field(items: &Value, name: &str) -> Vec<String> {
    strings(&Value::Array(
        items
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item[name].clone())
            .collect(),
    ))
}

/// A tool answer with the id it was asked about blanked out, so the answers
/// for two different ids can be compared for everything else.
fn masked(answer: &Result<Value, String>, id: &str) -> String {
    let digits = id.trim_start_matches("PMC");
    format!("{answer:?}").replace(digits, "<id>")
}

// ------------------------------------------------------------------ fixtures

#[test]
fn the_epub_fixture_is_published_after_the_cutoff_but_entered_pubmed_before_it() {
    let recorded: Value = serde_json::from_str(ESUMMARY).unwrap();
    let epub = &recorded[EPUB];
    assert_eq!(epub["sortpubdate"], "2020/02/20 00:00");
    let entrez = epub["history"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["pubstatus"] == "entrez")
        .unwrap();
    assert_eq!(entrez["date"], "2020/01/25 06:00");
}

#[test]
fn the_backdated_fixture_has_an_entrez_date_before_the_cutoff_but_was_created_after_it() {
    let recorded: Value = serde_json::from_str(ESUMMARY).unwrap();
    let backdated = &recorded[BACKDATED];
    assert_eq!(backdated["pubdate"], "2020 Sep 21");
    let milestone = |status: &str| {
        backdated["history"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["pubstatus"] == status)
            .unwrap()["date"]
            .clone()
    };
    // NCBI's history calls the create date `entrez` and the Entrez date `pubmed`.
    assert_eq!(milestone("entrez"), "2021/10/25 17:36");
    assert_eq!(milestone("pubmed"), "2020/01/01 00:00");
}

// --------------------------------------------------------------------- PubMed

#[tokio::test]
async fn search_pubmed_is_bounded_by_entrez_date() {
    let services = Services::start().await;

    let bounded = call!(services.bounded(), super::search::search_pubmed, {
        "query": "coronavirus"
    })
    .unwrap();
    // EPUB (issue date after D) is in; LATER and BACKDATED are out.
    assert_eq!(field(&bounded["articles"], "pmid"), [EPUB, OLD]);
    assert_eq!(bounded["count"], 2);
    assert!(
        strings(&bounded["filters_applied"])
            .iter()
            .any(|filter| filter.starts_with("As of 2020-02-01")),
        "{bounded}"
    );

    // The bound went out as ESearch's own parameters — all three, since
    // ESearch ignores `maxdate` on its own — and ends the day before D.
    assert!(
        services
            .asked(
                "esearch.fcgi",
                "datetype=crdt&mindate=1000/01/01&maxdate=2020/01/31"
            )
            .await,
        "{:?}",
        services.requests().await
    );
    assert!(!services.asked("efetch.fcgi", LATER).await);
    assert!(!services.asked("efetch.fcgi", BACKDATED).await);

    // The same search without --as-of reaches the later records.
    let unbounded_services = Services::start().await;
    let unbounded = call!(unbounded_services.server(None), super::search::search_pubmed, {
        "query": "coronavirus"
    })
    .unwrap();
    assert_eq!(
        field(&unbounded["articles"], "pmid"),
        [BACKDATED, LATER, EPUB, OLD]
    );
}

/// ESearch can bound only one date type, and the create date is not the
/// whole rule: a record created the evening before D enters PubMed on D.
/// Every hit is therefore checked again before it is fetched.
#[tokio::test]
async fn search_pubmed_checks_each_hit_against_the_whole_rule() {
    let services = Services::start().await;

    // LATER was created 2021-01-30 and entered PubMed 2021-01-31.
    let answer = call!(services.server(Some("2021-01-31")), super::search::search_pubmed, {
        "query": "coronavirus"
    })
    .unwrap();

    assert!(services.asked("esearch.fcgi", "maxdate=2021/01/30").await);
    assert_eq!(field(&answer["articles"], "pmid"), [EPUB, OLD]);
    assert!(!services.asked("efetch.fcgi", LATER).await);

    let next_day = call!(services.server(Some("2021-02-01")), super::search::search_pubmed, {
        "query": "coronavirus"
    })
    .unwrap();
    assert_eq!(field(&next_day["articles"], "pmid"), [LATER, EPUB, OLD]);
}

#[tokio::test]
async fn search_pubmed_clamps_a_later_end_year() {
    let services = Services::start().await;
    let server = services.bounded();

    let clamped = call!(server, super::search::search_pubmed, {
        "query": "coronavirus", "start_year": 2015, "end_year": 2025
    })
    .unwrap();
    assert!(
        strings(&clamped["filters_applied"]).contains(&"Published 2015-2020".to_string()),
        "{clamped}"
    );
    assert!(clamped["query"].as_str().unwrap().contains("2015:2020"));

    // An earlier end year is the caller's own, tighter bound.
    let earlier = call!(server, super::search::search_pubmed, {
        "query": "coronavirus", "start_year": 2010, "end_year": 2012
    })
    .unwrap();
    assert!(strings(&earlier["filters_applied"]).contains(&"Published 2010-2012".to_string()));

    // Never clamped below the start year, which no record could satisfy.
    let late_start = call!(server, super::search::search_pubmed, {
        "query": "coronavirus", "start_year": 2021, "end_year": 2023
    })
    .unwrap();
    assert!(strings(&late_start["filters_applied"]).contains(&"Published 2021-2021".to_string()));
}

#[tokio::test]
async fn fetch_articles_leaves_out_later_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::articles::fetch_articles, {
        "pmids": [OLD, EPUB, LATER, BACKDATED, UNKNOWN]
    })
    .unwrap();

    assert_eq!(field(&answer["articles"], "pmid"), [OLD, EPUB]);
    assert_eq!(answer["requested"], 5);
    assert_eq!(answer["count"], 2);
    // The later records are listed exactly like the id PubMed has never heard
    // of — including the one whose Entrez date claims it predates D.
    assert_eq!(strings(&answer["not_found"]), [LATER, BACKDATED, UNKNOWN]);
    assert!(!services.asked("efetch.fcgi", LATER).await);
    assert!(!services.asked("efetch.fcgi", BACKDATED).await);
}

#[tokio::test]
async fn fetch_summaries_leaves_out_later_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::summary::fetch_summaries, {
        "pmids": [OLD, EPUB, LATER, BACKDATED, UNKNOWN]
    })
    .unwrap();

    assert_eq!(field(&answer["summaries"], "pmid"), [OLD, EPUB]);
    assert_eq!(answer["count"], 2);
    // The epub-ahead record is returned although its issue date is after D.
    assert_eq!(answer["summaries"][1]["pub_date"], "2020 Feb 20");
}

#[tokio::test]
async fn export_citations_leaves_out_later_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::export::export_citations, {
        "pmids": [OLD, EPUB, LATER], "format": "ris"
    })
    .unwrap();

    assert_eq!(strings(&answer["pmids"]), [OLD, EPUB]);
    assert_eq!(answer["count"], 2);
    let content = answer["content"].as_str().unwrap();
    assert!(content.contains("Novel Coronavirus"), "{content}");
    assert!(!content.contains("Manaus"), "{content}");
}

#[tokio::test]
async fn pmc_full_text_tools_refuse_articles_whose_pubmed_record_is_later() {
    let services = Services::start().await;
    let server = services.bounded();

    let markdown = call!(server, super::markdown::get_pmc_markdown, { "pmc_id": PMC_EPUB });
    assert!(
        markdown.unwrap()["markdown"]
            .as_str()
            .unwrap()
            .contains("Body of PMC7092803")
    );
    let fulltext = call!(server, super::fulltext::get_pmc_fulltext, { "pmc_id": PMC_EPUB });
    assert_eq!(fulltext.unwrap()["section_count"], 1);
    let figures = call!(server, super::figures::get_pmc_figures, { "pmc_id": PMC_EPUB });
    assert_eq!(figures.unwrap()["pmc_id"], PMC_EPUB);

    for pmc_id in [PMC_LATER, PMC_UNKNOWN, "PMC1"] {
        let expected =
            format!("Failed to fetch PMC article: PMC full text not available for {pmc_id}");
        let answers = [
            call!(server, super::markdown::get_pmc_markdown, { "pmc_id": pmc_id }),
            call!(server, super::fulltext::get_pmc_fulltext, { "pmc_id": pmc_id }),
            call!(server, super::figures::get_pmc_figures, { "pmc_id": pmc_id }),
        ];
        for answer in answers {
            assert_eq!(answer, Err(expected.clone()), "{pmc_id}");
        }
    }

    // Without --as-of the same article is served.
    let unbounded = call!(services.server(None), super::markdown::get_pmc_markdown, {
        "pmc_id": PMC_LATER
    });
    assert!(unbounded.is_ok(), "{unbounded:?}");
}

#[tokio::test]
async fn related_articles_drop_later_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::elink::get_related_articles, {
        "pmids": [31978945]
    })
    .unwrap();

    assert_eq!(strings(&answer["related_pmids"]), [OLD]);
    assert_eq!(answer["total"], 1);
    // The list is narrowed at the source, not fetched whole.
    assert!(
        services
            .asked(
                "elink.fcgi",
                "datetype=crdt&mindate=1000/01/01&maxdate=2020/01/31"
            )
            .await
    );
}

#[tokio::test]
async fn citations_drop_later_citing_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::elink::get_citations, {
        "pmids": [2014248]
    })
    .unwrap();
    assert_eq!(strings(&answer["citing_pmids"]), [EPUB]);
    assert_eq!(
        answer["total"], 1,
        "the total must not count hidden records"
    );
    assert!(
        services
            .asked(
                "elink.fcgi",
                "datetype=crdt&mindate=1000/01/01&maxdate=2020/01/31"
            )
            .await
    );

    // Both records citing EPUB are later, so nothing cites it yet.
    let none = call!(services.bounded(), super::elink::get_citations, {
        "pmids": [31978945]
    })
    .unwrap();
    assert_eq!(none["total"], 0);
}

#[tokio::test]
async fn pmc_links_drop_links_of_later_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::elink::get_pmc_links, {
        "pmids": [31978945, 33515491]
    })
    .unwrap();

    assert_eq!(strings(&answer["pmc_ids"]), [PMC_EPUB]);
    assert_eq!(answer["count"], 1);
    assert!(!services.asked("elink.fcgi", LATER).await);
}

#[tokio::test]
async fn match_citations_does_not_resolve_to_later_records() {
    let services = Services::start().await;
    let citations = json!([
        { "journal": "n engl j med", "year": "2020", "volume": "382", "first_page": "727",
          "author_name": "zhu n", "key": "epub" },
        { "journal": "lancet", "year": "2021", "volume": "397", "first_page": "452",
          "author_name": "sabino ec", "key": "later" },
        { "journal": "nowhere", "year": "1900", "volume": "1", "first_page": "1",
          "author_name": "nobody", "key": "unknown" },
    ]);

    let answer = call!(services.bounded(), super::citmatch::match_citations, {
        "citations": citations
    })
    .unwrap();

    assert_eq!(answer["found"], 1);
    assert_eq!(answer["matches"][0]["pmid"], EPUB);
    assert_eq!(answer["matches"][0]["status"], "found");
    // The later match reads exactly like the citation that matched nothing.
    for hidden_or_unmatched in [&answer["matches"][1], &answer["matches"][2]] {
        assert_eq!(hidden_or_unmatched["status"], "not_found");
        assert!(hidden_or_unmatched.get("pmid").is_none());
    }

    let unbounded = call!(services.server(None), super::citmatch::match_citations, {
        "citations": citations
    })
    .unwrap();
    assert_eq!(unbounded["found"], 2);
}

#[tokio::test]
async fn pmid_to_pmcid_does_not_convert_later_records() {
    let services = Services::start().await;
    let server = services.bounded();

    let visible = call!(server, super::convert::pmid_to_pmcid, { "pmid": EPUB }).unwrap();
    assert_eq!(visible["pmcid"], PMC_EPUB);

    let hidden = call!(server, super::convert::pmid_to_pmcid, { "pmid": LATER }).unwrap();
    assert_eq!(hidden["available"], false);
    assert!(hidden.get("pmcid").is_none());
}

#[test]
fn global_query_is_not_offered() {
    let offered = |as_of: Option<&str>| -> HashSet<String> {
        PubMedServer::with_options(
            Arc::new(Client::new()),
            None,
            as_of.map(|date| date.parse().unwrap()),
        )
        .tool_router
        .list_all()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect()
    };

    let unbounded = offered(None);
    let bounded = offered(Some(AS_OF));
    assert!(unbounded.contains("global_query"));
    let removed: Vec<&String> = unbounded.difference(&bounded).collect();
    assert_eq!(removed, ["global_query"]);

    // Asking for it by name does not bring it back.
    let requested = HashSet::from(["global_query".to_string(), "spell_check".to_string()]);
    let server = PubMedServer::with_options(
        Arc::new(Client::new()),
        Some(&requested),
        Some(AS_OF.parse().unwrap()),
    );
    let names: Vec<String> = server
        .tool_router
        .list_all()
        .into_iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert_eq!(names, ["spell_check"]);
}

// ----------------------------------------------------------------- Europe PMC

#[tokio::test]
async fn europe_pmc_search_is_bounded_by_first_index_date() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::europe_pmc::europe_pmc_search, {
        "query": "coronavirus OR lectin", "result_type": "core"
    })
    .unwrap();

    assert_eq!(field(&answer["results"], "id"), [OLD, EPUB]);
    assert_eq!(
        answer["query"],
        "(coronavirus OR lectin) AND FIRST_IDATE:[* TO 2020-01-31]"
    );
    // Today's citation counts are part of the future.
    for record in answer["results"].as_array().unwrap() {
        assert!(record.get("cited_by_count").is_none(), "{record}");
    }
}

#[tokio::test]
async fn europe_pmc_search_checks_each_record_even_if_the_query_bound_fails() {
    let services = Services::start_with(false).await;

    // `id_list` records carry no dates, so the check needs a fuller answer.
    let answer = call!(services.bounded(), super::europe_pmc::europe_pmc_search, {
        "query": "coronavirus", "result_type": "id_list"
    })
    .unwrap();

    assert_eq!(field(&answer["results"], "id"), [OLD, EPUB]);
    assert!(services.asked("/search", "resultType=lite").await);
}

#[tokio::test]
async fn europe_pmc_search_refuses_queries_that_could_escape_the_bound() {
    let services = Services::start().await;
    let server = services.bounded();

    let unbalanced = call!(server, super::europe_pmc::europe_pmc_search, {
        "query": "coronavirus) OR (lectin"
    });
    assert!(unbalanced.unwrap_err().contains("unbalanced"));

    let by_citations = call!(server, super::europe_pmc::europe_pmc_search, {
        "query": "coronavirus", "sort": "CITED desc"
    });
    assert!(by_citations.unwrap_err().contains("citation count"));

    assert!(services.requests().await.is_empty());
}

#[tokio::test]
async fn europe_pmc_citations_drop_later_citing_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::europe_pmc::europe_pmc_citations, {
        "id": OLD
    })
    .unwrap();

    assert_eq!(field(&answer["entries"], "id"), [EPUB]);
    assert_eq!(answer["total"], 1);
    assert_eq!(answer["entries"][0]["pmid"], EPUB);
    assert_eq!(answer["entries"][0]["journal"], "N Engl J Med");
    assert!(answer["entries"][0].get("cited_by_count").is_none());
    // Asked as one bounded search, never as a walk through every citation.
    assert!(
        services
            .asked(
                "/search",
                "CITES:2014248_MED AND FIRST_IDATE:[* TO 2020-01-31]"
            )
            .await
    );
    assert!(!services.asked("/citations", "").await);

    // If the service ignored the bound, each record's own date still decides
    // and the total follows.
    let lax = Services::start_with(false).await;
    let answer = call!(lax.bounded(), super::europe_pmc::europe_pmc_citations, {
        "id": OLD
    })
    .unwrap();
    assert_eq!(field(&answer["entries"], "id"), [EPUB]);
    assert_eq!(answer["total"], 1);

    let unbounded = call!(services.server(None), super::europe_pmc::europe_pmc_citations, {
        "id": OLD
    })
    .unwrap();
    assert_eq!(field(&unbounded["entries"], "id"), [EPUB, LATER]);
    assert_eq!(unbounded["entries"][0]["cited_by_count"], "18524");
}

#[tokio::test]
async fn europe_pmc_references_drop_entries_matched_to_later_records() {
    let services = Services::start().await;

    let answer = call!(services.bounded(), super::europe_pmc::europe_pmc_references, {
        "id": EPUB
    })
    .unwrap();

    // The matched later record goes; the unmatched entry is only what the
    // visible article itself printed, so it stays.
    let entries = answer["entries"].as_array().unwrap();
    assert_eq!(answer["total"], 2);
    assert_eq!(entries[0]["id"], OLD);
    assert_eq!(entries[1]["title"], "An unmatched book chapter");
    assert!(entries[1].get("id").is_none());
}

#[tokio::test]
async fn europe_pmc_full_text_and_database_links_follow_the_record_date() {
    let services = Services::start().await;
    let server = services.bounded();

    let full_text = call!(server, super::europe_pmc::europe_pmc_fulltext, { "id": PMC_EPUB });
    assert_eq!(full_text.unwrap()["section_count"], 1);
    let links = call!(server, super::europe_pmc::europe_pmc_database_links, { "id": EPUB });
    assert_eq!(links.unwrap()["count"], 1);

    for raw_xml in [false, true] {
        let refused = call!(server, super::europe_pmc::europe_pmc_fulltext, {
            "id": PMC_LATER, "raw_xml": raw_xml
        });
        assert_eq!(
            refused,
            Err(
                "Failed to fetch Europe PMC full text: PMC full text not available for PMC/PMC7906746"
                    .to_string()
            )
        );
    }
    let requests = services.requests().await;
    assert!(
        !requests.iter().any(|(path, _)| path.contains(PMC_LATER)),
        "the hidden full text must not even be requested: {requests:?}"
    );

    let links = call!(server, super::europe_pmc::europe_pmc_database_links, { "id": LATER });
    assert_eq!(links.unwrap()["count"], 0);
}

// ------------------------------------------------------- indistinguishability

/// The central promise: asking about a record hidden by `--as-of` yields,
/// byte for byte, what asking about a record that does not exist yields.
#[tokio::test]
async fn a_later_record_is_indistinguishable_from_one_that_does_not_exist() {
    let services = Services::start().await;
    let server = services.bounded();
    let later: u32 = LATER.parse().unwrap();
    let unknown: u32 = UNKNOWN.parse().unwrap();

    macro_rules! assert_indistinguishable {
        ($name:literal, |$id:ident| $call:expr, $hidden:expr, $missing:expr, $shown:expr) => {{
            let $id = $hidden;
            let hidden = masked(&$call, &$hidden.to_string());
            let $id = $missing;
            let missing = masked(&$call, &$missing.to_string());
            assert_eq!(hidden, missing, "{}: hidden vs. nonexistent", $name);

            // Guard against a vacuous pass: the same call does tell a visible
            // record apart from a nonexistent one.
            let $id = $shown;
            let shown = masked(&$call, &$shown.to_string());
            assert_ne!(shown, missing, "{}: visible vs. nonexistent", $name);
        }};
    }

    assert_indistinguishable!(
        "fetch_articles",
        |id| call!(server, super::articles::fetch_articles, { "pmids": [id] }),
        LATER,
        UNKNOWN,
        EPUB
    );
    assert_indistinguishable!(
        "fetch_summaries",
        |id| call!(server, super::summary::fetch_summaries, { "pmids": [id] }),
        LATER,
        UNKNOWN,
        EPUB
    );
    assert_indistinguishable!(
        "export_citations",
        |id| call!(server, super::export::export_citations, { "pmids": [id] }),
        LATER,
        UNKNOWN,
        EPUB
    );
    assert_indistinguishable!(
        "pmid_to_pmcid",
        |id| call!(server, super::convert::pmid_to_pmcid, { "pmid": id }),
        LATER,
        UNKNOWN,
        EPUB
    );
    assert_indistinguishable!(
        "get_related_articles",
        |id| call!(server, super::elink::get_related_articles, { "pmids": [id] }),
        later,
        unknown,
        31978945u32
    );
    assert_indistinguishable!(
        "get_citations",
        |id| call!(server, super::elink::get_citations, { "pmids": [id] }),
        later,
        unknown,
        2014248u32
    );
    assert_indistinguishable!(
        "get_pmc_links",
        |id| call!(server, super::elink::get_pmc_links, { "pmids": [id] }),
        later,
        unknown,
        31978945u32
    );
    assert_indistinguishable!(
        "get_pmc_markdown",
        |id| call!(server, super::markdown::get_pmc_markdown, { "pmc_id": id }),
        PMC_LATER,
        PMC_UNKNOWN,
        PMC_EPUB
    );
    assert_indistinguishable!(
        "get_pmc_fulltext",
        |id| call!(server, super::fulltext::get_pmc_fulltext, { "pmc_id": id }),
        PMC_LATER,
        PMC_UNKNOWN,
        PMC_EPUB
    );
    assert_indistinguishable!(
        "get_pmc_figures",
        |id| call!(server, super::figures::get_pmc_figures, { "pmc_id": id }),
        PMC_LATER,
        PMC_UNKNOWN,
        PMC_EPUB
    );
    assert_indistinguishable!(
        "europe_pmc_fulltext",
        |id| call!(server, super::europe_pmc::europe_pmc_fulltext, { "id": id }),
        PMC_LATER,
        PMC_UNKNOWN,
        PMC_EPUB
    );
    assert_indistinguishable!(
        "europe_pmc_references",
        |id| call!(server, super::europe_pmc::europe_pmc_references, { "id": id }),
        LATER,
        UNKNOWN,
        EPUB
    );
    assert_indistinguishable!(
        "europe_pmc_citations",
        |id| call!(server, super::europe_pmc::europe_pmc_citations, { "id": id }),
        LATER,
        UNKNOWN,
        OLD
    );
    assert_indistinguishable!(
        "europe_pmc_database_links",
        |id| call!(server, super::europe_pmc::europe_pmc_database_links, { "id": id }),
        LATER,
        UNKNOWN,
        EPUB
    );
}

/// Without `--as-of` nothing is held back: the bound is opt-in.
#[tokio::test]
async fn an_unbounded_server_returns_later_records() {
    let services = Services::start().await;
    let server = services.server(None);

    let articles = call!(server, super::articles::fetch_articles, {
        "pmids": [OLD, EPUB, LATER, BACKDATED]
    })
    .unwrap();
    assert_eq!(articles["count"], 4);

    let citations = call!(server, super::elink::get_citations, { "pmids": [2014248] }).unwrap();
    assert_eq!(
        strings(&citations["citing_pmids"]),
        [EPUB, LATER, BACKDATED]
    );

    let search = call!(server, super::europe_pmc::europe_pmc_search, {
        "query": "coronavirus", "result_type": "core"
    })
    .unwrap();
    assert_eq!(search["query"], "coronavirus");
    assert_eq!(search["count"], 3);
    assert_eq!(search["results"][2]["cited_by_count"], 640);

    // No date lookups are made when there is no bound to apply.
    assert!(!services.asked("esummary.fcgi", "").await);
}
