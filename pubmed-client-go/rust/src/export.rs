//! Citation export (BibTeX, RIS, CSL-JSON, NBIB).
//!
//! Unlike every other call, this one moves data *into* Rust: Go sends articles
//! back so the formatter — with its escaping rules and BibTeX key generation —
//! stays the single implementation.
//!
//! Go marshals its `Article` with `omitempty`, so unset fields arrive missing
//! rather than null, which `PubMedArticle` (no `#[serde(default)]`) would
//! reject. Each incoming object is therefore merged over a template before
//! deserializing — see the `merge` function below. The template is written out
//! field by field on purpose: a new field on `PubMedArticle` or on any type it
//! nests breaks this file at compile time instead of silently failing to
//! deserialize at runtime.
//!
//! The call is pure: no client handle, no runtime, no network.

use std::ffi::c_char;

use serde_json::{Map, Value};

use pubmed_client::pubmed::models::{
    ChemicalConcept, MeshHeading, MeshQualifier, MeshTerm, SupplementalConcept,
};
use pubmed_client::{AbstractSection, Affiliation, Author, ExportFormat, PubMedArticle};

use crate::error::{ShimError, ShimResult};
use crate::ffi::{borrow_str, guard, parse_json_arg};

// ------------------------------------------------------------------------------------------------
// Templates
// ------------------------------------------------------------------------------------------------
//
// A template supplies two things at once: the value to use when a key is
// missing, and — through the single element of each array — the prototype to
// merge every incoming array element against. A missing array always defaults
// to empty, so the prototypes never leak into the result.

fn affiliation_template() -> Affiliation {
    Affiliation {
        id: None,
        institution: None,
        department: None,
        address: None,
        country: None,
    }
}

fn author_template() -> Author {
    Author {
        surname: None,
        given_names: None,
        initials: None,
        suffix: None,
        full_name: String::new(),
        affiliations: vec![affiliation_template()],
        orcid: None,
        email: None,
        is_corresponding: false,
        roles: vec![String::new()],
        collab_name: None,
    }
}

fn mesh_heading_template() -> MeshHeading {
    MeshHeading {
        mesh_terms: vec![MeshTerm {
            descriptor_name: String::new(),
            descriptor_ui: String::new(),
            major_topic: false,
            qualifiers: vec![MeshQualifier {
                qualifier_name: String::new(),
                qualifier_ui: String::new(),
                major_topic: false,
            }],
        }],
        supplemental_concepts: vec![SupplementalConcept {
            name: String::new(),
            ui: String::new(),
            concept_type: None,
        }],
    }
}

/// An empty article carrying one prototype element per collection.
fn article_template() -> PubMedArticle {
    PubMedArticle {
        pmid: String::new(),
        title: String::new(),
        authors: vec![author_template()],
        author_count: 0,
        journal: String::new(),
        pub_date: String::new(),
        doi: None,
        pmc_id: None,
        abstract_text: None,
        structured_abstract: Some(vec![AbstractSection {
            label: String::new(),
            text: String::new(),
        }]),
        article_types: vec![String::new()],
        mesh_headings: Some(vec![mesh_heading_template()]),
        keywords: Some(vec![String::new()]),
        chemical_list: Some(vec![ChemicalConcept {
            name: String::new(),
            registry_number: None,
            ui: None,
        }]),
        volume: None,
        issue: None,
        pages: None,
        language: None,
        journal_abbreviation: None,
        issn: None,
    }
}

// ------------------------------------------------------------------------------------------------
// Merge
// ------------------------------------------------------------------------------------------------

/// The value to use for a key the caller omitted.
///
/// Arrays in a template exist only to carry prototypes, so an omitted array is
/// empty rather than a copy of the prototype list.
fn default_for(template: &Value) -> Value {
    match template {
        Value::Array(_) => Value::Array(Vec::new()),
        other => other.clone(),
    }
}

/// Fill in whatever `incoming` left out, using `template`.
///
/// Objects merge key by key; arrays merge each element against the template's
/// single prototype element; anything else is taken from `incoming`. Keys the
/// template does not mention pass through untouched, so the Go structs may
/// carry fields this crate has never heard of.
fn merge(template: &Value, incoming: Value) -> Value {
    match (template, incoming) {
        (Value::Object(template), Value::Object(mut incoming)) => {
            let mut merged = Map::with_capacity(template.len() + incoming.len());
            for (key, template_value) in template {
                let value = match incoming.remove(key) {
                    Some(incoming_value) => merge(template_value, incoming_value),
                    None => default_for(template_value),
                };
                merged.insert(key.clone(), value);
            }
            merged.extend(incoming);
            Value::Object(merged)
        }
        (Value::Array(template), Value::Array(incoming)) => {
            let Some(prototype) = template.first() else {
                return Value::Array(incoming);
            };
            Value::Array(
                incoming
                    .into_iter()
                    .map(|element| merge(prototype, element))
                    .collect(),
            )
        }
        // A null where the template has structure means "explicitly unset",
        // which `Option` fields accept as-is.
        (_, incoming) => incoming,
    }
}

/// Merge one incoming article object over the template and deserialize it.
fn article_from_value(value: Value, index: usize) -> ShimResult<PubMedArticle> {
    if !value.is_object() {
        return Err(ShimError::invalid_argument(format!(
            "articles_json[{index}] must be an object"
        )));
    }

    let template = serde_json::to_value(article_template())
        .map_err(|e| ShimError::internal(format!("article template is not serializable: {e}")))?;

    serde_json::from_value(merge(&template, value)).map_err(|e| {
        ShimError::invalid_argument(format!("articles_json[{index}] is not an article: {e}"))
    })
}

/// Decode the article array Go sends.
fn decode_articles(articles: Vec<Value>) -> ShimResult<Vec<PubMedArticle>> {
    articles
        .into_iter()
        .enumerate()
        .map(|(index, value)| article_from_value(value, index))
        .collect()
}

/// Render `articles` in `format`.
fn render(articles: &[PubMedArticle], format: &str) -> ShimResult<String> {
    match format {
        // Batch helpers exist for these three and control the joining, so they
        // are preferred over mapping the per-article trait methods.
        "bibtex" => Ok(pubmed_client::export::articles_to_bibtex(articles)),
        "ris" => Ok(pubmed_client::export::articles_to_ris(articles)),
        "csl-json" => {
            serde_json::to_string_pretty(&pubmed_client::export::articles_to_csl_json(articles))
                .map_err(|e| ShimError::internal(format!("failed to serialize CSL-JSON: {e}")))
        }
        "nbib" => Ok(articles
            .iter()
            .map(ExportFormat::to_nbib)
            .collect::<Vec<_>>()
            .join("\n")),
        other => Err(ShimError::invalid_argument(format!(
            "unknown export format: '{other}'. Supported formats: bibtex, ris, csl-json, nbib"
        ))),
    }
}

/// Export a JSON array of articles as a citation document.
///
/// `format` is one of `bibtex`, `ris`, `csl-json`, `nbib`. Returns the rendered
/// document, not JSON (CSL-JSON is of course JSON, but it is the payload rather
/// than an envelope).
///
/// # Safety
///
/// `articles_json` and `format` must be valid NUL-terminated strings, and
/// `out_err` must be null or point to a writable `*mut c_char`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_export_articles(
    articles_json: *const c_char,
    format: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut c_char {
    guard(out_err, || {
        let values: Vec<Value> = unsafe { parse_json_arg(articles_json, "articles_json") }?;
        let format = unsafe { borrow_str(format, "format") }?;
        // Validate the format before decoding, so a typo is not masked by a
        // complaint about the payload.
        if values.is_empty() {
            return render(&[], format);
        }
        let articles = decode_articles(values)?;
        render(&articles, format)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    /// The minimum Go emits for an article with no optional fields set.
    const MINIMAL: &str = r#"{"pmid":"31978945","title":"A study","author_count":0,
                              "journal":"Nature","pub_date":"2020"}"#;

    fn parse(json: &str) -> Value {
        serde_json::from_str(json).expect("valid JSON")
    }

    #[test]
    fn missing_optional_fields_fall_back_to_the_template() {
        let article =
            article_from_value(parse(MINIMAL), 0).expect("a minimal article is enough to export");

        assert_eq!(article.pmid, "31978945");
        assert_eq!(article.doi, None);
        // The template's prototype elements must not leak into the result.
        assert!(article.authors.is_empty());
        assert!(article.article_types.is_empty());
        assert_eq!(article.keywords, Some(Vec::new()));
    }

    #[test]
    fn present_fields_override_the_template() {
        let article = article_from_value(
            parse(
                r#"{"pmid":"1","title":"T","author_count":1,"journal":"J","pub_date":"2020",
                    "doi":"10.1/x","article_types":["Review"]}"#,
            ),
            0,
        )
        .expect("valid article");

        assert_eq!(article.doi.as_deref(), Some("10.1/x"));
        assert_eq!(article.article_types, vec!["Review".to_string()]);
    }

    /// Nested collections need the same treatment as the top level: Go omits an
    /// author's empty affiliations, which `Vec<Affiliation>` would reject.
    #[test]
    fn nested_objects_are_filled_in_too() {
        let article = article_from_value(
            parse(
                r#"{"pmid":"1","title":"T","author_count":1,"journal":"J","pub_date":"2020",
                    "authors":[{"full_name":"Jane Doe","surname":"Doe","is_corresponding":false}]}"#,
            ),
            0,
        )
        .expect("an author without affiliations is still an author");

        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.authors[0].full_name, "Jane Doe");
        assert!(article.authors[0].affiliations.is_empty());
        assert!(article.authors[0].roles.is_empty());
    }

    #[test]
    fn nested_affiliations_are_filled_in_too() {
        let article = article_from_value(
            parse(
                r#"{"pmid":"1","title":"T","author_count":1,"journal":"J","pub_date":"2020",
                    "authors":[{"full_name":"Jane Doe",
                                "affiliations":[{"institution":"Harvard"}]}]}"#,
            ),
            0,
        )
        .expect("a partial affiliation is still an affiliation");

        let affiliation = &article.authors[0].affiliations[0];
        assert_eq!(affiliation.institution.as_deref(), Some("Harvard"));
        assert_eq!(affiliation.country, None);
    }

    #[test]
    fn mesh_headings_survive_the_merge() {
        let article = article_from_value(
            parse(
                r#"{"pmid":"1","title":"T","author_count":0,"journal":"J","pub_date":"2020",
                    "mesh_headings":[{"mesh_terms":[
                        {"descriptor_name":"Neoplasms","descriptor_ui":"D009369",
                         "major_topic":true}]}]}"#,
            ),
            0,
        )
        .expect("mesh headings without qualifiers are valid");

        let headings = article.mesh_headings.expect("headings were supplied");
        assert_eq!(headings[0].mesh_terms[0].descriptor_name, "Neoplasms");
        assert!(headings[0].mesh_terms[0].qualifiers.is_empty());
        assert!(headings[0].supplemental_concepts.is_empty());
    }

    #[test]
    fn unknown_keys_pass_through_and_are_then_ignored() {
        let article = article_from_value(
            parse(
                r#"{"pmid":"1","title":"T","author_count":0,"journal":"J","pub_date":"2020",
                      "from_the_future":42}"#,
            ),
            0,
        )
        .expect("an unknown key must not fail the merge");
        assert_eq!(article.pmid, "1");
    }

    #[test]
    fn explicit_nulls_are_respected() {
        let article = article_from_value(
            parse(
                r#"{"pmid":"1","title":"T","author_count":0,"journal":"J","pub_date":"2020",
                      "doi":null,"keywords":null}"#,
            ),
            0,
        )
        .expect("explicit nulls are valid for optional fields");
        assert_eq!(article.doi, None);
        assert_eq!(article.keywords, None);
    }

    #[test]
    fn every_format_renders_something() {
        let articles = vec![article_from_value(parse(MINIMAL), 0).expect("valid")];

        assert!(
            render(&articles, "bibtex")
                .expect("bibtex")
                .contains("@article")
        );
        assert!(render(&articles, "ris").expect("ris").contains("TY  -"));
        assert!(render(&articles, "nbib").expect("nbib").contains("PMID-"));

        let csl = render(&articles, "csl-json").expect("csl-json");
        let parsed: Value = serde_json::from_str(&csl).expect("CSL-JSON must be valid JSON");
        assert!(parsed.is_array(), "{csl}");
    }

    #[test]
    fn an_unknown_format_is_an_invalid_argument() {
        let error = render(&[], "markdown").expect_err("not a citation format");
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
        assert!(error.message.contains("bibtex"), "{}", error.message);
    }

    #[test]
    fn a_non_object_element_names_its_index() {
        let error = decode_articles(vec![Value::Null]).expect_err("null is not an article");
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
        assert!(error.message.contains("[0]"), "{}", error.message);
    }

    #[test]
    fn a_wrongly_typed_field_names_its_index() {
        let values: Vec<Value> =
            serde_json::from_str(&format!("[{MINIMAL}, {{\"pmid\": 5}}]")).expect("valid");
        let error = decode_articles(values).expect_err("pmid must be a string");
        assert!(error.message.contains("[1]"), "{}", error.message);
    }

    /// Round-trips a real article through serialization to prove the merge does
    /// not drop anything the formatter needs.
    #[test]
    fn a_full_article_survives_the_round_trip() {
        let mut original = article_template();
        original.pmid = "31978945".to_string();
        original.title = "A study of things".to_string();
        original.journal = "Nature".to_string();
        original.pub_date = "2020".to_string();
        original.doi = Some("10.1038/x".to_string());
        original.volume = Some("578".to_string());
        original.pages = Some("82-93".to_string());

        let encoded = serde_json::to_value(&original).expect("serializable");
        let decoded = article_from_value(encoded, 0).expect("round trip");

        assert_eq!(
            render(std::slice::from_ref(&decoded), "bibtex").expect("bibtex"),
            render(std::slice::from_ref(&original), "bibtex").expect("bibtex")
        );
    }
}
