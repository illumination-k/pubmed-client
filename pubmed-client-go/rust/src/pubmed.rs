//! PubMed E-utilities operations (ESearch, EFetch, ESummary, ELink, EInfo,
//! ECitMatch, EGQuery, ESpell).
//!
//! Every function follows the conventions in the crate docs: a client handle, a
//! nullable cancellation token, and an `out_err`. Results are JSON built from
//! the `pubmed-client` models, which are `Serialize` and therefore cross
//! unchanged unless [`crate::dto`] says otherwise.

use std::ffi::c_char;

use pubmed_client::CitationQuery;

use crate::cancel::{PubmedCancel, block_on};
use crate::client::{PubmedClient, borrow_client};
use crate::dto::{CitationQueryDto, SearchFullTextResultDto};
use crate::error::ShimResult;
use crate::ffi::{borrow_opt_str, borrow_str, guard, parse_json_arg, to_json};
use crate::query::parse_sort;
use pubmed_client::SortOrder;

/// Resolve the optional `sort` argument shared by the search entry points.
///
/// # Safety
///
/// `sort` must be null or a valid NUL-terminated string.
unsafe fn optional_sort(sort: *const c_char) -> ShimResult<Option<SortOrder>> {
    match unsafe { borrow_opt_str(sort, "sort") }? {
        None => Ok(None),
        Some(value) => parse_sort(value).map(Some),
    }
}

/// Search PubMed, returning a JSON array of PMIDs.
///
/// `sort` may be null for PubMed's default (relevance) ordering.
///
/// # Safety
///
/// See the module-level boundary conventions: `handle` must be live, `query` a
/// valid NUL-terminated string, `cancel` null or live, `out_err` null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_search_articles(
    handle: *const PubmedClient,
    query: *const c_char,
    limit: usize,
    sort: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let query = unsafe { borrow_str(query, "query") }?;
        let sort = unsafe { optional_sort(sort) }?;

        let pmids = unsafe {
            block_on(
                cancel,
                client.pubmed.search_articles(query, limit, sort.as_ref()),
            )
        }?;
        to_json(&pmids)
    })
}

/// Fetch full metadata for a single PMID, returning a JSON object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_fetch_article(
    handle: *const PubmedClient,
    pmid: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmid = unsafe { borrow_str(pmid, "pmid") }?;

        let article = unsafe { block_on(cancel, client.pubmed.fetch_article(pmid)) }?;
        to_json(&article)
    })
}

/// Fetch metadata for several PMIDs, given a JSON array of PMID strings.
/// Returns a JSON array of articles.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_fetch_articles(
    handle: *const PubmedClient,
    pmids_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmids: Vec<String> = unsafe { parse_json_arg(pmids_json, "pmids_json") }?;
        let pmid_refs: Vec<&str> = pmids.iter().map(String::as_str).collect();

        let articles = unsafe { block_on(cancel, client.pubmed.fetch_articles(&pmid_refs)) }?;
        to_json(&articles)
    })
}

/// Fetch metadata for an arbitrarily large PMID list by way of the history
/// server. Returns a JSON array of articles.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_fetch_all_by_pmids(
    handle: *const PubmedClient,
    pmids_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmids: Vec<String> = unsafe { parse_json_arg(pmids_json, "pmids_json") }?;
        let pmid_refs: Vec<&str> = pmids.iter().map(String::as_str).collect();

        let articles = unsafe { block_on(cancel, client.fetch_all_by_pmids(&pmid_refs)) }?;
        to_json(&articles)
    })
}

/// Search PubMed and fetch metadata for each hit, returning a JSON array of
/// articles.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_search_and_fetch(
    handle: *const PubmedClient,
    query: *const c_char,
    limit: usize,
    sort: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let query = unsafe { borrow_str(query, "query") }?;
        let sort = unsafe { optional_sort(sort) }?;

        let articles = unsafe {
            block_on(
                cancel,
                client.pubmed.search_and_fetch(query, limit, sort.as_ref()),
            )
        }?;
        to_json(&articles)
    })
}

/// Fetch lightweight ESummary records for a JSON array of PMIDs. Returns a JSON
/// array of summaries.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_fetch_summaries(
    handle: *const PubmedClient,
    pmids_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmids: Vec<String> = unsafe { parse_json_arg(pmids_json, "pmids_json") }?;
        let pmid_refs: Vec<&str> = pmids.iter().map(String::as_str).collect();

        let summaries = unsafe { block_on(cancel, client.fetch_summaries(&pmid_refs)) }?;
        to_json(&summaries)
    })
}

/// Search PubMed and fetch an ESummary record for each hit. Returns a JSON
/// array of summaries.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_search_and_fetch_summaries(
    handle: *const PubmedClient,
    query: *const c_char,
    limit: usize,
    sort: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let query = unsafe { borrow_str(query, "query") }?;
        let sort = unsafe { optional_sort(sort) }?;

        let summaries = unsafe {
            block_on(
                cancel,
                client
                    .pubmed
                    .search_and_fetch_summaries(query, limit, sort.as_ref()),
            )
        }?;
        to_json(&summaries)
    })
}

/// Search PubMed and attach PMC full text where it is available. Returns a JSON
/// array of `{article, full_text}` objects.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_search_with_full_text(
    handle: *const PubmedClient,
    query: *const c_char,
    limit: usize,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let query = unsafe { borrow_str(query, "query") }?;

        let results = unsafe { block_on(cancel, client.search_with_full_text(query, limit)) }?;
        let dtos: Vec<SearchFullTextResultDto> = results
            .iter()
            .map(|(article, full_text)| SearchFullTextResultDto::new(article, full_text.as_ref()))
            .collect();
        to_json(&dtos)
    })
}

/// Find articles related to a JSON array of PMIDs (ELink). Returns a JSON
/// `RelatedArticles` object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_get_related_articles(
    handle: *const PubmedClient,
    pmids_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmids: Vec<u32> = unsafe { parse_json_arg(pmids_json, "pmids_json") }?;

        let related = unsafe { block_on(cancel, client.get_related_articles(&pmids)) }?;
        to_json(&related)
    })
}

/// Find PMC full text for a JSON array of PMIDs (ELink). Returns a JSON
/// `PmcLinks` object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_get_pmc_links(
    handle: *const PubmedClient,
    pmids_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmids: Vec<u32> = unsafe { parse_json_arg(pmids_json, "pmids_json") }?;

        let links = unsafe { block_on(cancel, client.get_pmc_links(&pmids)) }?;
        to_json(&links)
    })
}

/// Find articles citing a JSON array of PMIDs (ELink). Returns a JSON
/// `Citations` object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_get_citations(
    handle: *const PubmedClient,
    pmids_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmids: Vec<u32> = unsafe { parse_json_arg(pmids_json, "pmids_json") }?;

        let citations = unsafe { block_on(cancel, client.get_citations(&pmids)) }?;
        to_json(&citations)
    })
}

/// List the available NCBI databases (EInfo). Returns a JSON array of names.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_get_database_list(
    handle: *const PubmedClient,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let databases = unsafe { block_on(cancel, client.get_database_list()) }?;
        to_json(&databases)
    })
}

/// Describe one NCBI database (EInfo). Returns a JSON `DatabaseInfo` object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_get_database_info(
    handle: *const PubmedClient,
    database: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let database = unsafe { borrow_str(database, "database") }?;

        let info = unsafe { block_on(cancel, client.get_database_info(database)) }?;
        to_json(&info)
    })
}

/// Spell-check a search term (ESpell). `database` may be null for PubMed.
/// Returns a JSON `SpellCheckResult` object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_spell_check(
    handle: *const PubmedClient,
    term: *const c_char,
    database: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let term = unsafe { borrow_str(term, "term") }?;
        let database = unsafe { borrow_opt_str(database, "database") }?;

        let result = match database {
            Some(database) => {
                unsafe { block_on(cancel, client.pubmed.spell_check_db(term, database)) }?
            }
            None => unsafe { block_on(cancel, client.spell_check(term)) }?,
        };
        to_json(&result)
    })
}

/// Count matches for a term across every Entrez database (EGQuery). Returns a
/// JSON `GlobalQueryResults` object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_global_query(
    handle: *const PubmedClient,
    term: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let term = unsafe { borrow_str(term, "term") }?;

        let results = unsafe { block_on(cancel, client.global_query(term)) }?;
        to_json(&results)
    })
}

/// Resolve citations to PMIDs (ECitMatch), given a JSON array of citation
/// queries. Returns a JSON `CitationMatches` object.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_match_citations(
    handle: *const PubmedClient,
    citations_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let dtos: Vec<CitationQueryDto> =
            unsafe { parse_json_arg(citations_json, "citations_json") }?;
        let citations: Vec<CitationQuery> = dtos.into_iter().map(CitationQuery::from).collect();

        let matches = unsafe { block_on(cancel, client.match_citations(&citations)) }?;
        to_json(&matches)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::NO_CANCEL;
    use crate::client::{pubmed_client_free, pubmed_client_new};
    use crate::ffi::pubmed_string_free;
    use std::ffi::CStr;
    use std::ptr;

    /// A client with the library defaults. Every test below fails before any
    /// request is attempted, so no network is involved.
    struct TestClient(*mut PubmedClient);

    impl TestClient {
        fn new() -> Self {
            let mut err: *mut c_char = ptr::null_mut();
            let handle = unsafe { pubmed_client_new(ptr::null(), &mut err) };
            assert!(!handle.is_null(), "failed to create a client");
            Self(handle)
        }
    }

    impl Drop for TestClient {
        fn drop(&mut self) {
            unsafe { pubmed_client_free(self.0) };
        }
    }

    /// Read and free an error envelope, returning its `kind`.
    fn error_kind(err: *mut c_char) -> String {
        assert!(!err.is_null(), "expected an error envelope");
        let raw = unsafe { CStr::from_ptr(err) }.to_string_lossy().to_string();
        unsafe { pubmed_string_free(err) };

        let parsed: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|_| panic!("not an envelope: {raw}"));
        parsed["kind"]
            .as_str()
            .unwrap_or_else(|| panic!("no kind in {raw}"))
            .to_string()
    }

    #[test]
    fn calls_reject_a_null_handle() {
        let mut err: *mut c_char = ptr::null_mut();
        let result = unsafe {
            pubmed_search_articles(
                ptr::null(),
                c"cancer".as_ptr(),
                1,
                ptr::null(),
                NO_CANCEL,
                &mut err,
            )
        };
        assert!(result.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    #[test]
    fn calls_reject_a_null_string_argument() {
        let client = TestClient::new();
        let mut err: *mut c_char = ptr::null_mut();
        let result = unsafe {
            pubmed_search_articles(client.0, ptr::null(), 1, ptr::null(), NO_CANCEL, &mut err)
        };
        assert!(result.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    #[test]
    fn an_unknown_sort_is_rejected_before_any_request() {
        let client = TestClient::new();
        let mut err: *mut c_char = ptr::null_mut();
        let result = unsafe {
            pubmed_search_articles(
                client.0,
                c"cancer".as_ptr(),
                1,
                c"sideways".as_ptr(),
                NO_CANCEL,
                &mut err,
            )
        };
        assert!(result.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    #[test]
    fn fetch_articles_rejects_a_malformed_pmid_array() {
        let client = TestClient::new();
        let mut err: *mut c_char = ptr::null_mut();
        let result =
            unsafe { pubmed_fetch_articles(client.0, c"[1, 2]".as_ptr(), NO_CANCEL, &mut err) };
        assert!(result.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    #[test]
    fn elink_calls_require_numeric_pmids() {
        let client = TestClient::new();
        let mut err: *mut c_char = ptr::null_mut();
        // The ELink API takes numeric UIDs, so quoted PMIDs are a caller bug.
        let result = unsafe {
            pubmed_get_related_articles(client.0, c"[\"31978945\"]".as_ptr(), NO_CANCEL, &mut err)
        };
        assert!(result.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    #[test]
    fn match_citations_rejects_a_malformed_citation() {
        let client = TestClient::new();
        let mut err: *mut c_char = ptr::null_mut();
        let result = unsafe {
            pubmed_match_citations(
                client.0,
                c"[{\"jrnl\":\"x\"}]".as_ptr(),
                NO_CANCEL,
                &mut err,
            )
        };
        assert!(result.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    /// A pre-fired token must short-circuit before the request goes out, which
    /// is what keeps a cancelled Go context from waiting on the network.
    #[test]
    fn a_pre_fired_token_cancels_the_call() {
        use crate::cancel::{pubmed_cancel_free, pubmed_cancel_new, pubmed_cancel_trigger};

        let client = TestClient::new();
        let token = pubmed_cancel_new();
        unsafe { pubmed_cancel_trigger(token) };

        let mut err: *mut c_char = ptr::null_mut();
        let result = unsafe {
            pubmed_search_articles(
                client.0,
                c"cancer".as_ptr(),
                1,
                ptr::null(),
                token,
                &mut err,
            )
        };

        assert!(result.is_null());
        assert_eq!(error_kind(err), "cancelled");
        unsafe { pubmed_cancel_free(token) };
    }
}
