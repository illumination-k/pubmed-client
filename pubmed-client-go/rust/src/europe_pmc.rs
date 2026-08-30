//! Europe PMC operations: cross-source search, JATS full text, reference and
//! citation graphs, external database links, and supplementary downloads.
//!
//! Europe PMC complements the NCBI E-utilities: it covers preprints (`PPR`),
//! patents (`PAT`), Agricola (`AGR`) and Chinese Biological Abstracts (`CBA`)
//! as well as PubMed (`MED`) and PMC, and needs no API key.

use std::ffi::c_char;
use std::path::Path;

use serde::Deserialize;

use pubmed_client::{EuropePmcId, EuropePmcSearchOptions, ResultType};

use crate::cancel::{PubmedCancel, block_on};
use crate::client::{PubmedClient, borrow_client};
use crate::dto::{
    EuropePmcCitationDto, EuropePmcDatabaseLinkDto, EuropePmcReferenceDto, EuropePmcResultDto,
    EuropePmcSearchPageDto,
};
use crate::error::{ShimError, ShimResult};
use crate::ffi::{borrow_opt_str, borrow_str, guard, parse_json_arg, to_json};

/// Search options decoded from the JSON blob Go passes to the search calls.
///
/// Every field is optional; an empty object (or a null pointer) reproduces
/// `EuropePmcSearchOptions::default()`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SearchOptionsDto {
    result_type: Option<String>,
    page_size: Option<u32>,
    cursor_mark: Option<String>,
    sort: Option<String>,
}

impl SearchOptionsDto {
    /// Build the search options these settings describe.
    fn into_options(self) -> ShimResult<EuropePmcSearchOptions> {
        let defaults = EuropePmcSearchOptions::default();
        Ok(EuropePmcSearchOptions {
            result_type: match &self.result_type {
                Some(value) => parse_result_type(value)?,
                None => defaults.result_type,
            },
            // Europe PMC rejects a page size outside this range outright.
            page_size: self.page_size.unwrap_or(defaults.page_size).clamp(1, 1000),
            cursor_mark: self.cursor_mark.unwrap_or(defaults.cursor_mark),
            sort: self.sort,
        })
    }
}

/// Parse a `resultType` name.
fn parse_result_type(value: &str) -> ShimResult<ResultType> {
    match value.trim().to_lowercase().as_str() {
        "idlist" | "id_list" => Ok(ResultType::IdList),
        "lite" => Ok(ResultType::Lite),
        "core" => Ok(ResultType::Core),
        other => Err(ShimError::invalid_argument(format!(
            "unknown result type: '{other}'. Supported types: idlist, lite, core"
        ))),
    }
}

/// Resolve the `(source, id)` pair a call addresses.
///
/// Europe PMC identifies every record by a source database plus an id. Three
/// spellings are accepted so callers rarely need to pass both:
///
/// * a fully-qualified `"SOURCE/ID"` string (e.g. `"PPR/PPR123456"`), which
///   wins over any separate `source`;
/// * an explicit `source` plus a bare id;
/// * a bare id alone — a `PMC`-prefixed id implies the `PMC` source, anything
///   else is treated as a PubMed (`MED`) record.
fn resolve_id(id: &str, source: Option<&str>) -> ShimResult<EuropePmcId> {
    EuropePmcId::resolve(id, source).map_err(|e| ShimError::invalid_argument(e.to_string()))
}

/// Search Europe PMC across pages until `limit` records are collected. Returns
/// a JSON array of Europe PMC records.
///
/// `options_json` may be null for the defaults (see `SearchOptionsDto`).
///
/// # Safety
///
/// See the boundary conventions on [`crate::pubmed::pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_search(
    handle: *const PubmedClient,
    query: *const c_char,
    limit: usize,
    options_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let query = unsafe { borrow_str(query, "query") }?;
        let options: SearchOptionsDto = if options_json.is_null() {
            SearchOptionsDto::default()
        } else {
            unsafe { parse_json_arg(options_json, "options_json") }?
        };
        let options = options.into_options()?;

        let results =
            unsafe { block_on(cancel, client.europe_pmc.search_all(query, limit, &options)) }?;
        let dtos: Vec<EuropePmcResultDto<'_>> = results.iter().map(Into::into).collect();
        to_json(&dtos)
    })
}

/// Fetch one page of Europe PMC search results. Returns a JSON object carrying
/// the total hit count, the next cursor, and the page's records.
///
/// # Safety
///
/// See [`europe_pmc_search`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_search_page(
    handle: *const PubmedClient,
    query: *const c_char,
    options_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let query = unsafe { borrow_str(query, "query") }?;
        let options: SearchOptionsDto = if options_json.is_null() {
            SearchOptionsDto::default()
        } else {
            unsafe { parse_json_arg(options_json, "options_json") }?
        };
        let options = options.into_options()?;

        let page = unsafe { block_on(cancel, client.europe_pmc.search_page(query, &options)) }?;
        to_json(&EuropePmcSearchPageDto::from(&page))
    })
}

/// Fetch and parse the full text of a Europe PMC record. Returns a JSON
/// article object (see [`crate::dto::PmcArticleDto`]).
///
/// Parsing requires a PMC id, so this supports PMC-sourced records only; use
/// [`europe_pmc_fetch_xml`] for other sources.
///
/// # Safety
///
/// See [`europe_pmc_search`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_fetch_full_text(
    handle: *const PubmedClient,
    id: *const c_char,
    source: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let id = unsafe { borrow_str(id, "id") }?;
        let source = unsafe { borrow_opt_str(source, "source") }?;
        let epmc_id = resolve_id(id, source)?;

        let article = unsafe { block_on(cancel, client.europe_pmc.fetch_full_text(&epmc_id)) }?;
        to_json(&crate::dto::PmcArticleDto::from(&article))
    })
}

/// Fetch the raw JATS XML for a Europe PMC record. Returns the XML, not JSON.
///
/// # Safety
///
/// See [`europe_pmc_search`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_fetch_xml(
    handle: *const PubmedClient,
    id: *const c_char,
    source: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let id = unsafe { borrow_str(id, "id") }?;
        let source = unsafe { borrow_opt_str(source, "source") }?;
        let epmc_id = resolve_id(id, source)?;

        unsafe { block_on(cancel, client.europe_pmc.fetch_full_text_xml(&epmc_id)) }
    })
}

/// Fetch every work cited by a Europe PMC record. Returns a JSON array.
///
/// # Safety
///
/// See [`europe_pmc_search`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_get_references(
    handle: *const PubmedClient,
    id: *const c_char,
    source: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let id = unsafe { borrow_str(id, "id") }?;
        let source = unsafe { borrow_opt_str(source, "source") }?;
        let epmc_id = resolve_id(id, source)?;

        let references = unsafe { block_on(cancel, client.europe_pmc.get_references(&epmc_id)) }?;
        let dtos: Vec<EuropePmcReferenceDto<'_>> = references.iter().map(Into::into).collect();
        to_json(&dtos)
    })
}

/// Fetch every article citing a Europe PMC record. Returns a JSON array.
///
/// # Safety
///
/// See [`europe_pmc_search`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_get_citations(
    handle: *const PubmedClient,
    id: *const c_char,
    source: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let id = unsafe { borrow_str(id, "id") }?;
        let source = unsafe { borrow_opt_str(source, "source") }?;
        let epmc_id = resolve_id(id, source)?;

        let citations = unsafe { block_on(cancel, client.europe_pmc.get_citations(&epmc_id)) }?;
        let dtos: Vec<EuropePmcCitationDto<'_>> = citations.iter().map(Into::into).collect();
        to_json(&dtos)
    })
}

/// Fetch every external database cross-reference for a Europe PMC record.
/// Returns a JSON array.
///
/// # Safety
///
/// See [`europe_pmc_search`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_get_database_links(
    handle: *const PubmedClient,
    id: *const c_char,
    source: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let id = unsafe { borrow_str(id, "id") }?;
        let source = unsafe { borrow_opt_str(source, "source") }?;
        let epmc_id = resolve_id(id, source)?;

        let links = unsafe { block_on(cancel, client.europe_pmc.get_database_links(&epmc_id)) }?;
        let dtos: Vec<EuropePmcDatabaseLinkDto<'_>> = links.iter().map(Into::into).collect();
        to_json(&dtos)
    })
}

/// Download a Europe PMC record's supplementary-files ZIP archive to
/// `output_path`. Returns the written path as a JSON string.
///
/// # Safety
///
/// See [`europe_pmc_search`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn europe_pmc_download_supplementary_files(
    handle: *const PubmedClient,
    id: *const c_char,
    source: *const c_char,
    output_path: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let id = unsafe { borrow_str(id, "id") }?;
        let source = unsafe { borrow_opt_str(source, "source") }?;
        let output_path = unsafe { borrow_str(output_path, "output_path") }?;
        let epmc_id = resolve_id(id, source)?;

        let written = unsafe {
            block_on(
                cancel,
                client
                    .europe_pmc
                    .download_supplementary_files(&epmc_id, Path::new(output_path)),
            )
        }?;
        to_json(&written.to_string_lossy())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubmed_client::EuropePmcSource;

    #[test]
    fn bare_pmc_id_defaults_to_pmc_source() {
        assert_eq!(
            resolve_id("PMC3258128", None).expect("valid").to_string(),
            "PMC/PMC3258128"
        );
    }

    #[test]
    fn bare_numeric_id_defaults_to_med_source() {
        assert_eq!(
            resolve_id("33515491", None).expect("valid").to_string(),
            "MED/33515491"
        );
    }

    #[test]
    fn explicit_pmc_source_normalizes_a_bare_number() {
        assert_eq!(
            resolve_id("3258128", Some("pmc"))
                .expect("valid")
                .to_string(),
            "PMC/PMC3258128"
        );
    }

    #[test]
    fn qualified_id_wins_over_source_argument() {
        let id = resolve_id("PPR/PPR123456", Some("MED")).expect("valid");
        assert_eq!(id.source, EuropePmcSource::Ppr);
        assert_eq!(id.id, "PPR123456");
    }

    #[test]
    fn unknown_source_is_passed_through() {
        assert_eq!(
            resolve_id("42", Some("xyz")).expect("valid").to_string(),
            "XYZ/42"
        );
    }

    #[test]
    fn invalid_ids_are_rejected() {
        for (id, source) in [("  ", None), ("MED/", None), ("nope", Some("PMC"))] {
            let error = resolve_id(id, source).expect_err("should be rejected");
            assert_eq!(error.kind, crate::error::ErrorKind::InvalidArgument);
        }
    }

    #[test]
    fn search_options_default_to_the_library_defaults() {
        let options = SearchOptionsDto::default().into_options().expect("valid");
        let defaults = EuropePmcSearchOptions::default();
        assert_eq!(options.page_size, defaults.page_size);
        assert_eq!(options.cursor_mark, defaults.cursor_mark);
        assert!(options.sort.is_none());
    }

    #[test]
    fn search_options_clamp_the_page_size() {
        let options = SearchOptionsDto {
            page_size: Some(9999),
            ..Default::default()
        }
        .into_options()
        .expect("valid");
        assert_eq!(options.page_size, 1000);
    }

    #[test]
    fn search_options_reject_an_unknown_result_type() {
        let error = SearchOptionsDto {
            result_type: Some("verbose".to_string()),
            ..Default::default()
        }
        .into_options()
        .expect_err("should be rejected");
        assert_eq!(error.kind, crate::error::ErrorKind::InvalidArgument);
    }
}
