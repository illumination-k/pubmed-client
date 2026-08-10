//! C-ABI shim exposing the PubMed / PMC client to Go via cgo.
//!
//! The public surface is intentionally small (an MVP, mirroring the R
//! bindings): create a client, search PubMed, fetch article metadata, and
//! retrieve PMC full text / Markdown. The ergonomic API lives in the Go package
//! one directory up.
//!
//! # Boundary conventions
//!
//! Rather than mirroring every Rust type in C, values cross the boundary as
//! JSON: each call returns a freshly allocated NUL-terminated C string that the
//! caller owns and must release with [`pubmed_string_free`]. A null return
//! signals failure, in which case `out_err` receives an owned message (also
//! freed with [`pubmed_string_free`]). Go re-parses the JSON into the typed
//! structs in `models.go`, which keeps the FFI surface a handful of functions
//! while the data model stays fully typed on both sides.
//!
//! Like the Python and R bindings, calls are synchronous from the caller's
//! point of view: a process-wide Tokio runtime drives the async `pubmed-client`
//! API and blocks until completion.
//!
//! Panics are caught at every boundary function and reported as errors — an
//! `extern "C"` function that unwinds would abort the whole Go process.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use pubmed_client::{
    Author, Client, ClientConfig, Figure, JournalMeta, PmcArticle, PmcMarkdownConverter, Reference,
    Section,
};

// ------------------------------------------------------------------------------------------------
// Runtime management
// ------------------------------------------------------------------------------------------------

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Get or create the process-wide Tokio runtime used to block on async calls.
///
/// A single shared runtime keeps connection pools and the NCBI rate limiter
/// alive across calls, mirroring the Python and R bindings.
#[allow(clippy::expect_used)]
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("failed to create Tokio runtime"))
}

// ------------------------------------------------------------------------------------------------
// Opaque handle
// ------------------------------------------------------------------------------------------------

/// Opaque client handle handed to Go as a raw pointer.
///
/// Created by [`pubmed_client_new`] and released by [`pubmed_client_free`].
pub struct PubmedClient {
    inner: Arc<Client>,
}

/// Client configuration decoded from the JSON blob Go passes to
/// [`pubmed_client_new`]. Every field is optional; omitted values fall back to
/// the `pubmed-client` defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ConfigDto {
    api_key: Option<String>,
    email: Option<String>,
    tool: Option<String>,
    rate_limit: Option<f64>,
    timeout_seconds: Option<u64>,
    user_agent: Option<String>,
    base_url: Option<String>,
    cache: bool,
}

impl ConfigDto {
    fn into_config(self) -> ClientConfig {
        let mut config = ClientConfig::new();
        if let Some(api_key) = self.api_key {
            config = config.with_api_key(api_key);
        }
        if let Some(email) = self.email {
            config = config.with_email(email);
        }
        if let Some(tool) = self.tool {
            config = config.with_tool(tool);
        }
        if let Some(rate_limit) = self.rate_limit {
            config = config.with_rate_limit(rate_limit);
        }
        if let Some(timeout_seconds) = self.timeout_seconds {
            config = config.with_timeout_seconds(timeout_seconds);
        }
        if let Some(user_agent) = self.user_agent {
            config = config.with_user_agent(user_agent);
        }
        if let Some(base_url) = self.base_url {
            config = config.with_base_url(base_url);
        }
        if self.cache {
            config = config.with_cache();
        }
        config
    }
}

// ------------------------------------------------------------------------------------------------
// String / pointer helpers
// ------------------------------------------------------------------------------------------------

/// Copy a Rust string into a freshly allocated C string owned by the caller.
fn into_c_string(value: String) -> Result<*mut c_char, String> {
    CString::new(value)
        .map(CString::into_raw)
        .map_err(|_| "result contained an interior NUL byte".to_string())
}

/// Serialize `value` as JSON for transport across the boundary.
fn to_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| format!("failed to serialize response: {e}"))
}

/// Borrow a NUL-terminated C string as a `&str`.
///
/// # Safety
///
/// `value` must be null, or point to a valid NUL-terminated string that stays
/// alive for as long as the returned reference is used.
unsafe fn borrow_str<'a>(value: *const c_char, name: &str) -> Result<&'a str, String> {
    if value.is_null() {
        return Err(format!("{name} must not be null"));
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| format!("{name} must be valid UTF-8"))
}

/// Borrow a client handle.
///
/// # Safety
///
/// `handle` must be null, or a pointer returned by [`pubmed_client_new`] that
/// has not yet been passed to [`pubmed_client_free`].
unsafe fn borrow_client<'a>(handle: *const PubmedClient) -> Result<&'a PubmedClient, String> {
    if handle.is_null() {
        return Err("client handle must not be null".to_string());
    }
    Ok(unsafe { &*handle })
}

/// Reset `out_err` to null so a stale message can never be mistaken for a fresh
/// one.
///
/// # Safety
///
/// `out_err` must be null or point to a writable `*mut c_char`.
unsafe fn clear_error(out_err: *mut *mut c_char) {
    if !out_err.is_null() {
        unsafe { *out_err = ptr::null_mut() };
    }
}

/// Store an owned error message in `out_err`.
///
/// # Safety
///
/// `out_err` must be null or point to a writable `*mut c_char`.
unsafe fn set_error(out_err: *mut *mut c_char, message: String) {
    if out_err.is_null() {
        return;
    }
    let owned =
        CString::new(message).unwrap_or_else(|_| c"error contained an interior NUL byte".into());
    unsafe { *out_err = owned.into_raw() };
}

/// Run `body` under the FFI error convention: a freshly allocated C string on
/// success, or null with `*out_err` set on failure.
///
/// Panics are caught and converted into errors so they never unwind into Go.
fn guard<F>(out_err: *mut *mut c_char, body: F) -> *mut c_char
where
    F: FnOnce() -> Result<String, String>,
{
    unsafe { clear_error(out_err) };

    let outcome = catch_unwind(AssertUnwindSafe(body))
        .unwrap_or_else(|_| Err("panic inside pubmed-client".to_string()));

    match outcome.and_then(into_c_string) {
        Ok(value) => value,
        Err(message) => {
            unsafe { set_error(out_err, message) };
            ptr::null_mut()
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Lifecycle
// ------------------------------------------------------------------------------------------------

/// Version of the underlying Rust crate, as a static NUL-terminated string.
///
/// The returned pointer is borrowed and must NOT be freed.
#[unsafe(no_mangle)]
pub extern "C" fn pubmed_client_version() -> *const c_char {
    const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr().cast()
}

/// Create a client from a JSON configuration blob (see `ConfigDto`).
///
/// Pass null for `config_json` to use the library defaults. Returns null and
/// sets `*out_err` on failure. The handle must be released with
/// [`pubmed_client_free`].
///
/// # Safety
///
/// `config_json` must be null or a valid NUL-terminated string, and `out_err`
/// must be null or point to a writable `*mut c_char`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_client_new(
    config_json: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut PubmedClient {
    unsafe { clear_error(out_err) };

    let outcome = catch_unwind(AssertUnwindSafe(|| -> Result<*mut PubmedClient, String> {
        let config = if config_json.is_null() {
            ConfigDto::default()
        } else {
            let raw = unsafe { borrow_str(config_json, "config_json") }?;
            serde_json::from_str(raw).map_err(|e| format!("invalid client config: {e}"))?
        };

        // Enter the runtime: the HTTP client builder expects a reactor context.
        let _guard = runtime().enter();
        let client = Client::with_config(config.into_config());
        Ok(Box::into_raw(Box::new(PubmedClient {
            inner: Arc::new(client),
        })))
    }))
    .unwrap_or_else(|_| Err("panic while creating client".to_string()));

    match outcome {
        Ok(handle) => handle,
        Err(message) => {
            unsafe { set_error(out_err, message) };
            ptr::null_mut()
        }
    }
}

/// Release a handle returned by [`pubmed_client_new`]. Null is a no-op.
///
/// # Safety
///
/// `handle` must come from [`pubmed_client_new`] and must not be used or freed
/// again afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_client_free(handle: *mut PubmedClient) {
    if handle.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(handle) });
}

/// Release a string returned by any of the call functions or written to their
/// `out_err`. Null is a no-op.
///
/// # Safety
///
/// `value` must be a pointer produced by this library and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    drop(unsafe { CString::from_raw(value) });
}

// ------------------------------------------------------------------------------------------------
// PubMed operations
// ------------------------------------------------------------------------------------------------

/// Search PubMed, returning a JSON array of PMIDs.
///
/// # Safety
///
/// See the module-level boundary conventions: `handle` must be live, `query` a
/// valid NUL-terminated string, `out_err` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_search_articles(
    handle: *const PubmedClient,
    query: *const c_char,
    limit: usize,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?.inner.clone();
        let query = unsafe { borrow_str(query, "query") }?;

        let pmids = runtime()
            .block_on(client.pubmed.search_articles(query, limit, None))
            .map_err(|e| e.to_string())?;
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
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?.inner.clone();
        let pmid = unsafe { borrow_str(pmid, "pmid") }?;

        let article = runtime()
            .block_on(client.pubmed.fetch_article(pmid))
            .map_err(|e| e.to_string())?;
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
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?.inner.clone();
        let raw = unsafe { borrow_str(pmids_json, "pmids_json") }?;
        let pmids: Vec<String> =
            serde_json::from_str(raw).map_err(|e| format!("invalid pmids array: {e}"))?;
        let pmid_refs: Vec<&str> = pmids.iter().map(String::as_str).collect();

        let articles = runtime()
            .block_on(client.pubmed.fetch_articles(&pmid_refs))
            .map_err(|e| e.to_string())?;
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
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?.inner.clone();
        let query = unsafe { borrow_str(query, "query") }?;

        let articles = runtime()
            .block_on(client.pubmed.search_and_fetch(query, limit, None))
            .map_err(|e| e.to_string())?;
        to_json(&articles)
    })
}

// ------------------------------------------------------------------------------------------------
// PMC operations
// ------------------------------------------------------------------------------------------------

/// Flattened projection of a [`PmcArticle`] for the Go bindings.
///
/// The JATS domain model is deeply nested (front / body / back); this borrows
/// the fields the Go `PMCArticle` struct exposes through the article's
/// accessor methods, so Go never has to mirror the full DTD tree.
#[derive(Serialize)]
struct PmcArticleDto<'a> {
    pmcid: String,
    pmid: Option<String>,
    title: Option<&'a str>,
    doi: Option<&'a str>,
    journal: &'a JournalMeta,
    volume: Option<&'a str>,
    issue: Option<&'a str>,
    abstract_text: Option<&'a str>,
    keywords: &'a [String],
    authors: &'a [Author],
    sections: &'a [Section],
    references: &'a [Reference],
    figures: Vec<&'a Figure>,
    figure_count: usize,
    table_count: usize,
}

impl<'a> From<&'a PmcArticle> for PmcArticleDto<'a> {
    fn from(article: &'a PmcArticle) -> Self {
        Self {
            pmcid: article.pmcid().to_string(),
            pmid: article.pmid().map(|pmid| pmid.to_string()),
            title: article.title(),
            doi: article.doi(),
            journal: article.journal(),
            volume: article.volume(),
            issue: article.issue(),
            abstract_text: article.abstract_text(),
            keywords: article.keywords(),
            authors: article.authors(),
            sections: article.sections(),
            references: article.references(),
            figures: article.all_figures(),
            figure_count: article.figure_count(),
            table_count: article.table_count(),
        }
    }
}

/// Fetch PMC full text for a PMCID, returning a JSON object (see
/// `PmcArticleDto`).
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_fetch_full_text(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?.inner.clone();
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;

        let article = runtime()
            .block_on(client.pmc.fetch_full_text(pmcid))
            .map_err(|e| e.to_string())?;
        to_json(&PmcArticleDto::from(&article))
    })
}

/// Fetch a PMC article and render it to Markdown. Returns the Markdown itself,
/// not JSON.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_fetch_markdown(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?.inner.clone();
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;

        let article = runtime()
            .block_on(client.pmc.fetch_full_text(pmcid))
            .map_err(|e| e.to_string())?;
        Ok(PmcMarkdownConverter::new().convert(&article))
    })
}

/// Check whether a PMID has PMC full text available. Returns a JSON string
/// holding the PMCID, or JSON `null` when unavailable.
///
/// # Safety
///
/// See [`pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_check_availability(
    handle: *const PubmedClient,
    pmid: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?.inner.clone();
        let pmid = unsafe { borrow_str(pmid, "pmid") }?;

        let pmcid = runtime()
            .block_on(client.pmc.check_pmc_availability(pmid))
            .map_err(|e| e.to_string())?;
        to_json(&pmcid)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a client with default config and free it — exercises the null
    /// `config_json` path and the handle lifecycle.
    #[test]
    fn client_new_accepts_null_config() {
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(ptr::null(), &mut err) };
        assert!(!handle.is_null());
        assert!(err.is_null());
        unsafe { pubmed_client_free(handle) };
    }

    #[test]
    fn client_new_reports_invalid_config() {
        let config = c"{ not json }";
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(config.as_ptr(), &mut err) };
        assert!(handle.is_null());
        assert!(!err.is_null());

        let message = unsafe { CStr::from_ptr(err) }.to_string_lossy().to_string();
        assert!(message.contains("invalid client config"), "{message}");
        unsafe { pubmed_string_free(err) };
    }

    #[test]
    fn client_new_rejects_unknown_config_keys() {
        let config = c"{\"nope\": 1}";
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(config.as_ptr(), &mut err) };
        assert!(handle.is_null());
        unsafe { pubmed_string_free(err) };
    }

    #[test]
    fn client_new_applies_config() {
        let config = c"{\"api_key\":\"k\",\"email\":\"a@b.c\",\"rate_limit\":9.0,\"cache\":true}";
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(config.as_ptr(), &mut err) };
        assert!(!handle.is_null(), "unexpected error creating client");
        unsafe { pubmed_client_free(handle) };
    }

    /// A null handle must surface as an error rather than a segfault.
    #[test]
    fn calls_reject_null_handle() {
        let mut err: *mut c_char = ptr::null_mut();
        let result =
            unsafe { pubmed_search_articles(ptr::null(), c"cancer".as_ptr(), 1, &mut err) };
        assert!(result.is_null());
        assert!(!err.is_null());
        unsafe { pubmed_string_free(err) };
    }

    /// A null string argument must surface as an error, not a crash.
    #[test]
    fn calls_reject_null_arguments() {
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(ptr::null(), &mut err) };
        assert!(!handle.is_null());

        let result = unsafe { pubmed_search_articles(handle, ptr::null(), 1, &mut err) };
        assert!(result.is_null());
        assert!(!err.is_null());

        unsafe { pubmed_string_free(err) };
        unsafe { pubmed_client_free(handle) };
    }

    /// Malformed JSON in `pmids_json` is rejected before any network call.
    #[test]
    fn fetch_articles_rejects_invalid_json() {
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(ptr::null(), &mut err) };
        assert!(!handle.is_null());

        let result = unsafe { pubmed_fetch_articles(handle, c"[1, 2]".as_ptr(), &mut err) };
        assert!(result.is_null());
        assert!(!err.is_null());

        let message = unsafe { CStr::from_ptr(err) }.to_string_lossy().to_string();
        assert!(message.contains("invalid pmids array"), "{message}");

        unsafe { pubmed_string_free(err) };
        unsafe { pubmed_client_free(handle) };
    }

    #[test]
    fn version_is_the_crate_version() {
        let version = unsafe { CStr::from_ptr(pubmed_client_version()) };
        assert_eq!(version.to_string_lossy(), env!("CARGO_PKG_VERSION"));
    }

    /// Freeing null pointers must be a no-op, since Go's deferred cleanup can
    /// run after a failed call.
    #[test]
    fn free_tolerates_null() {
        unsafe { pubmed_client_free(ptr::null_mut()) };
        unsafe { pubmed_string_free(ptr::null_mut()) };
    }
}
