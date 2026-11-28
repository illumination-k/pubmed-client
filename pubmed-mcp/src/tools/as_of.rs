//! `--as-of`: make the server behave as if it ran on an earlier day.
//!
//! A backtest asks "given only what was available before date D, would an
//! agent have found X?". With `--as-of D` no tool returns a record that became
//! available on or after D, and a record that did is indistinguishable from
//! one that does not exist — reporting "3 results hidden" would itself tell
//! the agent that later evidence exists.
//!
//! "Available" is the earliest day a reader could have seen the record, not
//! its publication date: an epub-ahead-of-print article is in PubMed before
//! its issue date. So the date compared against D is
//!
//! * PubMed: the later of the record's create date (`CRDT`) and Entrez date
//!   (`EDAT`), read from ESummary's `history`;
//! * Europe PMC: `firstIndexDate` (`FIRST_IDATE`);
//!
//! and, when a record lacks that, the earliest of its other history dates. A
//! record with no usable date at all is hidden: the gate fails closed.
//!
//! # Why not the Entrez date alone
//!
//! `EDAT` is not always the day a record entered PubMed. A record added more
//! than a year after publication has its `EDAT` set back to the publication
//! date, and a year-only publication date becomes January 1st: PMID 34695337
//! was published in September 2020 and created in October 2021, yet carries
//! `EDAT` 2020-01-01 and would be "available" to a backtest pinned at February
//! 2020. The create date is when the record really appeared; `EDAT` can in
//! turn trail it by a day (created in the evening, released the next
//! morning). Requiring both to precede D admits nothing either would refuse.
//!
//! NCBI's naming invites confusion here: in a record's history the create
//! date is the entry with status `entrez`, and the Entrez date the entry with
//! status `pubmed`.
//!
//! Everything date-related lives here so the rule has one implementation; the
//! tools only ask "which of these are visible?".

use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use pubmed_client::error::ParseError;
use pubmed_client::{
    ArticleSummary, EuropePmcId, EuropePmcResult, EuropePmcSearchOptions, EuropePmcSource,
    PmcArticle, PubMedError, PubMedId, SearchDateRange, SearchDateType,
};
use rmcp::handler::server::wrapper::Json;
use rmcp::model::ErrorData;
use rmcp::schemars;
use serde::Serialize;

use super::PubMedServer;
use super::common::internal_error;

/// A calendar day. Ordered chronologically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 31,
    }
}

impl Date {
    fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        let valid = year >= 1 && (1..=12).contains(&month) && day >= 1;
        (valid && day <= days_in_month(year, month)).then_some(Self { year, month, day })
    }

    /// Parse the leading `YYYY-MM-DD` or `YYYY/MM/DD` of `text`, ignoring
    /// whatever follows (ESummary appends a time: `2020/01/25 06:00`).
    ///
    /// A partial date (`2020`, `2020/01`) is not a day and yields `None`.
    pub fn parse_prefix(text: &str) -> Option<Self> {
        let bytes = text.trim_start().as_bytes();
        if bytes.len() < 10 || bytes.get(10).is_some_and(u8::is_ascii_digit) {
            return None;
        }
        let digits = |range: std::ops::Range<usize>| -> Option<u16> {
            let part = &bytes[range];
            part.iter()
                .all(u8::is_ascii_digit)
                .then(|| part.iter().fold(0u16, |n, b| n * 10 + u16::from(b - b'0')))
        };
        let separator = bytes[4];
        if !(separator == b'-' || separator == b'/') || bytes[7] != separator {
            return None;
        }
        Self::new(digits(0..4)?, digits(5..7)? as u8, digits(8..10)? as u8)
    }

    /// The day before this one.
    fn pred(self) -> Self {
        if self.day > 1 {
            Self {
                day: self.day - 1,
                ..self
            }
        } else if self.month > 1 {
            let month = self.month - 1;
            Self {
                month,
                day: days_in_month(self.year, month),
                ..self
            }
        } else if self.year > 1 {
            Self {
                year: self.year - 1,
                month: 12,
                day: 31,
            }
        } else {
            self
        }
    }

    /// `YYYY/MM/DD`, the form E-utilities date parameters take.
    fn slashed(self) -> String {
        format!("{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }

    pub fn year(self) -> u16 {
        self.year
    }
}

impl fmt::Display for Date {
    /// ISO 8601 (`YYYY-MM-DD`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The day the server pretends to run on. Records that became available on
/// or after it are invisible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsOf(Date);

impl FromStr for AsOf {
    type Err = String;

    /// Strict ISO 8601 calendar date. Anything looser (`2020-1-5`, `2020/01/05`,
    /// a bare year) is rejected rather than guessed at: a misread cutoff
    /// silently invalidates the backtest it was meant to bound.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || format!("'{text}' is not a valid date; expected YYYY-MM-DD");
        if text.len() != 10 || text.as_bytes()[4] != b'-' {
            return Err(invalid());
        }
        Date::parse_prefix(text).map(AsOf).ok_or_else(invalid)
    }
}

impl fmt::Display for AsOf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsOf {
    /// The last day whose records are visible (D-1).
    pub fn last_visible_day(self) -> Date {
        self.0.pred()
    }

    /// Whether a record first available on `date` is visible. A record with
    /// no known date is not.
    pub fn admits(self, date: Option<Date>) -> bool {
        date.is_some_and(|date| date < self.0)
    }

    /// The E-utilities date range that narrows an ESearch or ELink answer to
    /// records created before D.
    ///
    /// The services take one date type per request, so this is the create
    /// date — the one that shuts out backdated records — and the tools still
    /// pass what comes back through [`PubMedServer::visible_pmids`], which
    /// applies the whole rule.
    pub fn eutils_range(self) -> SearchDateRange {
        SearchDateRange::until(SearchDateType::Create, self.last_visible_day().slashed())
    }

    /// The Europe PMC query clause that bounds a search to visible records.
    pub fn europe_pmc_clause(self) -> String {
        format!("FIRST_IDATE:[* TO {}]", self.last_visible_day())
    }

    /// One line for `filters_applied` and the server instructions.
    pub fn describe(self) -> String {
        format!(
            "As of {self}: only records available before this date are returned (PubMed create and Entrez dates, Europe PMC first index date)"
        )
    }
}

/// The earliest date in `dates` that parses as a full day.
fn earliest<'a>(dates: impl IntoIterator<Item = &'a str>) -> Option<Date> {
    dates.into_iter().filter_map(Date::parse_prefix).min()
}

/// The day a PubMed record became available: the later of its create date
/// and its Entrez date (see the module docs for why both), or failing those
/// the earliest date in its history.
pub fn pubmed_available_on(summary: &ArticleSummary) -> Option<Date> {
    let milestone = |status: &str| {
        summary
            .history
            .iter()
            .find(|entry| entry.pub_status.eq_ignore_ascii_case(status))
            .and_then(|entry| Date::parse_prefix(&entry.date))
    };
    // History status `entrez` is the create date, `pubmed` the Entrez date.
    match (milestone("entrez"), milestone("pubmed")) {
        (Some(created), Some(entered)) => Some(created.max(entered)),
        (Some(date), None) | (None, Some(date)) => Some(date),
        (None, None) => earliest(summary.history.iter().map(|entry| entry.date.as_str())),
    }
}

/// The day a Europe PMC record became available: its first index date, or
/// failing that the earliest of the other dates the record carries.
pub fn europe_pmc_available_on(record: &EuropePmcResult) -> Option<Date> {
    let field = |name: &str| record.extra.get(name).and_then(|value| value.as_str());
    field("firstIndexDate")
        .and_then(Date::parse_prefix)
        .or_else(|| {
            earliest(
                [
                    "firstPublicationDate",
                    "electronicPublicationDate",
                    "dateOfCreation",
                ]
                .into_iter()
                .filter_map(field),
            )
        })
}

/// A tool's answer plus the as-of date it was produced under.
///
/// Every tool returns this, so an agent (and the person reading its
/// transcript) can see that the view is bounded without the server having to
/// say what the bound removed.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct Bounded<T> {
    #[serde(flatten)]
    pub output: T,
    /// Set when the server runs with `--as-of`: the date (YYYY-MM-DD) the
    /// server behaves as if it ran on. Records that became available on or
    /// after it are not returned and look like records that do not exist.
    /// Absent when the server is unbounded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of: Option<String>,
}

/// The error a PMC / Europe PMC full-text tool answers with when the record
/// is not visible. Built from the client's own "not available" error so a
/// hidden record reads the same as one the service does not have.
pub fn not_available(context: &str, id: &str) -> ErrorData {
    let error = PubMedError::from(ParseError::PmcNotAvailable { id: id.to_string() });
    internal_error(format!("{context}: {error}"))
}

/// Characters a Europe PMC identifier or source code may consist of. An id
/// is spliced into a query to look its date up, so anything that could act
/// as query syntax disqualifies it (such a record is simply "not found").
fn is_plain_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Whether parentheses and quotes in a Europe PMC query are balanced.
///
/// The as-of clause is ANDed onto the *parenthesised* user query. An
/// unbalanced query such as `a) OR (b` would close that parenthesis early and
/// leave `OR (b` outside the bound, so it is rejected up front.
pub fn is_balanced_query(query: &str) -> bool {
    let mut depth = 0usize;
    let mut in_quotes = false;
    let mut chars = query.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            '(' if !in_quotes => depth += 1,
            ')' if !in_quotes => {
                let Some(shallower) = depth.checked_sub(1) else {
                    return false;
                };
                depth = shallower;
            }
            _ => {}
        }
    }
    depth == 0 && !in_quotes
}

/// Europe PMC accepts long queries, but they travel in a GET URL; this keeps
/// each lookup comfortably short.
const EUROPE_PMC_LOOKUP_CHUNK: usize = 50;

impl PubMedServer {
    /// Wrap a tool's answer with the as-of date in force.
    pub(crate) fn bounded<T>(
        &self,
        result: Result<Json<T>, ErrorData>,
    ) -> Result<Json<Bounded<T>>, ErrorData> {
        result.map(|Json(output)| {
            Json(Bounded {
                output,
                as_of: self.as_of.map(|as_of| as_of.to_string()),
            })
        })
    }

    /// Which of `pmids` are visible under `as_of`.
    ///
    /// One ESummary round trip per 200 PMIDs. A PMID that ESummary has no
    /// record for is simply absent from the answer, exactly like a hidden one.
    pub(crate) async fn visible_pmids(
        &self,
        as_of: AsOf,
        pmids: impl IntoIterator<Item = u32>,
    ) -> Result<HashSet<u32>, ErrorData> {
        let mut unique: Vec<u32> = pmids.into_iter().collect();
        unique.sort_unstable();
        unique.dedup();
        if unique.is_empty() {
            return Ok(HashSet::new());
        }

        let ids: Vec<String> = unique.iter().map(u32::to_string).collect();
        let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let summaries = self
            .client
            .pubmed
            .fetch_summaries(&id_refs)
            .await
            .map_err(|e| internal_error(format!("Failed to look up record dates: {e}")))?;

        Ok(summaries
            .iter()
            .filter(|summary| as_of.admits(pubmed_available_on(summary)))
            .filter_map(|summary| PubMedId::parse(&summary.pmid).ok())
            .map(|pmid| pmid.as_u32())
            .collect())
    }

    /// Whether the PubMed record behind a PMC article is visible.
    ///
    /// An article with no PubMed record has no Entrez date to check and is
    /// treated as not visible.
    pub(crate) async fn pmc_article_is_visible(
        &self,
        as_of: AsOf,
        article: &PmcArticle,
    ) -> Result<bool, ErrorData> {
        let Some(pmid) = article.pmid().map(|pmid| pmid.as_u32()) else {
            return Ok(false);
        };
        Ok(self.visible_pmids(as_of, [pmid]).await?.contains(&pmid))
    }

    /// Whether a Europe PMC record is visible under `as_of`. A record Europe
    /// PMC does not have is not.
    pub(crate) async fn europe_pmc_record_is_visible(
        &self,
        as_of: AsOf,
        id: &EuropePmcId,
    ) -> Result<bool, ErrorData> {
        Ok(self.visible_europe_pmc_record(as_of, id).await?.is_some())
    }

    /// The record `id` addresses, if it is visible under `as_of`.
    pub(crate) async fn visible_europe_pmc_record(
        &self,
        as_of: AsOf,
        id: &EuropePmcId,
    ) -> Result<Option<EuropePmcResult>, ErrorData> {
        let source = id.source.as_str();
        if !is_plain_token(&id.id) || !is_plain_token(source) {
            return Ok(None);
        }
        // A PMC id addresses the full text of a record that Europe PMC
        // usually files under MED; `PMCID:` finds it under either source.
        let by_pmcid = id.source == EuropePmcSource::Pmc;
        let query = if by_pmcid {
            format!("PMCID:{}", id.id)
        } else {
            format!("EXT_ID:{} AND SRC:{source}", id.id)
        };

        let records = self.europe_pmc_lookup(&query).await?;
        // Match the answer against the id rather than trusting the query to
        // have selected exactly this record.
        Ok(records.into_iter().find(|record| {
            let same_record = if by_pmcid {
                record
                    .pmcid
                    .as_deref()
                    .is_some_and(|pmcid| pmcid.eq_ignore_ascii_case(&id.id))
            } else {
                record.source.eq_ignore_ascii_case(source) && record.id == id.id
            };
            same_record && as_of.admits(europe_pmc_available_on(record))
        }))
    }

    /// Which of the `(source, id)` pairs are visible under `as_of`.
    pub(crate) async fn visible_europe_pmc_records(
        &self,
        as_of: AsOf,
        records: impl IntoIterator<Item = (String, String)>,
    ) -> Result<HashSet<(String, String)>, ErrorData> {
        let mut wanted: Vec<(String, String)> = records
            .into_iter()
            .filter(|(source, id)| is_plain_token(source) && is_plain_token(id))
            .collect();
        wanted.sort();
        wanted.dedup();

        let mut visible = HashSet::new();
        for chunk in wanted.chunks(EUROPE_PMC_LOOKUP_CHUNK) {
            let mut sources: Vec<&str> = chunk.iter().map(|(source, _)| source.as_str()).collect();
            sources.dedup();
            let query = sources
                .iter()
                .map(|source| {
                    let ids: Vec<&str> = chunk
                        .iter()
                        .filter(|(s, _)| s == source)
                        .map(|(_, id)| id.as_str())
                        .collect();
                    format!("(SRC:{source} AND EXT_ID:({}))", ids.join(" OR "))
                })
                .collect::<Vec<_>>()
                .join(" OR ");

            for record in self.europe_pmc_lookup(&query).await? {
                let key = (record.source.clone(), record.id.clone());
                if chunk.contains(&key) && as_of.admits(europe_pmc_available_on(&record)) {
                    visible.insert(key);
                }
            }
        }
        Ok(visible)
    }

    /// Run a date lookup query. One page is always enough — a lookup names
    /// at most [`EUROPE_PMC_LOOKUP_CHUNK`] records — and `lite` records carry
    /// `firstIndexDate`.
    async fn europe_pmc_lookup(&self, query: &str) -> Result<Vec<EuropePmcResult>, ErrorData> {
        let opts = EuropePmcSearchOptions {
            page_size: (EUROPE_PMC_LOOKUP_CHUNK * 2) as u32,
            ..Default::default()
        };
        self.client
            .europe_pmc
            .search_page(query, &opts)
            .await
            .map(|page| page.results)
            .map_err(|e| internal_error(format!("Failed to look up record dates: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubmed_client::ArticleHistoryDate;

    fn date(text: &str) -> Date {
        Date::parse_prefix(text).unwrap_or_else(|| panic!("{text} should parse"))
    }

    fn as_of(text: &str) -> AsOf {
        text.parse().unwrap()
    }

    #[test]
    fn as_of_accepts_only_strict_iso_dates() {
        assert_eq!(as_of("2020-02-01").to_string(), "2020-02-01");
        assert_eq!(as_of("2024-02-29").to_string(), "2024-02-29");

        for invalid in [
            "",
            "2020",
            "2020-02",
            "2020/02/01",
            "2020-2-1",
            "20200201",
            "2020-02-30",
            "2023-02-29",
            "2020-13-01",
            "2020-00-10",
            "2020-02-01T00:00:00Z",
            " 2020-02-01",
            "yesterday",
        ] {
            let err = invalid.parse::<AsOf>().expect_err(invalid);
            assert!(err.contains("YYYY-MM-DD"), "{err}");
        }
    }

    #[test]
    fn last_visible_day_crosses_month_year_and_leap_boundaries() {
        for (cutoff, last) in [
            ("2020-02-15", "2020-02-14"),
            ("2020-03-01", "2020-02-29"),
            ("2021-03-01", "2021-02-28"),
            ("1900-03-01", "1900-02-28"),
            ("2000-03-01", "2000-02-29"),
            ("2020-01-01", "2019-12-31"),
            ("2020-05-01", "2020-04-30"),
        ] {
            assert_eq!(as_of(cutoff).last_visible_day(), date(last), "{cutoff}");
        }
    }

    #[test]
    fn the_cutoff_day_itself_is_hidden() {
        let as_of = as_of("2020-02-01");
        assert!(as_of.admits(Some(date("2020-01-31"))));
        assert!(!as_of.admits(Some(date("2020-02-01"))));
        assert!(!as_of.admits(Some(date("2020-02-02"))));
        assert!(!as_of.admits(None), "an undated record must fail closed");
    }

    #[test]
    fn source_dates_are_read_in_both_spellings() {
        assert_eq!(date("2020/01/25 06:00"), date("2020-01-25"));
        for partial in [
            "2020",
            "2020/01",
            "2020 Jan 24",
            "",
            "2020-01-2",
            "2020/01-25",
        ] {
            assert_eq!(Date::parse_prefix(partial), None, "{partial}");
        }
    }

    #[test]
    fn bounds_are_rendered_in_each_service_s_syntax() {
        let as_of = as_of("2020-02-01");
        let range = as_of.eutils_range();
        assert_eq!(range.date_type, SearchDateType::Create);
        assert_eq!(range.min_date, "1000/01/01");
        assert_eq!(range.max_date, "2020/01/31");
        assert_eq!(as_of.europe_pmc_clause(), "FIRST_IDATE:[* TO 2020-01-31]");
    }

    fn summary(history: &[(&str, &str)]) -> ArticleSummary {
        let json = serde_json::json!({
            "pmid": "1", "title": "", "authors": [], "journal": "", "full_journal_name": "",
            "pub_date": "", "epub_date": "", "doi": null, "pmc_id": null, "volume": "",
            "issue": "", "pages": "", "languages": [], "pub_types": [], "issn": "", "essn": "",
            "sort_pub_date": "", "pmc_ref_count": 0, "record_status": "",
        });
        let mut summary: ArticleSummary = serde_json::from_value(json).unwrap();
        summary.history = history
            .iter()
            .map(|(pub_status, date)| ArticleHistoryDate {
                pub_status: pub_status.to_string(),
                date: date.to_string(),
            })
            .collect();
        summary
    }

    #[test]
    fn pubmed_availability_is_the_later_of_create_and_entrez_dates() {
        // Usual case: created and entered PubMed the same day.
        let usual = summary(&[
            ("received", "2019/11/02 00:00"),
            ("pubmed", "2020/01/25 06:00"),
            ("medline", "2020/03/07 06:00"),
            ("entrez", "2020/01/25 06:00"),
        ]);
        assert_eq!(pubmed_available_on(&usual), Some(date("2020-01-25")));

        // Created in the evening, in PubMed the next morning.
        let overnight = summary(&[
            ("pubmed", "2021/01/31 06:00"),
            ("entrez", "2021/01/30 20:08"),
        ]);
        assert_eq!(pubmed_available_on(&overnight), Some(date("2021-01-31")));

        // PMID 34695337: added a year late, Entrez date set back to January
        // 1st of the publication year. The create date is the truth.
        let backdated = summary(&[
            ("entrez", "2021/10/25 17:36"),
            ("pubmed", "2020/01/01 00:00"),
        ]);
        assert_eq!(pubmed_available_on(&backdated), Some(date("2021-10-25")));
        assert!(!as_of("2020-02-01").admits(pubmed_available_on(&backdated)));
    }

    #[test]
    fn pubmed_availability_falls_back_when_a_date_is_missing() {
        let only_entrez = summary(&[
            ("pubmed", "2020/01/26 06:00"),
            ("medline", "2020/03/07 06:00"),
        ]);
        assert_eq!(pubmed_available_on(&only_entrez), Some(date("2020-01-26")));

        // Neither date: the earliest of whatever history the record has.
        let neither = summary(&[
            ("medline", "2020/03/07 06:00"),
            ("accepted", "2020/01/10 00:00"),
        ]);
        assert_eq!(pubmed_available_on(&neither), Some(date("2020-01-10")));
        assert_eq!(pubmed_available_on(&summary(&[])), None);
    }

    fn europe_pmc_record(fields: serde_json::Value) -> EuropePmcResult {
        serde_json::from_value(fields).unwrap()
    }

    #[test]
    fn europe_pmc_availability_is_the_first_index_date() {
        let record = europe_pmc_record(serde_json::json!({
            "id": "1", "source": "MED",
            "firstIndexDate": "2021-03-06", "firstPublicationDate": "2021-12-01",
        }));
        assert_eq!(europe_pmc_available_on(&record), Some(date("2021-03-06")));
    }

    #[test]
    fn europe_pmc_availability_falls_back_to_the_earliest_other_date() {
        let record = europe_pmc_record(serde_json::json!({
            "id": "1", "source": "MED",
            "firstPublicationDate": "2021-12-01", "electronicPublicationDate": "2021-03-04",
        }));
        assert_eq!(europe_pmc_available_on(&record), Some(date("2021-03-04")));
        let undated = europe_pmc_record(serde_json::json!({ "id": "1", "source": "MED" }));
        assert_eq!(europe_pmc_available_on(&undated), None);
    }

    #[test]
    fn a_query_that_could_escape_the_bound_is_unbalanced() {
        for balanced in [
            "malaria vaccine",
            "(a OR b) AND c",
            r#"TITLE:"a (b" AND c"#,
            r"a \) b",
            "FIRST_PDATE:[2000-01-01 TO 2010-01-01]",
        ] {
            assert!(is_balanced_query(balanced), "{balanced}");
        }
        for unbalanced in ["a) OR (b", "(a", "a)", r#"a" OR (b"#, r#""a"#] {
            assert!(!is_balanced_query(unbalanced), "{unbalanced}");
        }
    }

    #[test]
    fn ids_that_could_act_as_query_syntax_are_not_plain() {
        for plain in ["33515491", "PMC7906746", "PPR123456", "WO2020123456", "MED"] {
            assert!(is_plain_token(plain), "{plain}");
        }
        for odd in ["", "1 OR 2", "1)", "a:b", "\"1\"", "1*"] {
            assert!(!is_plain_token(odd), "{odd}");
        }
    }
}
