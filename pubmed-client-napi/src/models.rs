use napi_derive::napi;
use pubmed_client::{
    pmc::PmcArticle,
    pubmed::{ArticleSummary, PubMedArticle},
};

/// Author information
#[napi(object)]
pub struct Author {
    /// Full name of the author
    pub full_name: String,
    /// ORCID identifier if available
    pub orcid: Option<String>,
    /// Primary affiliation
    pub affiliation: Option<String>,
}

/// PubMed article metadata
#[napi(object)]
pub struct Article {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// List of authors
    pub authors: Vec<Author>,
    /// Journal name
    pub journal: String,
    /// Publication date
    pub pub_date: String,
    /// DOI if available
    pub doi: Option<String>,
    /// PMC ID if available
    pub pmc_id: Option<String>,
    /// Abstract text
    pub abstract_text: Option<String>,
    /// Article types (e.g., "Research Article", "Review")
    pub article_types: Vec<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// Journal volume (e.g., "88")
    pub volume: Option<String>,
    /// Journal issue number (e.g., "3")
    pub issue: Option<String>,
    /// Page range (e.g., "123-130")
    pub pages: Option<String>,
    /// Article language (e.g., "eng")
    pub language: Option<String>,
    /// ISO journal abbreviation (e.g., "J Biol Chem")
    pub journal_abbreviation: Option<String>,
    /// ISSN
    pub issn: Option<String>,
}

impl From<PubMedArticle> for Article {
    fn from(article: PubMedArticle) -> Self {
        Article {
            pmid: article.pmid,
            title: article.title,
            authors: article
                .authors
                .into_iter()
                .map(|a| {
                    let affiliation = a.primary_affiliation().map(|aff| {
                        aff.institution
                            .clone()
                            .unwrap_or_else(|| aff.address.clone().unwrap_or_default())
                    });
                    Author {
                        full_name: a.full_name,
                        orcid: a.orcid,
                        affiliation,
                    }
                })
                .collect(),
            journal: article.journal,
            pub_date: article.pub_date,
            doi: article.doi,
            pmc_id: article.pmc_id,
            abstract_text: article.abstract_text,
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

/// Lightweight article summary from ESummary API
///
/// Contains basic metadata without abstracts, MeSH terms, or chemical lists.
/// Use fetchSummaries() for faster bulk metadata retrieval.
#[napi(object)]
pub struct Summary {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// Author names
    pub authors: Vec<String>,
    /// Journal name
    pub journal: String,
    /// Full journal name
    pub full_journal_name: String,
    /// Publication date
    pub pub_date: String,
    /// Electronic publication date
    pub epub_date: String,
    /// DOI if available
    pub doi: Option<String>,
    /// PMC ID if available
    pub pmc_id: Option<String>,
    /// Journal volume
    pub volume: String,
    /// Journal issue
    pub issue: String,
    /// Page range
    pub pages: String,
    /// Languages
    pub languages: Vec<String>,
    /// Publication types
    pub pub_types: Vec<String>,
    /// ISSN
    pub issn: String,
    /// Electronic ISSN
    pub essn: String,
    /// Sorted publication date
    pub sort_pub_date: String,
    /// PMC reference count
    pub pmc_ref_count: u32,
    /// Record status
    pub record_status: String,
}

impl From<ArticleSummary> for Summary {
    fn from(s: ArticleSummary) -> Self {
        Summary {
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
            pmc_ref_count: s.pmc_ref_count as u32,
            record_status: s.record_status,
        }
    }
}

/// Reference information from PMC articles
#[napi(object)]
pub struct Reference {
    /// Reference ID
    pub id: String,
    /// Reference title
    pub title: Option<String>,
    /// Authors as a single string
    pub authors: String,
    /// Journal name
    pub journal: Option<String>,
    /// Publication year
    pub year: Option<String>,
    /// PubMed ID if available
    pub pmid: Option<String>,
    /// DOI if available
    pub doi: Option<String>,
}

/// Section of a PMC article
#[napi(object)]
pub struct Section {
    /// Section type (e.g., "introduction", "methods")
    pub section_type: Option<String>,
    /// Section title
    pub title: Option<String>,
    /// Section content
    pub content: String,
}

/// Information about OA (Open Access) subset availability for a PMC article
///
/// The OA subset contains articles with programmatic access to full-text XML.
/// Not all PMC articles are in the OA subset - some publishers restrict programmatic access
/// even though the article may be viewable on the PMC website.
#[napi(object)]
pub struct OaSubsetInfo {
    /// PMC ID (e.g., "PMC7906746")
    pub pmcid: String,
    /// Whether the article is in the OA subset
    pub is_oa_subset: bool,
    /// Citation string (if available)
    pub citation: Option<String>,
    /// License type (if available)
    pub license: Option<String>,
    /// Whether the article is retracted
    pub retracted: bool,
    /// Download link for tar.gz package (if available)
    pub download_link: Option<String>,
    /// Format of the download (e.g., "tgz", "pdf")
    pub download_format: Option<String>,
    /// Last updated timestamp for the download
    pub updated: Option<String>,
    /// Error code if not in OA subset
    pub error_code: Option<String>,
    /// Error message if not in OA subset
    pub error_message: Option<String>,
}

impl From<pubmed_client::OaSubsetInfo> for OaSubsetInfo {
    fn from(info: pubmed_client::OaSubsetInfo) -> Self {
        OaSubsetInfo {
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

/// Spelling suggestion result from the ESpell API
#[napi(object)]
pub struct SpellCheckResult {
    /// The database that was queried
    pub database: String,
    /// The original query string as submitted
    pub query: String,
    /// The full corrected/suggested query as a plain string
    pub corrected_query: String,
    /// Whether any spelling corrections were made
    pub has_corrections: bool,
    /// The corrected terms (only the replaced parts)
    pub replacements: Vec<String>,
}

impl From<pubmed_client::SpellCheckResult> for SpellCheckResult {
    fn from(result: pubmed_client::SpellCheckResult) -> Self {
        let has_corrections = result.has_corrections();
        let replacements = result
            .replacements()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        SpellCheckResult {
            database: result.database,
            query: result.query,
            corrected_query: result.corrected_query,
            has_corrections,
            replacements,
        }
    }
}

/// Full-text article from PMC
#[napi(object)]
pub struct FullTextArticle {
    /// PMC ID
    pub pmcid: String,
    /// PubMed ID if available
    pub pmid: Option<String>,
    /// Article title
    pub title: Option<String>,
    /// List of authors
    pub authors: Vec<Author>,
    /// Journal name
    pub journal: Option<String>,
    /// Publication date
    pub pub_date: String,
    /// DOI if available
    pub doi: Option<String>,
    /// Article sections
    pub sections: Vec<Section>,
    /// References
    pub references: Vec<Reference>,
    /// Keywords
    pub keywords: Vec<String>,
}

impl From<PmcArticle> for FullTextArticle {
    fn from(article: PmcArticle) -> Self {
        let PmcArticle {
            front, body, back, ..
        } = article;
        let journal_meta = front.journal_meta;
        let meta = front.article_meta;
        let sections = body.map(|b| b.sections).unwrap_or_default();
        let references = back.map(|b| b.references).unwrap_or_default();
        FullTextArticle {
            pmcid: meta.pmcid.to_string(),
            pmid: meta.pmid.map(|p| p.to_string()),
            title: meta.title_group.article_title,
            authors: meta
                .authors
                .into_iter()
                .map(|a| {
                    let affiliation = a.primary_affiliation().map(|aff| {
                        aff.institution
                            .clone()
                            .unwrap_or_else(|| aff.address.clone().unwrap_or_default())
                    });
                    Author {
                        full_name: a.full_name,
                        orcid: a.orcid,
                        affiliation,
                    }
                })
                .collect(),
            journal: journal_meta.title,
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
            sections: sections
                .into_iter()
                .map(|s| Section {
                    section_type: s.section_type,
                    title: s.title,
                    content: s.content,
                })
                .collect(),
            references: references
                .into_iter()
                .map(|r| Reference {
                    id: r.id,
                    title: r.title,
                    authors: r
                        .authors
                        .iter()
                        .map(|a| a.full_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    journal: r.source,
                    year: r.year,
                    pmid: r.pmid,
                    doi: r.doi,
                })
                .collect(),
            keywords: meta.keywords,
        }
    }
}

/// Markdown conversion options
#[napi(object)]
#[derive(Default)]
pub struct MarkdownOptions {
    /// Include metadata header
    pub include_metadata: Option<bool>,
    /// Include table of contents
    pub include_toc: Option<bool>,
    /// Use YAML frontmatter for metadata
    pub use_yaml_frontmatter: Option<bool>,
    /// Include ORCID links for authors
    pub include_orcid_links: Option<bool>,
    /// Include figure captions
    pub include_figure_captions: Option<bool>,
}

/// Result from EPost API for uploading PMIDs to the NCBI History server
///
/// Contains WebEnv and query_key identifiers that can be used with
/// subsequent API calls for fetching articles from the history server.
#[napi(object)]
pub struct EPostResult {
    /// WebEnv session identifier
    pub webenv: String,
    /// Query key for the uploaded IDs within the session
    pub query_key: String,
}

impl From<pubmed_client::EPostResult> for EPostResult {
    fn from(result: pubmed_client::EPostResult) -> Self {
        EPostResult {
            webenv: result.webenv,
            query_key: result.query_key,
        }
    }
}

/// Related articles result from the ELink API
#[napi(object)]
pub struct RelatedArticles {
    /// Source PMIDs the relations were computed from
    pub source_pmids: Vec<u32>,
    /// PMIDs of related articles
    pub related_pmids: Vec<u32>,
    /// ELink link type (e.g., "pubmed_pubmed")
    pub link_type: String,
}

impl From<pubmed_client::RelatedArticles> for RelatedArticles {
    fn from(related: pubmed_client::RelatedArticles) -> Self {
        RelatedArticles {
            source_pmids: related.source_pmids,
            related_pmids: related.related_pmids,
            link_type: related.link_type,
        }
    }
}

/// PMC availability links from the ELink API
#[napi(object)]
pub struct PmcLinks {
    /// Source PMIDs the links were computed from
    pub source_pmids: Vec<u32>,
    /// PMC IDs available for the source PMIDs
    pub pmc_ids: Vec<String>,
}

impl From<pubmed_client::PmcLinks> for PmcLinks {
    fn from(links: pubmed_client::PmcLinks) -> Self {
        PmcLinks {
            source_pmids: links.source_pmids,
            pmc_ids: links.pmc_ids,
        }
    }
}

/// Citing articles result from the ELink API
#[napi(object)]
pub struct Citations {
    /// Source PMIDs the citations were computed from
    pub source_pmids: Vec<u32>,
    /// PMIDs of articles citing the source PMIDs
    pub citing_pmids: Vec<u32>,
}

impl From<pubmed_client::Citations> for Citations {
    fn from(citations: pubmed_client::Citations) -> Self {
        Citations {
            source_pmids: citations.source_pmids,
            citing_pmids: citations.citing_pmids,
        }
    }
}

/// Detailed information about an NCBI database from the EInfo API
#[napi(object)]
pub struct DatabaseInfo {
    /// Internal database name (e.g., "pubmed")
    pub name: String,
    /// Human-readable database name
    pub menu_name: String,
    /// Database description
    pub description: String,
    /// Build identifier if available
    pub build: Option<String>,
    /// Number of records if available
    pub count: Option<i64>,
    /// Last update timestamp if available
    pub last_update: Option<String>,
}

impl From<pubmed_client::DatabaseInfo> for DatabaseInfo {
    fn from(info: pubmed_client::DatabaseInfo) -> Self {
        DatabaseInfo {
            name: info.name,
            menu_name: info.menu_name,
            description: info.description,
            build: info.build,
            count: info.count.map(|c| c as i64),
            last_update: info.last_update,
        }
    }
}

/// Input for a single citation match query (ECitMatch API)
#[napi(object)]
pub struct CitationQuery {
    /// Journal title
    pub journal: String,
    /// Publication year
    pub year: String,
    /// Volume
    pub volume: String,
    /// First page
    pub first_page: String,
    /// First author name
    pub author_name: String,
    /// User-defined key echoed back in the result
    pub key: String,
}

impl From<&CitationQuery> for pubmed_client::CitationQuery {
    fn from(query: &CitationQuery) -> Self {
        pubmed_client::CitationQuery::new(
            &query.journal,
            &query.year,
            &query.volume,
            &query.first_page,
            &query.author_name,
            &query.key,
        )
    }
}

/// Result of a single citation match (ECitMatch API)
#[napi(object)]
pub struct CitationMatch {
    /// Journal title from the query
    pub journal: String,
    /// Year from the query
    pub year: String,
    /// Volume from the query
    pub volume: String,
    /// First page from the query
    pub first_page: String,
    /// Author name from the query
    pub author_name: String,
    /// User-defined key from the query
    pub key: String,
    /// Matched PMID (null if not found)
    pub pmid: Option<String>,
    /// Match status ("found", "not_found", or "ambiguous")
    pub status: String,
}

impl From<&pubmed_client::CitationMatch> for CitationMatch {
    fn from(m: &pubmed_client::CitationMatch) -> Self {
        let status = match m.status {
            pubmed_client::CitationMatchStatus::Found => "found",
            pubmed_client::CitationMatchStatus::NotFound => "not_found",
            pubmed_client::CitationMatchStatus::Ambiguous => "ambiguous",
        };
        CitationMatch {
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

/// Record count for a single NCBI database from the EGQuery API
#[napi(object)]
pub struct DatabaseCount {
    /// Internal database name (e.g., "pubmed", "pmc")
    pub db_name: String,
    /// Human-readable database name
    pub menu_name: String,
    /// Number of matching records
    pub count: i64,
    /// Query status (e.g., "Ok")
    pub status: String,
}

impl From<&pubmed_client::DatabaseCount> for DatabaseCount {
    fn from(dc: &pubmed_client::DatabaseCount) -> Self {
        DatabaseCount {
            db_name: dc.db_name.clone(),
            menu_name: dc.menu_name.clone(),
            count: dc.count as i64,
            status: dc.status.clone(),
        }
    }
}

/// Results from the EGQuery API for a global database search
#[napi(object)]
pub struct GlobalQueryResults {
    /// The query term that was searched
    pub term: String,
    /// Per-database record counts
    pub results: Vec<DatabaseCount>,
}

impl From<pubmed_client::GlobalQueryResults> for GlobalQueryResults {
    fn from(results: pubmed_client::GlobalQueryResults) -> Self {
        GlobalQueryResults {
            term: results.term,
            results: results.results.iter().map(DatabaseCount::from).collect(),
        }
    }
}

/// Figure metadata extracted from a PMC article
#[napi(object)]
pub struct Figure {
    /// Figure ID
    pub id: String,
    /// Figure label (e.g., "Figure 1")
    pub label: Option<String>,
    /// Figure caption
    pub caption: Option<String>,
    /// Alternative text
    pub alt_text: Option<String>,
    /// Figure type
    pub fig_type: Option<String>,
    /// Reference to the graphic file
    pub graphic_href: Option<String>,
}

impl From<&pubmed_client::Figure> for Figure {
    fn from(figure: &pubmed_client::Figure) -> Self {
        Figure {
            id: figure.id.clone(),
            label: figure.label.clone(),
            caption: figure.caption.clone(),
            alt_text: figure.alt_text.clone(),
            fig_type: figure.fig_type.clone(),
            graphic_href: figure.graphic_href.clone(),
        }
    }
}

/// A figure extracted from a downloaded PMC OA Cloud article, with file metadata
#[napi(object)]
pub struct ExtractedFigure {
    /// Figure metadata
    pub figure: Figure,
    /// Path to the extracted image file
    pub extracted_file_path: String,
    /// File size in bytes if available
    pub file_size: Option<i64>,
    /// Image width in pixels if available
    pub width: Option<u32>,
    /// Image height in pixels if available
    pub height: Option<u32>,
}

impl From<&pubmed_client::ExtractedFigure> for ExtractedFigure {
    fn from(extracted: &pubmed_client::ExtractedFigure) -> Self {
        let (width, height) = match extracted.dimensions {
            Some((w, h)) => (Some(w), Some(h)),
            None => (None, None),
        };
        ExtractedFigure {
            figure: Figure::from(&extracted.figure),
            extracted_file_path: extracted.extracted_file_path.clone(),
            file_size: extracted.file_size.map(|s| s as i64),
            width,
            height,
        }
    }
}
