//! EFetch tool for PubMed MCP server: full article records by PMID.

use pubmed_client::{Author, PubMedArticle, PubMedId};
use rmcp::{handler::server::wrapper::Parameters, model::*, schemars};
use serde::Deserialize;
use std::fmt::Write as _;
use tracing::info;

use super::common::{internal_error, invalid_params, text_result};

/// Upper bound on PMIDs per call.
///
/// A full record is one to two orders of magnitude larger than an ESummary
/// row, so an unbounded list would flood the assistant's context long before
/// it hit NCBI's own 200-per-request batch limit. Matches `search_pubmed`'s
/// result cap.
const MAX_PMIDS: usize = 100;

/// Request parameters for the fetch_articles tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ArticlesRequest {
    #[schemars(
        description = "PubMed IDs to fetch full records for (e.g., ['31978945', '33515491']; max 100)"
    )]
    pub pmids: Vec<String>,

    #[schemars(description = "Include the full abstract text (default: true)")]
    pub include_abstract: Option<bool>,

    #[schemars(description = "Include MeSH headings and chemical substances (default: true)")]
    pub include_mesh: Option<bool>,

    #[schemars(description = "Include author affiliations (default: false; verbose)")]
    pub include_affiliations: Option<bool>,
}

/// Fetch complete PubMed records for known PMIDs using the EFetch API.
///
/// Unlike `search_pubmed` (200-character abstract preview) and
/// `fetch_summaries` (ESummary bibliographic overview), this returns the full
/// record: complete abstract, MeSH headings, keywords, article types, and all
/// identifiers.
pub async fn fetch_articles(
    server: &super::PubMedServer,
    Parameters(params): Parameters<ArticlesRequest>,
) -> Result<CallToolResult, ErrorData> {
    if params.pmids.is_empty() {
        return Err(invalid_params("At least one PMID is required"));
    }
    if params.pmids.len() > MAX_PMIDS {
        return Err(invalid_params(format!(
            "Too many PMIDs: {} requested, at most {MAX_PMIDS} per call. Split the list across several calls.",
            params.pmids.len()
        )));
    }

    // Validate here rather than letting the client reject the batch: a bad
    // PMID is the caller's mistake, and naming the offenders lets the
    // assistant fix its own input instead of retrying blind.
    let invalid: Vec<&str> = params
        .pmids
        .iter()
        .filter(|pmid| PubMedId::parse(pmid).is_err())
        .map(|pmid| pmid.as_str())
        .collect();
    if !invalid.is_empty() {
        return Err(invalid_params(format!(
            "Invalid PMID(s): {}. PMIDs are positive integers, e.g. '31978945'.",
            invalid.join(", ")
        )));
    }

    let include_abstract = params.include_abstract.unwrap_or(true);
    let include_mesh = params.include_mesh.unwrap_or(true);
    let include_affiliations = params.include_affiliations.unwrap_or(false);

    info!(
        pmids_count = params.pmids.len(),
        include_abstract, include_mesh, include_affiliations, "Fetching full articles via EFetch"
    );

    let pmid_refs: Vec<&str> = params.pmids.iter().map(|s| s.as_str()).collect();
    let articles = server
        .client
        .pubmed
        .fetch_articles(&pmid_refs)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch articles: {}", e)))?;

    if articles.is_empty() {
        return text_result("No articles found for the given PMIDs.");
    }

    let mut result = format!(
        "Retrieved {} of {} articles:\n",
        articles.len(),
        pmid_refs.len()
    );

    // PMIDs that came back empty are not an error (a withdrawn or mistyped-but-
    // well-formed ID looks the same), but silently returning fewer records
    // would leave the caller guessing which ones are missing.
    let missing = missing_pmids(&pmid_refs, &articles);
    if !missing.is_empty() {
        let _ = writeln!(result, "Not found: {}", missing.join(", "));
    }
    result.push('\n');

    for (i, article) in articles.iter().enumerate() {
        format_article(
            &mut result,
            i + 1,
            article,
            include_abstract,
            include_mesh,
            include_affiliations,
        );
    }

    text_result(result)
}

/// Requested PMIDs that EFetch returned no record for.
///
/// Compares parsed values rather than the raw strings so that whitespace or a
/// leading zero in the request still matches the canonical PMID in the
/// response.
fn missing_pmids<'a>(requested: &[&'a str], articles: &[PubMedArticle]) -> Vec<&'a str> {
    let returned: Vec<u32> = articles
        .iter()
        .filter_map(|a| PubMedId::parse(&a.pmid).ok())
        .map(|id| id.as_u32())
        .collect();

    requested
        .iter()
        .filter(|pmid| match PubMedId::parse(pmid) {
            Ok(id) => !returned.contains(&id.as_u32()),
            Err(_) => true,
        })
        .copied()
        .collect()
}

fn format_article(
    out: &mut String,
    index: usize,
    article: &PubMedArticle,
    include_abstract: bool,
    include_mesh: bool,
    include_affiliations: bool,
) {
    let _ = writeln!(out, "{}. {}", index, article.title);
    let _ = writeln!(out, "   PMID: {}", article.pmid);
    if let Some(ref pmc_id) = article.pmc_id {
        let _ = writeln!(out, "   PMC ID: {}", pmc_id);
    }
    if let Some(ref doi) = article.doi {
        let _ = writeln!(out, "   DOI: {}", doi);
    }

    // Journal line: "Journal Name (2020) 88(3): 123-130"
    let mut journal = article.journal.clone();
    let _ = write!(journal, " ({})", article.pub_date);
    if let Some(ref volume) = article.volume {
        let _ = write!(journal, " {}", volume);
    }
    if let Some(ref issue) = article.issue {
        let _ = write!(journal, "({})", issue);
    }
    if let Some(ref pages) = article.pages {
        let _ = write!(journal, ": {}", pages);
    }
    let _ = writeln!(out, "   Journal: {}", journal);
    if let Some(ref abbreviation) = article.journal_abbreviation {
        let _ = writeln!(out, "   Journal abbreviation: {}", abbreviation);
    }
    if let Some(ref issn) = article.issn {
        let _ = writeln!(out, "   ISSN: {}", issn);
    }
    if let Some(ref language) = article.language {
        let _ = writeln!(out, "   Language: {}", language);
    }

    if !article.authors.is_empty() {
        let names: Vec<&str> = article
            .authors
            .iter()
            .map(|a| a.full_name.as_str())
            .collect();
        let _ = writeln!(
            out,
            "   Authors ({}): {}",
            article.author_count,
            names.join(", ")
        );

        if include_affiliations {
            let affiliations = grouped_affiliations(&article.authors);
            if !affiliations.is_empty() {
                let _ = writeln!(out, "   Affiliations:");
                for (i, (text, authors)) in affiliations.iter().enumerate() {
                    let _ = writeln!(out, "     [{}] {}", i + 1, text);
                    let _ = writeln!(out, "         {}", authors.join(", "));
                }
            }
        }
    }

    if !article.article_types.is_empty() {
        let _ = writeln!(
            out,
            "   Article types: {}",
            article.article_types.join(", ")
        );
    }

    if include_abstract {
        // A structured abstract keeps its BACKGROUND/METHODS/RESULTS labels;
        // `abstract_text` holds the same content flattened, so print one or
        // the other, never both.
        match article.structured_abstract.as_deref() {
            Some(sections) if !sections.is_empty() => {
                let _ = writeln!(out, "   Abstract:");
                for section in sections {
                    let _ = writeln!(out, "     {}: {}", section.label, section.text);
                }
            }
            _ => {
                if let Some(ref abstract_text) = article.abstract_text {
                    let _ = writeln!(out, "   Abstract: {}", abstract_text);
                }
            }
        }
    }

    if let Some(ref keywords) = article.keywords
        && !keywords.is_empty()
    {
        let _ = writeln!(out, "   Keywords: {}", keywords.join(", "));
    }

    if include_mesh {
        let terms = mesh_terms(article);
        if !terms.is_empty() {
            let _ = writeln!(out, "   MeSH terms: {}", terms.join("; "));
        }

        if let Some(ref chemicals) = article.chemical_list
            && !chemicals.is_empty()
        {
            let names: Vec<&str> = chemicals.iter().map(|c| c.name.as_str()).collect();
            let _ = writeln!(out, "   Substances: {}", names.join(", "));
        }
    }

    out.push('\n');
}

/// Group authors by affiliation, preserving first-seen order.
///
/// PubMed repeats the *whole* affiliation string on every author of a
/// collaboration — a 2 KB institute blob on a 19-author paper is 38 KB of
/// identical text. Printing each distinct affiliation once, with the authors
/// that share it, keeps the information and drops the duplication.
fn grouped_affiliations(authors: &[Author]) -> Vec<(String, Vec<&str>)> {
    let mut grouped: Vec<(String, Vec<&str>)> = Vec::new();

    for author in authors {
        for affiliation in &author.affiliations {
            // Affiliations arrive from PubMed as one free-text blob in
            // `address`; the structured fields are only populated for PMC.
            let parts: Vec<&str> = [
                affiliation.department.as_deref(),
                affiliation.institution.as_deref(),
                affiliation.address.as_deref(),
                affiliation.country.as_deref(),
            ]
            .into_iter()
            .flatten()
            .filter(|part: &&str| !part.trim().is_empty())
            .collect();
            if parts.is_empty() {
                continue;
            }

            let text = parts.join(", ");
            match grouped.iter_mut().find(|(existing, _)| *existing == text) {
                Some((_, sharers)) => {
                    if !sharers.contains(&author.full_name.as_str()) {
                        sharers.push(&author.full_name);
                    }
                }
                None => grouped.push((text, vec![&author.full_name])),
            }
        }
    }

    grouped
}

/// Render MeSH headings as `Descriptor*/qualifier*` strings.
///
/// `*` marks a major topic — the distinction that decides whether a heading is
/// what the paper is *about* or merely mentioned, and the reason a bare list
/// of descriptor names is not enough to answer indexing questions.
fn mesh_terms(article: &PubMedArticle) -> Vec<String> {
    let Some(ref headings) = article.mesh_headings else {
        return Vec::new();
    };

    headings
        .iter()
        .flat_map(|heading| heading.mesh_terms.iter())
        .map(|term| {
            let mut rendered = term.descriptor_name.clone();
            if term.major_topic {
                rendered.push('*');
            }
            for qualifier in &term.qualifiers {
                let _ = write!(rendered, "/{}", qualifier.qualifier_name);
                if qualifier.major_topic {
                    rendered.push('*');
                }
            }
            rendered
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubmed_client::Affiliation;
    use pubmed_client::pubmed::{
        AbstractSection, ChemicalConcept, MeshHeading, MeshQualifier, MeshTerm,
    };

    fn author(full_name: &str) -> Author {
        Author {
            surname: None,
            given_names: None,
            initials: None,
            suffix: None,
            full_name: full_name.to_string(),
            affiliations: Vec::new(),
            orcid: None,
            email: None,
            is_corresponding: false,
            roles: Vec::new(),
            collab_name: None,
        }
    }

    fn article(pmid: &str) -> PubMedArticle {
        PubMedArticle {
            pmid: pmid.to_string(),
            title: "A study of things".to_string(),
            authors: vec![author("Jane Doe")],
            author_count: 1,
            journal: "Journal of Things".to_string(),
            pub_date: "2020".to_string(),
            doi: None,
            pmc_id: None,
            abstract_text: None,
            structured_abstract: None,
            article_types: Vec::new(),
            mesh_headings: None,
            keywords: None,
            chemical_list: None,
            volume: None,
            issue: None,
            pages: None,
            language: None,
            journal_abbreviation: None,
            issn: None,
        }
    }

    fn render(article: &PubMedArticle) -> String {
        let mut out = String::new();
        format_article(&mut out, 1, article, true, true, false);
        out
    }

    #[test]
    fn core_metadata_is_rendered() {
        let rendered = render(&PubMedArticle {
            doi: Some("10.1000/xyz".to_string()),
            pmc_id: Some("PMC7092803".to_string()),
            volume: Some("88".to_string()),
            issue: Some("3".to_string()),
            pages: Some("123-130".to_string()),
            ..article("31978945")
        });

        assert!(rendered.contains("1. A study of things"), "{rendered}");
        assert!(rendered.contains("PMID: 31978945"), "{rendered}");
        assert!(rendered.contains("PMC ID: PMC7092803"), "{rendered}");
        assert!(rendered.contains("DOI: 10.1000/xyz"), "{rendered}");
        assert!(
            rendered.contains("Journal: Journal of Things (2020) 88(3): 123-130"),
            "{rendered}"
        );
        assert!(rendered.contains("Authors (1): Jane Doe"), "{rendered}");
    }

    #[test]
    fn absent_optional_fields_leave_no_empty_labels() {
        let rendered = render(&article("31978945"));

        for label in [
            "PMC ID",
            "DOI",
            "ISSN",
            "Language",
            "Abstract",
            "Keywords",
            "MeSH terms",
        ] {
            assert!(!rendered.contains(label), "{label} in:\n{rendered}");
        }
    }

    #[test]
    fn full_abstract_is_not_truncated() {
        // The whole point of this tool over search_pubmed's 200-char preview.
        let long = "x".repeat(5_000);
        let rendered = render(&PubMedArticle {
            abstract_text: Some(long.clone()),
            ..article("31978945")
        });

        assert!(rendered.contains(&long), "abstract was truncated");
        assert!(!rendered.contains("..."), "{rendered}");
    }

    #[test]
    fn structured_abstract_keeps_its_labels_and_is_not_duplicated() {
        let rendered = render(&PubMedArticle {
            abstract_text: Some("Background text Results text".to_string()),
            structured_abstract: Some(vec![
                AbstractSection {
                    label: "BACKGROUND".to_string(),
                    text: "Background text".to_string(),
                },
                AbstractSection {
                    label: "RESULTS".to_string(),
                    text: "Results text".to_string(),
                },
            ]),
            ..article("31978945")
        });

        assert!(
            rendered.contains("BACKGROUND: Background text"),
            "{rendered}"
        );
        assert!(rendered.contains("RESULTS: Results text"), "{rendered}");
        // The flattened `abstract_text` holds the same prose; printing both
        // would double the article's largest field.
        assert!(
            !rendered.contains("Abstract: Background text Results text"),
            "{rendered}"
        );
    }

    #[test]
    fn mesh_terms_mark_major_topics_and_qualifiers() {
        let with_mesh = PubMedArticle {
            mesh_headings: Some(vec![MeshHeading {
                mesh_terms: vec![
                    MeshTerm {
                        descriptor_name: "Diabetes Mellitus, Type 2".to_string(),
                        descriptor_ui: "D003924".to_string(),
                        major_topic: true,
                        qualifiers: vec![MeshQualifier {
                            qualifier_name: "drug therapy".to_string(),
                            qualifier_ui: "Q000188".to_string(),
                            major_topic: true,
                        }],
                    },
                    MeshTerm {
                        descriptor_name: "Humans".to_string(),
                        descriptor_ui: "D006801".to_string(),
                        major_topic: false,
                        qualifiers: Vec::new(),
                    },
                ],
                supplemental_concepts: Vec::new(),
            }]),
            chemical_list: Some(vec![ChemicalConcept {
                name: "Metformin".to_string(),
                registry_number: Some("9100L32L2N".to_string()),
                ui: None,
            }]),
            ..article("31978945")
        };

        assert_eq!(
            mesh_terms(&with_mesh),
            vec![
                "Diabetes Mellitus, Type 2*/drug therapy*".to_string(),
                "Humans".to_string(),
            ]
        );

        let rendered = render(&with_mesh);
        assert!(rendered.contains("Substances: Metformin"), "{rendered}");

        // ...and nothing MeSH-related when the caller opts out.
        let mut without = String::new();
        format_article(&mut without, 1, &with_mesh, true, false, false);
        assert!(!without.contains("MeSH terms"), "{without}");
        assert!(!without.contains("Substances"), "{without}");
    }

    #[test]
    fn mesh_terms_of_an_unindexed_article_are_empty() {
        assert!(mesh_terms(&article("31978945")).is_empty());
    }

    #[test]
    fn affiliations_are_opt_in() {
        let with_affiliation = PubMedArticle {
            authors: vec![Author {
                affiliations: vec![Affiliation {
                    id: None,
                    institution: None,
                    department: None,
                    address: Some("Some University, Springfield".to_string()),
                    country: None,
                }],
                ..author("Jane Doe")
            }],
            ..article("31978945")
        };

        let mut default_off = String::new();
        format_article(&mut default_off, 1, &with_affiliation, true, true, false);
        assert!(!default_off.contains("Springfield"), "{default_off}");

        let mut opted_in = String::new();
        format_article(&mut opted_in, 1, &with_affiliation, true, true, true);
        assert!(
            opted_in.contains("[1] Some University, Springfield"),
            "{opted_in}"
        );
        assert!(opted_in.contains("Jane Doe"), "{opted_in}");
    }

    #[test]
    fn a_shared_affiliation_is_printed_once_for_all_its_authors() {
        // PubMed repeats a collaboration's whole affiliation blob on every
        // author; printing it per author is the difference between 2 KB and
        // 38 KB on a 19-author paper.
        let shared = Affiliation {
            id: None,
            institution: None,
            department: None,
            address: Some("Shared Institute, Beijing".to_string()),
            country: Some("China".to_string()),
        };
        let authors = vec![
            Author {
                affiliations: vec![shared.clone()],
                ..author("Na Zhu")
            },
            Author {
                affiliations: vec![shared.clone()],
                ..author("Wenjie Tan")
            },
            Author {
                affiliations: vec![Affiliation {
                    address: Some("Other University".to_string()),
                    country: None,
                    ..shared.clone()
                }],
                ..author("Wei Shi")
            },
        ];

        assert_eq!(
            grouped_affiliations(&authors),
            vec![
                (
                    "Shared Institute, Beijing, China".to_string(),
                    vec!["Na Zhu", "Wenjie Tan"]
                ),
                ("Other University".to_string(), vec!["Wei Shi"]),
            ]
        );
    }

    #[test]
    fn authors_without_affiliations_produce_no_section() {
        assert!(grouped_affiliations(&[author("Jane Doe")]).is_empty());

        // ...and neither does an affiliation whose every field is blank.
        let blank = vec![Author {
            affiliations: vec![Affiliation {
                id: Some("aff1".to_string()),
                institution: None,
                department: None,
                address: Some("   ".to_string()),
                country: None,
            }],
            ..author("Jane Doe")
        }];
        assert!(grouped_affiliations(&blank).is_empty());
    }

    #[test]
    fn missing_pmids_are_reported_by_difference() {
        let returned = vec![article("31978945"), article("33515491")];
        assert_eq!(
            missing_pmids(&["31978945", "99999999", "33515491"], &returned),
            vec!["99999999"]
        );
        assert!(missing_pmids(&["31978945", "33515491"], &returned).is_empty());
    }

    #[test]
    fn missing_pmids_tolerates_non_canonical_input() {
        // EFetch echoes the canonical PMID, so a padded or space-wrapped
        // request must not be reported as missing.
        let returned = vec![article("31978945")];
        assert!(missing_pmids(&[" 31978945 ", "031978945"], &returned).is_empty());
    }
}
