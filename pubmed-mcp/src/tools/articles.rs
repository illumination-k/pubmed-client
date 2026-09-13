//! EFetch tool for PubMed MCP server: full article records by PMID.

use pubmed_client::{Author, PubMedArticle, PubMedId};
use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{internal_error, invalid_params};

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

/// Where an article appeared.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct JournalOut {
    /// Full journal name.
    pub title: String,
    /// ISO abbreviation, when the record carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbreviation: Option<String>,
    /// ISSN.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issn: Option<String>,
    /// Publication date as PubMed reports it (often just a year).
    pub pub_date: String,
    /// Volume.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Page range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
}

/// One labelled part of a structured abstract.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AbstractSectionOut {
    /// Section label (e.g. "BACKGROUND", "METHODS").
    pub label: String,
    /// Section text.
    pub text: String,
}

/// An affiliation and the authors who share it.
///
/// PubMed repeats the *whole* affiliation string on every author of a
/// collaboration; grouping keeps the information and drops the duplication.
#[derive(Debug, PartialEq, Serialize, schemars::JsonSchema)]
pub struct AffiliationOut {
    /// The affiliation text, as PubMed records it.
    pub affiliation: String,
    /// Names of the authors carrying this affiliation.
    pub authors: Vec<String>,
}

/// A MeSH qualifier (subheading) attached to a descriptor.
#[derive(Debug, PartialEq, Serialize, schemars::JsonSchema)]
pub struct MeshQualifierOut {
    /// Qualifier name (e.g. "drug therapy").
    pub name: String,
    /// Qualifier unique identifier.
    pub ui: String,
    /// Whether this qualifier is a major topic of the article.
    pub major_topic: bool,
}

/// A MeSH heading assigned to the article by NLM indexers.
#[derive(Debug, PartialEq, Serialize, schemars::JsonSchema)]
pub struct MeshTermOut {
    /// Descriptor name (e.g. "Diabetes Mellitus, Type 2").
    pub descriptor: String,
    /// Descriptor unique identifier (e.g. "D003924").
    pub descriptor_ui: String,
    /// Whether the article is *about* this descriptor, as opposed to merely
    /// mentioning it.
    pub major_topic: bool,
    /// Subheadings narrowing the descriptor.
    pub qualifiers: Vec<MeshQualifierOut>,
}

/// A chemical substance indexed for the article.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SubstanceOut {
    /// Substance name.
    pub name: String,
    /// CAS registry number, when NLM recorded one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_number: Option<String>,
}

/// A complete PubMed record.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ArticleOut {
    /// PubMed ID.
    pub pmid: String,
    /// Article title.
    pub title: String,
    /// PMC ID, when a free full-text version exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmc_id: Option<String>,
    /// DOI, when the record carries one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    /// Journal and issue details.
    pub journal: JournalOut,
    /// Publication language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Author names, in author order.
    pub authors: Vec<String>,
    /// Author count as PubMed reports it; may exceed `authors.len()` for
    /// records that truncate the list.
    pub author_count: u32,
    /// Affiliations grouped by the authors sharing them. Empty unless
    /// `include_affiliations` was set.
    pub affiliations: Vec<AffiliationOut>,
    /// Publication types (e.g. "Journal Article", "Review").
    pub article_types: Vec<String>,
    /// Full abstract text. Omitted when `include_abstract` is false, or when
    /// the abstract is structured (see `structured_abstract`, which carries
    /// the same prose with its labels intact).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    /// Labelled abstract parts, when the publisher supplied a structured
    /// abstract.
    pub structured_abstract: Vec<AbstractSectionOut>,
    /// Author keywords.
    pub keywords: Vec<String>,
    /// MeSH headings. Empty unless `include_mesh` is set (the default).
    pub mesh_terms: Vec<MeshTermOut>,
    /// Chemical substances. Empty unless `include_mesh` is set (the default).
    pub substances: Vec<SubstanceOut>,
}

/// Structured answer of the `fetch_articles` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ArticlesOutput {
    /// Number of PMIDs requested.
    pub requested: usize,
    /// Number of records returned.
    pub count: usize,
    /// Requested PMIDs that EFetch returned no record for. Not an error: a
    /// withdrawn or mistyped-but-well-formed ID looks the same.
    pub not_found: Vec<String>,
    /// The records, in the order EFetch returned them.
    pub articles: Vec<ArticleOut>,
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
) -> Result<Json<ArticlesOutput>, ErrorData> {
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

    let not_found = missing_pmids(&pmid_refs, &articles)
        .into_iter()
        .map(str::to_string)
        .collect();

    let articles: Vec<ArticleOut> = articles
        .iter()
        .map(|article| {
            article_out(
                article,
                include_abstract,
                include_mesh,
                include_affiliations,
            )
        })
        .collect();

    Ok(Json(ArticlesOutput {
        requested: pmid_refs.len(),
        count: articles.len(),
        not_found,
        articles,
    }))
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

fn article_out(
    article: &PubMedArticle,
    include_abstract: bool,
    include_mesh: bool,
    include_affiliations: bool,
) -> ArticleOut {
    // A structured abstract keeps its BACKGROUND/METHODS/RESULTS labels;
    // `abstract_text` holds the same content flattened, so emit one or the
    // other, never both.
    let structured_abstract: Vec<AbstractSectionOut> = match article.structured_abstract.as_deref()
    {
        Some(sections) if include_abstract => sections
            .iter()
            .map(|section| AbstractSectionOut {
                label: section.label.clone(),
                text: section.text.clone(),
            })
            .collect(),
        _ => Vec::new(),
    };
    let abstract_text = if include_abstract && structured_abstract.is_empty() {
        article.abstract_text.clone()
    } else {
        None
    };

    ArticleOut {
        pmid: article.pmid.clone(),
        title: article.title.clone(),
        pmc_id: article.pmc_id.clone(),
        doi: article.doi.clone(),
        journal: JournalOut {
            title: article.journal.clone(),
            abbreviation: article.journal_abbreviation.clone(),
            issn: article.issn.clone(),
            pub_date: article.pub_date.clone(),
            volume: article.volume.clone(),
            issue: article.issue.clone(),
            pages: article.pages.clone(),
        },
        language: article.language.clone(),
        authors: article
            .authors
            .iter()
            .map(|author| author.full_name.clone())
            .collect(),
        author_count: article.author_count,
        affiliations: if include_affiliations {
            grouped_affiliations(&article.authors)
        } else {
            Vec::new()
        },
        article_types: article.article_types.clone(),
        abstract_text,
        structured_abstract,
        keywords: article.keywords.clone().unwrap_or_default(),
        mesh_terms: if include_mesh {
            mesh_terms(article)
        } else {
            Vec::new()
        },
        substances: if include_mesh {
            article
                .chemical_list
                .iter()
                .flatten()
                .map(|chemical| SubstanceOut {
                    name: chemical.name.clone(),
                    registry_number: chemical.registry_number.clone(),
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}

/// Group authors by affiliation, preserving first-seen order.
///
/// PubMed repeats the *whole* affiliation string on every author of a
/// collaboration — a 2 KB institute blob on a 19-author paper is 38 KB of
/// identical text. Emitting each distinct affiliation once, with the authors
/// that share it, keeps the information and drops the duplication.
fn grouped_affiliations(authors: &[Author]) -> Vec<AffiliationOut> {
    let mut grouped: Vec<AffiliationOut> = Vec::new();

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
            match grouped.iter_mut().find(|group| group.affiliation == text) {
                Some(group) => {
                    if !group.authors.contains(&author.full_name) {
                        group.authors.push(author.full_name.clone());
                    }
                }
                None => grouped.push(AffiliationOut {
                    affiliation: text,
                    authors: vec![author.full_name.clone()],
                }),
            }
        }
    }

    grouped
}

/// MeSH headings, flattened across heading groups.
///
/// `major_topic` is carried through rather than dropped: it decides whether a
/// heading is what the paper is *about* or merely mentioned, and a bare list
/// of descriptor names cannot answer indexing questions.
fn mesh_terms(article: &PubMedArticle) -> Vec<MeshTermOut> {
    let Some(ref headings) = article.mesh_headings else {
        return Vec::new();
    };

    headings
        .iter()
        .flat_map(|heading| heading.mesh_terms.iter())
        .map(|term| MeshTermOut {
            descriptor: term.descriptor_name.clone(),
            descriptor_ui: term.descriptor_ui.clone(),
            major_topic: term.major_topic,
            qualifiers: term
                .qualifiers
                .iter()
                .map(|qualifier| MeshQualifierOut {
                    name: qualifier.qualifier_name.clone(),
                    ui: qualifier.qualifier_ui.clone(),
                    major_topic: qualifier.major_topic,
                })
                .collect(),
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

    fn convert(article: &PubMedArticle) -> ArticleOut {
        article_out(article, true, true, false)
    }

    /// The structured answer is what reaches the client, so assert on the
    /// serialized JSON rather than only on the Rust struct.
    fn json(article: &ArticleOut) -> serde_json::Value {
        serde_json::to_value(article).expect("ArticleOut should serialize")
    }

    #[test]
    fn core_metadata_is_carried_through() {
        let out = convert(&PubMedArticle {
            doi: Some("10.1000/xyz".to_string()),
            pmc_id: Some("PMC7092803".to_string()),
            volume: Some("88".to_string()),
            issue: Some("3".to_string()),
            pages: Some("123-130".to_string()),
            ..article("31978945")
        });

        assert_eq!(out.pmid, "31978945");
        assert_eq!(out.title, "A study of things");
        assert_eq!(out.pmc_id.as_deref(), Some("PMC7092803"));
        assert_eq!(out.doi.as_deref(), Some("10.1000/xyz"));
        assert_eq!(out.journal.title, "Journal of Things");
        assert_eq!(out.journal.pub_date, "2020");
        assert_eq!(out.journal.volume.as_deref(), Some("88"));
        assert_eq!(out.journal.issue.as_deref(), Some("3"));
        assert_eq!(out.journal.pages.as_deref(), Some("123-130"));
        assert_eq!(out.authors, vec!["Jane Doe".to_string()]);
        assert_eq!(out.author_count, 1);
    }

    #[test]
    fn absent_optional_fields_are_omitted_from_the_json() {
        let value = json(&convert(&article("31978945")));

        for key in ["pmc_id", "doi", "language", "abstract_text"] {
            assert!(value.get(key).is_none(), "{key} in: {value}");
        }
        for key in ["issn", "abbreviation", "volume", "issue", "pages"] {
            assert!(value["journal"].get(key).is_none(), "journal.{key}");
        }
        // Absent collections stay present as empty arrays: a consumer can
        // iterate them without a null check.
        for key in ["keywords", "mesh_terms", "substances", "affiliations"] {
            assert_eq!(value[key], serde_json::json!([]), "{key}");
        }
    }

    #[test]
    fn full_abstract_is_not_truncated() {
        // The whole point of this tool over search_pubmed's 200-char preview.
        let long = "x".repeat(5_000);
        let out = convert(&PubMedArticle {
            abstract_text: Some(long.clone()),
            ..article("31978945")
        });

        assert_eq!(out.abstract_text.as_deref(), Some(long.as_str()));
    }

    #[test]
    fn structured_abstract_keeps_its_labels_and_is_not_duplicated() {
        let out = convert(&PubMedArticle {
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

        let labels: Vec<&str> = out
            .structured_abstract
            .iter()
            .map(|section| section.label.as_str())
            .collect();
        assert_eq!(labels, vec!["BACKGROUND", "RESULTS"]);
        // The flattened `abstract_text` holds the same prose; emitting both
        // would double the article's largest field.
        assert_eq!(out.abstract_text, None);
    }

    #[test]
    fn opting_out_of_the_abstract_drops_both_shapes() {
        let with_both = PubMedArticle {
            abstract_text: Some("Flat".to_string()),
            structured_abstract: Some(vec![AbstractSection {
                label: "BACKGROUND".to_string(),
                text: "Background text".to_string(),
            }]),
            ..article("31978945")
        };

        let out = article_out(&with_both, false, true, false);
        assert_eq!(out.abstract_text, None);
        assert!(out.structured_abstract.is_empty());
    }

    #[test]
    fn mesh_terms_keep_major_topics_and_qualifiers() {
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
                MeshTermOut {
                    descriptor: "Diabetes Mellitus, Type 2".to_string(),
                    descriptor_ui: "D003924".to_string(),
                    major_topic: true,
                    qualifiers: vec![MeshQualifierOut {
                        name: "drug therapy".to_string(),
                        ui: "Q000188".to_string(),
                        major_topic: true,
                    }],
                },
                MeshTermOut {
                    descriptor: "Humans".to_string(),
                    descriptor_ui: "D006801".to_string(),
                    major_topic: false,
                    qualifiers: Vec::new(),
                },
            ]
        );

        let out = convert(&with_mesh);
        assert_eq!(out.substances.len(), 1);
        assert_eq!(out.substances[0].name, "Metformin");

        // ...and nothing MeSH-related when the caller opts out.
        let without = article_out(&with_mesh, true, false, false);
        assert!(without.mesh_terms.is_empty());
        assert!(without.substances.is_empty());
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

        assert!(
            article_out(&with_affiliation, true, true, false)
                .affiliations
                .is_empty()
        );

        let opted_in = article_out(&with_affiliation, true, true, true);
        assert_eq!(
            opted_in.affiliations,
            vec![AffiliationOut {
                affiliation: "Some University, Springfield".to_string(),
                authors: vec!["Jane Doe".to_string()],
            }]
        );
    }

    #[test]
    fn a_shared_affiliation_is_emitted_once_for_all_its_authors() {
        // PubMed repeats a collaboration's whole affiliation blob on every
        // author; repeating it per author is the difference between 2 KB and
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
                AffiliationOut {
                    affiliation: "Shared Institute, Beijing, China".to_string(),
                    authors: vec!["Na Zhu".to_string(), "Wenjie Tan".to_string()],
                },
                AffiliationOut {
                    affiliation: "Other University".to_string(),
                    authors: vec!["Wei Shi".to_string()],
                },
            ]
        );
    }

    #[test]
    fn authors_without_affiliations_produce_no_entries() {
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
