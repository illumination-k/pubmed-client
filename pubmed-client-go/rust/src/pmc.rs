//! PMC (PubMed Central) operations: full text, raw XML, Markdown rendering,
//! availability checks, and Open Access file downloads.

use std::collections::HashMap;
use std::ffi::c_char;
use std::path::Path;

use serde::Deserialize;

use pubmed_client::{HeadingStyle, MarkdownConfig, PmcMarkdownConverter, ReferenceStyle};

use crate::cancel::{PubmedCancel, block_on, block_on_infallible};
use crate::client::{PubmedClient, borrow_client};
use crate::dto::PmcArticleDto;
use crate::error::{ShimError, ShimResult};
use crate::ffi::{borrow_str, guard, parse_json_arg, to_json};

/// Markdown rendering options, decoded from the JSON blob Go passes to
/// [`pmc_fetch_markdown`].
///
/// Every field is optional and defaults to `MarkdownConfig::default()`, so an
/// empty object (or a null pointer) reproduces `PmcMarkdownConverter::new()`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct MarkdownOptionsDto {
    include_metadata: Option<bool>,
    yaml_frontmatter: Option<bool>,
    include_orcid_links: Option<bool>,
    include_identifier_links: Option<bool>,
    include_figure_captions: Option<bool>,
    include_toc: Option<bool>,
    heading_style: Option<String>,
    reference_style: Option<String>,
    max_heading_level: Option<u8>,
    /// Figure id to local file path. Supplying any entry also switches on
    /// local figure rendering, since a path map is only useful when the images
    /// are actually linked.
    figure_paths: Option<HashMap<String, String>>,
}

impl MarkdownOptionsDto {
    /// Build the converter these options describe.
    fn into_converter(self) -> ShimResult<(PmcMarkdownConverter, Option<HashMap<String, String>>)> {
        let mut config = MarkdownConfig::default();

        if let Some(include) = self.include_metadata {
            config.metadata.include_metadata = include;
        }
        if let Some(yaml) = self.yaml_frontmatter {
            config.metadata.use_yaml_frontmatter = yaml;
        }
        if let Some(include) = self.include_orcid_links {
            config.metadata.include_orcid_links = include;
        }
        if let Some(include) = self.include_identifier_links {
            config.metadata.include_identifier_links = include;
        }
        if let Some(include) = self.include_figure_captions {
            config.figures.include_figure_captions = include;
        }
        if let Some(include) = self.include_toc {
            config.include_toc = include;
        }
        if let Some(style) = &self.heading_style {
            config.heading_style = parse_heading_style(style)?;
        }
        if let Some(style) = &self.reference_style {
            config.reference_style = parse_reference_style(style)?;
        }
        if let Some(level) = self.max_heading_level {
            if !(1..=6).contains(&level) {
                return Err(ShimError::invalid_argument(format!(
                    "max_heading_level must be between 1 and 6, got {level}"
                )));
            }
            config.max_heading_level = level;
        }

        let figure_paths = self.figure_paths.filter(|paths| !paths.is_empty());
        if figure_paths.is_some() {
            config.figures.include_local_figures = true;
        }

        Ok((PmcMarkdownConverter::with_config(config), figure_paths))
    }
}

/// Parse a heading style name.
fn parse_heading_style(value: &str) -> ShimResult<HeadingStyle> {
    match value.trim().to_lowercase().as_str() {
        "atx" => Ok(HeadingStyle::ATX),
        "setext" => Ok(HeadingStyle::Setext),
        other => Err(ShimError::invalid_argument(format!(
            "unknown heading style: '{other}'. Supported styles: atx, setext"
        ))),
    }
}

/// Parse a reference style name.
fn parse_reference_style(value: &str) -> ShimResult<ReferenceStyle> {
    match value.trim().to_lowercase().as_str() {
        "numbered" => Ok(ReferenceStyle::Numbered),
        "author-year" | "author_year" => Ok(ReferenceStyle::AuthorYear),
        "full-citation" | "full_citation" => Ok(ReferenceStyle::FullCitation),
        other => Err(ShimError::invalid_argument(format!(
            "unknown reference style: '{other}'. \
             Supported styles: numbered, author-year, full-citation"
        ))),
    }
}

/// Fetch PMC full text for a PMCID, returning a JSON object (see
/// [`PmcArticleDto`]).
///
/// # Safety
///
/// See the boundary conventions on [`crate::pubmed::pubmed_search_articles`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_fetch_full_text(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;

        let article = unsafe { block_on(cancel, client.pmc.fetch_full_text(pmcid)) }?;
        to_json(&PmcArticleDto::from(&article))
    })
}

/// Fetch the raw JATS XML for a PMCID. Returns the XML itself, not JSON.
///
/// # Safety
///
/// See [`pmc_fetch_full_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_fetch_xml(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;

        unsafe { block_on(cancel, client.pmc.fetch_xml(pmcid)) }
    })
}

/// Fetch a PMC article and render it to Markdown. Returns the Markdown itself,
/// not JSON.
///
/// `options_json` may be null for the default rendering (see
/// `MarkdownOptionsDto`).
///
/// # Safety
///
/// See [`pmc_fetch_full_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_fetch_markdown(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    options_json: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;
        let options: MarkdownOptionsDto = if options_json.is_null() {
            MarkdownOptionsDto::default()
        } else {
            unsafe { parse_json_arg(options_json, "options_json") }?
        };
        let (converter, figure_paths) = options.into_converter()?;

        let article = unsafe { block_on(cancel, client.pmc.fetch_full_text(pmcid)) }?;
        Ok(converter.convert_with_figures(&article, figure_paths.as_ref()))
    })
}

/// Check whether a PMID has PMC full text available. Returns a JSON string
/// holding the PMCID, or JSON `null` when unavailable.
///
/// # Safety
///
/// See [`pmc_fetch_full_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_check_availability(
    handle: *const PubmedClient,
    pmid: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmid = unsafe { borrow_str(pmid, "pmid") }?;

        let pmcid = unsafe { block_on(cancel, client.pmc.check_pmc_availability(pmid)) }?;
        to_json(&pmcid)
    })
}

/// Report whether a PMCID is in the PMC Open Access subset, along with its
/// licence and retraction status. Returns a JSON `OaSubsetInfo` object.
///
/// # Safety
///
/// See [`pmc_fetch_full_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_is_oa_subset(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;

        let info = unsafe { block_on(cancel, client.pmc.is_oa_subset(pmcid)) }?;
        to_json(&info)
    })
}

/// Download an Open Access article's files into `output_dir`. Returns a JSON
/// array of the paths written.
///
/// # Safety
///
/// See [`pmc_fetch_full_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_download_files(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    output_dir: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;
        let output_dir = unsafe { borrow_str(output_dir, "output_dir") }?;

        let files = unsafe {
            block_on(
                cancel,
                client.pmc.download_files(pmcid, Path::new(output_dir)),
            )
        }?;
        to_json(&files)
    })
}

/// Download an Open Access article's figures into `output_dir` and pair each
/// with its caption from the XML. Returns a JSON array of `ExtractedFigure`
/// objects.
///
/// # Safety
///
/// See [`pmc_fetch_full_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_extract_figures(
    handle: *const PubmedClient,
    pmcid: *const c_char,
    output_dir: *const c_char,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        let pmcid = unsafe { borrow_str(pmcid, "pmcid") }?;
        let output_dir = unsafe { borrow_str(output_dir, "output_dir") }?;

        let figures = unsafe {
            block_on(
                cancel,
                client
                    .pmc
                    .extract_figures_with_captions(pmcid, Path::new(output_dir)),
            )
        }?;
        to_json(&figures)
    })
}

/// Drop every cached PMC response. Returns JSON `null`.
///
/// # Safety
///
/// See [`pmc_fetch_full_text`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pmc_clear_cache(
    handle: *const PubmedClient,
    cancel: *const PubmedCancel,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let client = unsafe { borrow_client(handle) }?;
        unsafe { block_on_infallible(cancel, client.pmc.clear_cache()) }?;
        Ok("null".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_reproduce_the_default_converter() {
        let (_, figure_paths) = MarkdownOptionsDto::default()
            .into_converter()
            .expect("defaults are valid");
        assert!(figure_paths.is_none());
    }

    #[test]
    fn options_decode_from_an_empty_object() {
        let options: MarkdownOptionsDto = serde_json::from_str("{}").expect("empty is valid");
        options.into_converter().expect("valid");
    }

    #[test]
    fn options_reject_unknown_keys() {
        serde_json::from_str::<MarkdownOptionsDto>(r#"{"include_tock": true}"#)
            .expect_err("a typo must not be silently ignored");
    }

    #[test]
    fn a_figure_path_map_switches_on_local_figures() {
        let options: MarkdownOptionsDto =
            serde_json::from_str(r#"{"figure_paths":{"fig1":"./fig1.jpg"}}"#).expect("valid");
        let (_, figure_paths) = options.into_converter().expect("valid");
        assert_eq!(
            figure_paths.expect("paths were supplied").get("fig1"),
            Some(&"./fig1.jpg".to_string())
        );
    }

    #[test]
    fn an_empty_figure_path_map_is_treated_as_unset() {
        let options: MarkdownOptionsDto =
            serde_json::from_str(r#"{"figure_paths":{}}"#).expect("valid");
        let (_, figure_paths) = options.into_converter().expect("valid");
        assert!(figure_paths.is_none());
    }

    #[test]
    fn heading_and_reference_styles_parse_case_insensitively() {
        assert_eq!(
            parse_heading_style("ATX").expect("valid"),
            HeadingStyle::ATX
        );
        assert_eq!(
            parse_heading_style(" setext ").expect("valid"),
            HeadingStyle::Setext
        );
        assert_eq!(
            parse_reference_style("Author-Year").expect("valid"),
            ReferenceStyle::AuthorYear
        );
        assert_eq!(
            parse_reference_style("full_citation").expect("valid"),
            ReferenceStyle::FullCitation
        );
    }

    #[test]
    fn unknown_styles_are_invalid_arguments() {
        assert!(parse_heading_style("underline").is_err());
        assert!(parse_reference_style("harvard").is_err());
    }

    #[test]
    fn an_out_of_range_heading_level_is_rejected() {
        let options: MarkdownOptionsDto =
            serde_json::from_str(r#"{"max_heading_level":9}"#).expect("valid JSON");
        let Err(error) = options.into_converter() else {
            panic!("9 is not a heading level");
        };
        assert!(
            error.message.contains("between 1 and 6"),
            "{}",
            error.message
        );
    }
}
