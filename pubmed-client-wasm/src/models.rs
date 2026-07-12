use pubmed_client::{pmc::PmcArticle, pubmed::ArticleSummary, pubmed::PubMedArticle};
use serde::{Deserialize, Serialize};

/// JavaScript-friendly markdown conversion options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsMarkdownOptions {
    #[serde(default)]
    pub include_metadata: Option<bool>,
    #[serde(default)]
    pub include_toc: Option<bool>,
    #[serde(default)]
    pub use_yaml_frontmatter: Option<bool>,
    #[serde(default)]
    pub include_orcid_links: Option<bool>,
    #[serde(default)]
    pub include_figure_captions: Option<bool>,
}

/// JavaScript-friendly OA (Open Access) subset availability information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsOaSubsetInfo {
    pub pmcid: String,
    pub is_oa_subset: bool,
    pub citation: Option<String>,
    pub license: Option<String>,
    pub retracted: bool,
    pub download_link: Option<String>,
    pub download_format: Option<String>,
    pub updated: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl From<pubmed_client::OaSubsetInfo> for JsOaSubsetInfo {
    fn from(info: pubmed_client::OaSubsetInfo) -> Self {
        Self {
            pmcid: info.pmcid,
            is_oa_subset: info.is_oa_subset,
            citation: info.citation,
            license: info.license,
            retracted: info.retracted,
            download_link: info.download_link,
            download_format: info.download_format,
            updated: info.updated,
            error_code: info.error_code,
            error_message: info.error_message,
        }
    }
}

/// JavaScript-friendly EPost result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsEPostResult {
    pub webenv: String,
    pub query_key: String,
}

/// JavaScript-friendly article representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsArticle {
    pub pmid: String,
    pub title: String,
    pub authors: Vec<String>,
    pub journal: String,
    pub pub_date: String,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub pmc_id: Option<String>,
    pub article_types: Vec<String>,
    pub keywords: Vec<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub language: Option<String>,
    pub journal_abbreviation: Option<String>,
    pub issn: Option<String>,
}

impl From<PubMedArticle> for JsArticle {
    fn from(article: PubMedArticle) -> Self {
        // Convert Author structs to simple strings
        let author_names: Vec<String> = article
            .authors
            .into_iter()
            .map(|author| author.full_name)
            .collect();

        Self {
            pmid: article.pmid,
            title: article.title,
            authors: author_names,
            journal: article.journal,
            pub_date: article.pub_date,
            abstract_text: article.abstract_text,
            doi: article.doi,
            pmc_id: article.pmc_id,
            article_types: article.article_types,
            keywords: article.keywords.unwrap_or_default(),
            volume: article.volume,
            issue: article.issue,
            pages: article.pages,
            language: article.language,
            journal_abbreviation: article.journal_abbreviation,
            issn: article.issn,
        }
    }
}

/// JavaScript-friendly lightweight article summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsSummary {
    pub pmid: String,
    pub title: String,
    pub authors: Vec<String>,
    pub journal: String,
    pub full_journal_name: String,
    pub pub_date: String,
    pub epub_date: String,
    pub doi: Option<String>,
    pub pmc_id: Option<String>,
    pub volume: String,
    pub issue: String,
    pub pages: String,
    pub languages: Vec<String>,
    pub pub_types: Vec<String>,
    pub issn: String,
    pub essn: String,
    pub sort_pub_date: String,
    pub pmc_ref_count: u64,
    pub record_status: String,
}

impl From<ArticleSummary> for JsSummary {
    fn from(s: ArticleSummary) -> Self {
        Self {
            pmid: s.pmid,
            title: s.title,
            authors: s.authors,
            journal: s.journal,
            full_journal_name: s.full_journal_name,
            pub_date: s.pub_date,
            epub_date: s.epub_date,
            doi: s.doi,
            pmc_id: s.pmc_id,
            volume: s.volume,
            issue: s.issue,
            pages: s.pages,
            languages: s.languages,
            pub_types: s.pub_types,
            issn: s.issn,
            essn: s.essn,
            sort_pub_date: s.sort_pub_date,
            pmc_ref_count: s.pmc_ref_count,
            record_status: s.record_status,
        }
    }
}

/// JavaScript-friendly full text representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsFullText {
    pub pmcid: String,
    pub pmid: Option<String>,
    pub title: Option<String>,
    pub authors: Vec<JsAuthor>,
    pub journal: JsJournal,
    pub pub_date: String,
    pub doi: Option<String>,
    pub sections: Vec<JsSection>,
    pub references: Vec<JsReference>,
    pub article_type: Option<String>,
    pub keywords: Vec<String>,
    pub funding: Vec<JsFunding>,
    pub conflict_of_interest: Option<String>,
    pub acknowledgments: Option<String>,
    pub data_availability: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsFunding {
    pub source: Option<String>,
    pub award_id: Option<String>,
    pub statement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsAuthor {
    pub given_names: Option<String>,
    pub surname: Option<String>,
    pub full_name: String,
    pub email: Option<String>,
    pub affiliations: Vec<String>,
    pub is_corresponding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsJournal {
    pub title: Option<String>,
    pub abbreviation: Option<String>,
    pub publisher: Option<String>,
    pub issn_print: Option<String>,
    pub issn_electronic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsSection {
    pub section_type: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub subsections: Vec<JsSection>,
    pub figures: Vec<JsFigure>,
    pub tables: Vec<JsTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsFigure {
    pub id: String,
    pub label: Option<String>,
    pub caption: Option<String>,
    pub graphic_href: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsTable {
    pub id: String,
    pub label: Option<String>,
    pub caption: Option<String>,
    pub footnotes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct JsReference {
    pub id: String,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub journal: Option<String>,
    pub year: Option<String>,
    pub pmid: Option<String>,
    pub doi: Option<String>,
}

impl From<PmcArticle> for JsFullText {
    fn from(article: PmcArticle) -> Self {
        let PmcArticle {
            article_type,
            front,
            body,
            back,
            data_availability,
            ..
        } = article;
        let journal_meta = front.journal_meta;
        let meta = front.article_meta;
        let sections = body.map(|b| b.sections).unwrap_or_default();
        let back = back.unwrap_or(pubmed_client::pmc::Back {
            acknowledgments: None,
            conflict_of_interest: None,
            references: Vec::new(),
            appendices: Vec::new(),
            glossary: Vec::new(),
        });
        Self {
            pmcid: meta.pmcid.to_string(),
            pmid: meta.pmid.map(|p| p.to_string()),
            title: meta.title_group.article_title,
            authors: meta.authors.into_iter().map(JsAuthor::from).collect(),
            journal: JsJournal::from(journal_meta),
            pub_date: meta
                .pub_dates
                .first()
                .map(|d| {
                    let mut s = String::new();
                    if let Some(y) = d.year {
                        s.push_str(&y.to_string());
                    }
                    if let Some(m) = d.month {
                        s.push_str(&format!("-{:02}", m));
                    }
                    if let Some(day) = d.day {
                        s.push_str(&format!("-{:02}", day));
                    }
                    s
                })
                .unwrap_or_default(),
            doi: meta.doi,
            sections: sections.into_iter().map(JsSection::from).collect(),
            references: back.references.into_iter().map(JsReference::from).collect(),
            article_type,
            keywords: meta.keywords,
            funding: meta
                .funding
                .into_iter()
                .map(|f| JsFunding {
                    source: f.source,
                    award_id: f.award_id,
                    statement: f.statement,
                })
                .collect(),
            conflict_of_interest: back.conflict_of_interest,
            acknowledgments: back.acknowledgments,
            data_availability,
        }
    }
}

impl From<JsFullText> for PmcArticle {
    fn from(js: JsFullText) -> Self {
        use pubmed_client::pmc::{ArticleMeta, Back, Body, Front, JournalMeta, TitleGroup};
        Self {
            article_type: js.article_type,
            front: Front {
                journal_meta: JournalMeta::from(js.journal),
                article_meta: ArticleMeta {
                    pmcid: pubmed_client::PmcId::parse(&js.pmcid)
                        .unwrap_or_else(|_| pubmed_client::PmcId::from_u32(1)),
                    pmid: js
                        .pmid
                        .and_then(|p| pubmed_client::PubMedId::parse(&p).ok()),
                    doi: js.doi,
                    categories: Vec::new(),
                    title_group: TitleGroup {
                        article_title: js.title,
                        subtitle: None,
                    },
                    authors: js.authors.into_iter().map(|a| a.into()).collect(),
                    pub_dates: Vec::new(),
                    volume: None,
                    issue: None,
                    fpage: None,
                    lpage: None,
                    elocation_id: None,
                    history: Vec::new(),
                    permissions: None,
                    abstracts: Vec::new(),
                    keywords: js.keywords,
                    keyword_groups: Vec::new(),
                    subject_groups: Vec::new(),
                    related_articles: Vec::new(),
                    author_notes: Vec::new(),
                    funding: js
                        .funding
                        .into_iter()
                        .map(|f| pubmed_client::pmc::FundingInfo {
                            source: f.source,
                            award_id: f.award_id,
                            statement: f.statement,
                        })
                        .collect(),
                },
            },
            body: Some(Body {
                sections: js.sections.into_iter().map(|s| s.into()).collect(),
            }),
            back: Some(Back {
                acknowledgments: js.acknowledgments,
                conflict_of_interest: js.conflict_of_interest,
                references: js.references.into_iter().map(|r| r.into()).collect(),
                appendices: Vec::new(),
                glossary: Vec::new(),
            }),
            supplementary_materials: Vec::new(),
            data_availability: js.data_availability,
        }
    }
}

impl From<pubmed_client::Author> for JsAuthor {
    fn from(author: pubmed_client::Author) -> Self {
        // Convert affiliations to simple strings
        let affiliation_names: Vec<String> = author
            .affiliations
            .into_iter()
            .filter_map(|a| a.institution)
            .collect();

        Self {
            given_names: author.given_names,
            surname: author.surname,
            full_name: author.full_name,
            email: author.email,
            affiliations: affiliation_names,
            is_corresponding: author.is_corresponding,
        }
    }
}

impl From<JsAuthor> for pubmed_client::Author {
    fn from(js: JsAuthor) -> Self {
        let affiliations = js
            .affiliations
            .into_iter()
            .map(|name| pubmed_client::Affiliation {
                id: None,
                institution: Some(name),
                department: None,
                address: None,
                country: None,
            })
            .collect();

        Self {
            given_names: js.given_names,
            surname: js.surname,
            initials: None,
            suffix: None,
            full_name: js.full_name,
            affiliations,
            orcid: None,
            email: js.email,
            roles: Vec::new(),
            collab_name: None,
            is_corresponding: js.is_corresponding,
        }
    }
}

impl From<pubmed_client::pmc::JournalMeta> for JsJournal {
    fn from(journal: pubmed_client::pmc::JournalMeta) -> Self {
        Self {
            title: journal.title,
            abbreviation: journal.abbreviation,
            publisher: journal.publisher,
            issn_print: journal.issn_print,
            issn_electronic: journal.issn_electronic,
        }
    }
}

impl From<JsJournal> for pubmed_client::pmc::JournalMeta {
    fn from(js: JsJournal) -> Self {
        Self {
            title: js.title,
            abbreviation: js.abbreviation,
            issn_print: js.issn_print,
            issn_electronic: js.issn_electronic,
            publisher: js.publisher,
        }
    }
}

impl From<pubmed_client::pmc::Section> for JsSection {
    fn from(section: pubmed_client::pmc::Section) -> Self {
        Self {
            section_type: section.section_type,
            title: section.title,
            content: section.content,
            subsections: section
                .subsections
                .into_iter()
                .map(JsSection::from)
                .collect(),
            figures: section
                .figures
                .into_iter()
                .map(|f| JsFigure {
                    id: f.id,
                    label: f.label,
                    caption: f.caption,
                    graphic_href: f.graphic_href,
                })
                .collect(),
            tables: section
                .tables
                .into_iter()
                .map(|t| JsTable {
                    id: t.id,
                    label: t.label,
                    caption: t.caption,
                    footnotes: t.footnotes,
                })
                .collect(),
        }
    }
}

impl From<JsSection> for pubmed_client::pmc::Section {
    fn from(js: JsSection) -> Self {
        Self {
            id: None,
            section_type: js.section_type,
            label: None,
            title: js.title,
            content: js.content,
            subsections: js.subsections.into_iter().map(|s| s.into()).collect(),
            figures: js
                .figures
                .into_iter()
                .map(|f| pubmed_client::pmc::Figure {
                    id: f.id,
                    label: f.label,
                    caption: f.caption,
                    alt_text: None,
                    fig_type: None,
                    graphic_href: f.graphic_href,
                })
                .collect(),
            tables: js
                .tables
                .into_iter()
                .map(|t| pubmed_client::pmc::Table {
                    id: t.id,
                    label: t.label,
                    caption: t.caption,
                    head: Vec::new(),
                    body: Vec::new(),
                    footnotes: t.footnotes,
                })
                .collect(),
            formulas: Vec::new(),
            cited_reference_ids: Vec::new(),
        }
    }
}

impl From<pubmed_client::pmc::Reference> for JsReference {
    fn from(reference: pubmed_client::pmc::Reference) -> Self {
        // Convert Author structs to simple strings
        let author_names: Vec<String> = reference
            .authors
            .into_iter()
            .map(|author| author.full_name)
            .collect();

        Self {
            id: reference.id,
            title: reference.title,
            authors: author_names,
            journal: reference.source,
            year: reference.year,
            pmid: reference.pmid,
            doi: reference.doi,
        }
    }
}

impl From<JsReference> for pubmed_client::pmc::Reference {
    fn from(js: JsReference) -> Self {
        // Convert simple strings back to Author structs
        let authors: Vec<pubmed_client::Author> = js
            .authors
            .into_iter()
            .map(pubmed_client::Author::from_full_name)
            .collect();

        Self {
            id: js.id,
            publication_type: None,
            title: js.title,
            authors,
            editors: Vec::new(),
            source: js.journal,
            year: js.year,
            volume: None,
            issue: None,
            pages: None,
            elocation_id: None,
            publisher_name: None,
            publisher_loc: None,
            edition: None,
            isbn: None,
            conf_name: None,
            pmid: js.pmid,
            doi: js.doi,
        }
    }
}

// ================================================================================================
// ECitMatch types for WASM
// ================================================================================================

/// JavaScript-friendly citation query input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsCitationQuery {
    pub journal: String,
    pub year: String,
    pub volume: String,
    pub first_page: String,
    pub author_name: String,
    pub key: String,
}

/// JavaScript-friendly citation match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsCitationMatch {
    pub journal: String,
    pub year: String,
    pub volume: String,
    pub first_page: String,
    pub author_name: String,
    pub key: String,
    pub pmid: Option<String>,
    pub status: String,
}

impl From<&pubmed_client::CitationMatch> for JsCitationMatch {
    fn from(m: &pubmed_client::CitationMatch) -> Self {
        let status = match m.status {
            pubmed_client::CitationMatchStatus::Found => "found",
            pubmed_client::CitationMatchStatus::NotFound => "not_found",
            pubmed_client::CitationMatchStatus::Ambiguous => "ambiguous",
        };
        Self {
            journal: m.journal.clone(),
            year: m.year.clone(),
            volume: m.volume.clone(),
            first_page: m.first_page.clone(),
            author_name: m.author_name.clone(),
            key: m.key.clone(),
            pmid: m.pmid.clone(),
            status: status.to_string(),
        }
    }
}

// ================================================================================================
// EGQuery types for WASM
// ================================================================================================

/// JavaScript-friendly database count result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsDatabaseCount {
    pub db_name: String,
    pub menu_name: String,
    pub count: u64,
    pub status: String,
}

/// JavaScript-friendly global query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsGlobalQueryResults {
    pub term: String,
    pub results: Vec<JsDatabaseCount>,
}

impl From<pubmed_client::GlobalQueryResults> for JsGlobalQueryResults {
    fn from(results: pubmed_client::GlobalQueryResults) -> Self {
        Self {
            term: results.term,
            results: results
                .results
                .into_iter()
                .map(|r| JsDatabaseCount {
                    db_name: r.db_name,
                    menu_name: r.menu_name,
                    count: r.count,
                    status: r.status,
                })
                .collect(),
        }
    }
}

// ================================================================================================
// ESpell types for WASM
// ================================================================================================

/// JavaScript-friendly spell check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsSpellCheckResult {
    pub database: String,
    pub query: String,
    pub corrected_query: String,
    pub has_corrections: bool,
    pub replacements: Vec<String>,
}

impl From<pubmed_client::SpellCheckResult> for JsSpellCheckResult {
    fn from(result: pubmed_client::SpellCheckResult) -> Self {
        let has_corrections = result.has_corrections();
        let replacements = result
            .replacements()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            database: result.database,
            query: result.query,
            corrected_query: result.corrected_query,
            has_corrections,
            replacements,
        }
    }
}
