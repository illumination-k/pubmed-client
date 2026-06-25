//! PMC domain models based on JATS Archiving 1.4 DTD
//!
//! **This module is the single model layer for PMC full-text articles.**
//! All parsing produces these types directly; there is no separate
//! intermediate parser model.
//!
//! The type hierarchy mirrors the JATS `<article>` content model:
//!
//! ```text
//! <article>            → PmcArticle
//!   <front>            → Front
//!     <journal-meta>   → JournalMeta
//!     <article-meta>   → ArticleMeta (ids, TitleGroup, contributors,
//!                         pub info, history, Permissions, Abstract,
//!                         keywords, funding)
//!   <body>             → Body (Vec<Section>, recursive)
//!   <back>             → Back (ack, COI, Vec<Reference>, appendices, glossary)
//! ```
//!
//! Design principles:
//! - DTD-faithful: every field maps to a JATS element/attribute, structured
//!   following the DTD hierarchy and declaration order
//! - No extraction concerns: fields like `file_path` or inferred `file_type` are excluded
//! - Type-safe IDs: uses `PmcId` / `PubMedId` instead of raw strings
//! - Reuses shared types: `Author` and `HistoryDate` from `common::models`
//! - Text mining ready: structured abstracts, table content, formulas, definitions
//!
//! Documented deviations from strict DTD structure:
//! - `supplementary_materials` and `data_availability` live on [`PmcArticle`]
//!   because in real PMC XML they appear in `<body>` sections, `<back>`, or
//!   `<floats-group>`; the parser collects them document-wide without
//!   tracking their position
//! - `<contrib-group>` is flattened to `Vec<Author>` (only author contribs
//!   are modeled)
//!
//! Flattened read access is provided through accessor methods on
//! [`PmcArticle`] (e.g. [`PmcArticle::title`], [`PmcArticle::sections`]).

use crate::common::{Author, HistoryDate, PmcId, PubMedId, PublicationDate};
use serde::{Deserialize, Serialize};

// ============================================================================
// Top-level article
// ============================================================================

/// PMC full-text article.
///
/// Maps to JATS `<article>`: `front, body?, back?`.
///
/// DTD: <https://jats.nlm.nih.gov/archiving/tag-library/1.4/element/article.html>
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PmcArticle {
    /// Article type. From `<article article-type="...">` attribute.
    pub article_type: Option<String>,
    /// Front matter. From `<front>`.
    pub front: Front,
    /// Article body. From `<body>`. `None` when the article has no body
    /// (e.g., metadata-only records).
    pub body: Option<Body>,
    /// Back matter. From `<back>`. `None` when the article has no back matter.
    pub back: Option<Back>,

    // --- Document-wide collections (deviation from strict DTD placement) ---
    /// Supplementary materials. From `<supplementary-material>`, collected
    /// from the entire document (`<body>` sections, `<back>`, or `<floats-group>`).
    pub supplementary_materials: Vec<SupplementaryMaterial>,
    /// Data availability statement. From `<sec sec-type="data-availability">`
    /// or `<notes notes-type="data-availability">`, wherever it appears.
    pub data_availability: Option<String>,
}

// ============================================================================
// Front matter (<front>)
// ============================================================================

/// Front matter.
///
/// Maps to JATS `<front>`: `journal-meta?, article-meta?`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Front {
    /// Journal metadata. From `<journal-meta>`.
    pub journal_meta: JournalMeta,
    /// Article metadata. From `<article-meta>`.
    pub article_meta: ArticleMeta,
}

/// Journal metadata.
///
/// Maps to JATS `<journal-meta>`. Note that `volume` and `issue` are intentionally
/// excluded here — in the DTD they belong to `<article-meta>`, not `<journal-meta>`,
/// and are placed on [`ArticleMeta`] accordingly.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JournalMeta {
    /// Journal title. From `<journal-title-group>/<journal-title>`. `None`
    /// when the element is absent in the XML.
    pub title: Option<String>,
    /// Abbreviated journal title. From `<journal-id journal-id-type="iso-abbrev">`
    /// or `<abbrev-journal-title>`.
    pub abbreviation: Option<String>,
    /// Print ISSN. From `<issn pub-type="ppub">`.
    pub issn_print: Option<String>,
    /// Electronic ISSN. From `<issn pub-type="epub">`.
    pub issn_electronic: Option<String>,
    /// Publisher name. From `<publisher>/<publisher-name>`.
    pub publisher: Option<String>,
}

/// Article metadata.
///
/// Maps to JATS `<article-meta>`. Fields follow the DTD declaration order:
/// `article-id*, article-categories?, title-group, contrib-group*, pub-date*,
/// volume?, issue?, fpage?, lpage?, elocation-id?, history?, permissions?,
/// abstract*, kwd-group*, funding-group*`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArticleMeta {
    // --- Identifiers (<article-id>) ---
    /// PMC ID (e.g., PMC7906746). From `<article-id pub-id-type="pmc">`.
    pub pmcid: PmcId,
    /// PubMed ID. From `<article-id pub-id-type="pmid">`.
    pub pmid: Option<PubMedId>,
    /// DOI. From `<article-id pub-id-type="doi">`.
    pub doi: Option<String>,

    /// Subject categories. From `<article-categories>/<subj-group>/<subject>`.
    pub categories: Vec<String>,

    /// Title group. From `<title-group>`.
    pub title_group: TitleGroup,

    /// Authors. From `<contrib-group>/<contrib contrib-type="author">`.
    pub authors: Vec<Author>,

    /// Publication dates (epub, ppub, collection, etc.). From `<pub-date>`.
    pub pub_dates: Vec<PublicationDate>,
    /// Volume number. From `<volume>`.
    pub volume: Option<String>,
    /// Issue number. From `<issue>`.
    pub issue: Option<String>,
    /// First page. From `<fpage>`.
    pub fpage: Option<String>,
    /// Last page. From `<lpage>`.
    pub lpage: Option<String>,
    /// Electronic location ID. From `<elocation-id>`.
    pub elocation_id: Option<String>,

    /// Publication history dates. From `<history>/<date>`.
    pub history: Vec<HistoryDate>,

    /// Copyright and licensing. From `<permissions>`.
    pub permissions: Option<Permissions>,

    /// Abstracts. From `<abstract>` (repeatable in the DTD, e.g. a main
    /// abstract plus a graphical or teaser abstract).
    pub abstracts: Vec<Abstract>,

    /// Keywords (flattened across all groups). From `<kwd-group>/<kwd>`.
    pub keywords: Vec<String>,

    /// Keyword groups, preserving `kwd-group-type` and `xml:lang`. From
    /// `<kwd-group>`. This is the structured counterpart of [`keywords`](Self::keywords),
    /// distinguishing e.g. author-supplied keywords from MeSH-derived ones.
    #[serde(default)]
    pub keyword_groups: Vec<KeywordGroup>,

    /// Subject groups, preserving `subj-group-type`. From
    /// `<article-categories>/<subj-group>`. Structured counterpart of
    /// [`categories`](Self::categories).
    #[serde(default)]
    pub subject_groups: Vec<SubjectGroup>,

    /// Related articles (corrections, retractions, companion/peer-reviewed
    /// articles). From `<related-article>`.
    #[serde(default)]
    pub related_articles: Vec<RelatedArticle>,

    /// Author notes (correspondence details, equal-contribution and present-
    /// address statements). From `<author-notes>` (`<corresp>` / `<fn>`).
    #[serde(default)]
    pub author_notes: Vec<String>,

    /// Funding information. From `<funding-group>/<award-group>`.
    pub funding: Vec<FundingInfo>,
}

/// A keyword group.
///
/// Maps to JATS `<kwd-group>`. Articles frequently carry several groups — e.g.
/// author-supplied keywords plus publisher subject terms — distinguished by
/// `@kwd-group-type` and `@xml:lang`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct KeywordGroup {
    /// Group type (e.g., "author", "npg-subject"). From `@kwd-group-type`.
    pub group_type: Option<String>,
    /// Language of the keywords. From `@xml:lang`.
    pub lang: Option<String>,
    /// Keywords in this group. From `<kwd>`.
    pub keywords: Vec<String>,
}

/// A subject (article-category) group.
///
/// Maps to JATS `<article-categories>/<subj-group>`. `@subj-group-type` carries
/// the role of the terms (e.g. "heading", "discipline").
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SubjectGroup {
    /// Group type (e.g., "heading", "discipline"). From `@subj-group-type`.
    pub group_type: Option<String>,
    /// Subjects in this group. From `<subject>`.
    pub subjects: Vec<String>,
}

/// A link to a related article.
///
/// Maps to JATS `<related-article>`. The `related_article_type` drives common
/// text-mining use cases such as detecting corrections and retractions
/// (e.g. `"corrected-article"`, `"retracted-article"`, `"companion"`,
/// `"peer-reviewed-article"`).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RelatedArticle {
    /// Relationship type. From `@related-article-type`.
    pub related_article_type: Option<String>,
    /// External link type (e.g., "doi", "pmc"). From `@ext-link-type`.
    pub ext_link_type: Option<String>,
    /// Link target (DOI, PMC id, URL). From `@xlink:href`.
    pub href: Option<String>,
    /// Element id. From `@id`.
    pub id: Option<String>,
}

impl RelatedArticle {
    /// Whether this link points to an article corrected by the current one
    /// (`related-article-type="corrected-article"`).
    pub fn is_correction(&self) -> bool {
        self.related_article_type.as_deref() == Some("corrected-article")
    }

    /// Whether this link points to an article retracted by the current one
    /// (`related-article-type="retracted-article"`).
    pub fn is_retraction(&self) -> bool {
        self.related_article_type.as_deref() == Some("retracted-article")
    }
}

/// Title group.
///
/// Maps to JATS `<title-group>`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TitleGroup {
    /// Article title. From `<article-title>`. `None` when the element is
    /// absent in the XML.
    pub article_title: Option<String>,
    /// Article subtitle. From `<subtitle>`.
    pub subtitle: Option<String>,
}

/// Copyright and licensing information.
///
/// Maps to JATS `<permissions>`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Permissions {
    /// Copyright statement. From `<copyright-statement>`
    /// (falls back to `<copyright-year>`).
    pub copyright_statement: Option<String>,
    /// License. From `<license>`.
    pub license: Option<License>,
}

/// License information.
///
/// Maps to JATS `<license>`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct License {
    /// License URL. From `<license xlink:href="...">` or `<ali:license_ref>`.
    pub href: Option<String>,
    /// License text. From `<license-p>` content.
    pub text: Option<String>,
}

/// Abstract.
///
/// Maps to JATS `<abstract>`. The DTD allows multiple abstracts
/// distinguished by `@abstract-type`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Abstract {
    /// Abstract type (e.g., "graphical", "teaser"). From `<abstract abstract-type="...">`.
    pub abstract_type: Option<String>,
    /// Plain abstract text (flattened). Concatenation of all `<p>` texts.
    pub text: String,
    /// Structured abstract sections. From `<abstract>/<sec>`.
    /// Present when the abstract has labeled sections (e.g., Background, Methods, Results).
    pub sections: Vec<AbstractSection>,
}

/// Structured abstract section.
///
/// Maps to `<abstract>/<sec>`. Many biomedical journals use structured abstracts
/// with labeled sections (Background, Methods, Results, Conclusions).
/// This structure preserves the section boundaries for text mining pipelines
/// that need to process abstract sections independently.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AbstractSection {
    /// Section label (e.g., "Background", "Methods", "Results", "Conclusions").
    /// From `<title>` inside `<abstract>/<sec>`.
    pub label: Option<String>,
    /// Section text content. From `<p>` inside `<abstract>/<sec>`.
    pub text: String,
}

// ============================================================================
// Body content (<body>)
// ============================================================================

/// Article body.
///
/// Maps to JATS `<body>`: `(p | sec | ...)*`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Body {
    /// Article sections. From `<body>/<sec>`.
    pub sections: Vec<Section>,
}

/// Article section.
///
/// Maps to JATS `<sec>`. Sections form a recursive tree via `subsections`.
/// Figures, tables, and formulas that appear inline within the section are
/// collected in their respective fields.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Section {
    /// Section ID. From `<sec id="...">`.
    pub id: Option<String>,
    /// Section type. From `<sec sec-type="...">`.
    /// Common values: `"intro"`, `"methods"`, `"results"`, `"discussion"`,
    /// `"conclusions"`, `"supplementary-material"`, `"data-availability"`.
    pub section_type: Option<String>,
    /// Section label/number (e.g., "1.", "2.1"). From `<label>`.
    pub label: Option<String>,
    /// Section title. From `<title>`.
    pub title: Option<String>,
    /// Concatenated paragraph text. From `<p>` elements.
    pub content: String,
    /// Nested subsections. From child `<sec>` elements.
    pub subsections: Vec<Section>,
    /// Figures within this section. From `<fig>` elements.
    pub figures: Vec<Figure>,
    /// Tables within this section. From `<table-wrap>` elements.
    pub tables: Vec<Table>,
    /// Display formulas within this section. From `<disp-formula>` elements.
    pub formulas: Vec<Formula>,
}

/// Figure.
///
/// Maps to JATS `<fig>`. The `graphic_href` field contains the domain-level
/// reference to the graphic file (from `<graphic xlink:href="...">`).
/// Actual file extraction paths and sizes belong to the client layer's
/// `ExtractedFigure` type, not here.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Figure {
    /// Figure ID. From `<fig id="...">`.
    pub id: String,
    /// Figure label (e.g., "Figure 1"). From `<label>`.
    pub label: Option<String>,
    /// Figure caption. From `<caption>/<p>`. `None` when the XML element is
    /// absent or could not be parsed.
    pub caption: Option<String>,
    /// Alt text. From `<alt-text>`.
    pub alt_text: Option<String>,
    /// Figure type (e.g., "figure", "scheme", "chart"). From `<fig fig-type="...">`.
    pub fig_type: Option<String>,
    /// Graphic href from the XML. From `<graphic xlink:href="...">`.
    pub graphic_href: Option<String>,
}

/// Table wrapper.
///
/// Maps to JATS `<table-wrap>`. Table body is parsed into structured rows/cells
/// for direct programmatic access without requiring downstream HTML parsing.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Table {
    /// Table ID. From `<table-wrap id="...">`.
    pub id: String,
    /// Table label (e.g., "Table 1"). From `<label>`.
    pub label: Option<String>,
    /// Table caption. From `<caption>/<p>`. `None` when the XML element is
    /// absent or could not be parsed.
    pub caption: Option<String>,
    /// Header rows. From `<thead>/<tr>`.
    pub head: Vec<TableRow>,
    /// Body rows. From `<tbody>/<tr>` (or direct `<tr>` if no `<tbody>`).
    pub body: Vec<TableRow>,
    /// Table footnotes. From `<table-wrap-foot>/<fn>`.
    pub footnotes: Vec<String>,
}

/// A single table row.
///
/// Maps to XHTML `<tr>` inside JATS `<table>`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TableRow {
    /// Cells in this row. From `<th>` or `<td>` elements.
    pub cells: Vec<TableCell>,
}

/// A single table cell.
///
/// Maps to XHTML `<th>` or `<td>` inside JATS `<table>`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TableCell {
    /// Cell text content (XML tags stripped).
    pub content: String,
    /// Whether this is a header cell (`<th>`) vs data cell (`<td>`).
    pub is_header: bool,
    /// Column span. From `@colspan` attribute.
    pub colspan: Option<u32>,
    /// Row span. From `@rowspan` attribute.
    pub rowspan: Option<u32>,
}

/// Display formula.
///
/// Maps to JATS `<disp-formula>`. Formulas can be represented as MathML,
/// TeX/LaTeX, plain text, or as graphic images. The `notation` field indicates
/// which representation is stored in `content`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Formula {
    /// Formula ID. From `<disp-formula id="...">`.
    pub id: Option<String>,
    /// Formula label (e.g., "1", "(1)"). From `<label>`.
    pub label: Option<String>,
    /// Formula content. From `<tex-math>`, `<mml:math>`, or text content.
    pub content: Option<String>,
    /// Notation type indicating the format of `content`.
    /// `"tex"` for `<tex-math>`, `"mathml"` for `<mml:math>`, `"text"` for plain text.
    pub notation: Option<String>,
    /// Graphic href for image-based formulas. From `<graphic xlink:href="...">`.
    pub graphic_href: Option<String>,
}

// ============================================================================
// Back matter (<back>)
// ============================================================================

/// Back matter.
///
/// Maps to JATS `<back>`: `(ack | app-group | bio | fn-group | glossary |
/// ref-list | notes | sec)*`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Back {
    /// Acknowledgments. From `<ack>`.
    pub acknowledgments: Option<String>,
    /// Conflict of interest statement. From `<fn-group>/<fn fn-type="COI-statement">`
    /// or `<sec>` whose title mentions conflicts/competing interests.
    pub conflict_of_interest: Option<String>,
    /// Reference list. From `<ref-list>/<ref>`.
    pub references: Vec<Reference>,
    /// Appendices. From `<app-group>/<app>`.
    pub appendices: Vec<Section>,
    /// Glossary definitions. From `<glossary>/<def-list>`.
    pub glossary: Vec<Definition>,
}

/// Reference citation.
///
/// Maps to JATS `<ref>` containing `<element-citation>` or `<mixed-citation>`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Reference {
    /// Reference ID. From `<ref id="...">`.
    pub id: String,
    /// Publication type. From `<element-citation publication-type="...">`.
    /// Common values: `"journal"`, `"book"`, `"web"`, `"other"`.
    pub publication_type: Option<String>,
    /// Article or chapter title. From `<article-title>` or `<chapter-title>`.
    pub title: Option<String>,
    /// Authors. From `<person-group>/<name>`.
    pub authors: Vec<Author>,
    /// Source (journal name or book title). From `<source>`.
    pub source: Option<String>,
    /// Publication year. From `<year>`.
    pub year: Option<String>,
    /// Volume. From `<volume>`.
    pub volume: Option<String>,
    /// Issue. From `<issue>`.
    pub issue: Option<String>,
    /// Page range. From `<fpage>`-`<lpage>`.
    pub pages: Option<String>,
    /// Electronic location ID. From `<elocation-id>` (used by e-journals in
    /// place of page numbers).
    #[serde(default)]
    pub elocation_id: Option<String>,
    /// Editors. From `<person-group person-group-type="editor">` (common for
    /// book and book-chapter citations).
    #[serde(default)]
    pub editors: Vec<Author>,
    /// Publisher name. From `<publisher-name>` (book / report citations).
    #[serde(default)]
    pub publisher_name: Option<String>,
    /// Publisher location. From `<publisher-loc>`.
    #[serde(default)]
    pub publisher_loc: Option<String>,
    /// Edition. From `<edition>`.
    #[serde(default)]
    pub edition: Option<String>,
    /// ISBN. From `<isbn>` (book citations).
    #[serde(default)]
    pub isbn: Option<String>,
    /// Conference name. From `<conf-name>` (conference-proceedings citations).
    #[serde(default)]
    pub conf_name: Option<String>,
    /// PubMed ID. From `<pub-id pub-id-type="pmid">`.
    pub pmid: Option<String>,
    /// DOI. From `<pub-id pub-id-type="doi">`.
    pub doi: Option<String>,
}

impl Reference {
    /// Format a human-readable citation string.
    pub fn format_citation(&self) -> String {
        let mut parts = Vec::new();

        if !self.authors.is_empty() {
            let author_names: Vec<String> = self
                .authors
                .iter()
                .map(|a| a.full_name.clone())
                .filter(|n| !n.trim().is_empty())
                .collect();
            if !author_names.is_empty() {
                parts.push(author_names.join(", "));
            }
        }

        if let Some(title) = &self.title
            && !title.trim().is_empty()
        {
            parts.push(title.clone());
        }

        if let Some(source) = &self.source
            && !source.trim().is_empty()
        {
            let mut source_part = source.clone();
            if let Some(year) = &self.year
                && !year.trim().is_empty()
                && year != "n.d."
            {
                source_part.push_str(&format!(" ({year})"));
            }
            if let Some(volume) = &self.volume
                && !volume.trim().is_empty()
            {
                source_part.push_str(&format!(" {volume}"));
                if let Some(issue) = &self.issue
                    && !issue.trim().is_empty()
                {
                    source_part.push_str(&format!("({issue})"));
                }
            }
            if let Some(pages) = &self.pages
                && !pages.trim().is_empty()
            {
                source_part.push_str(&format!(": {pages}"));
            }
            parts.push(source_part);
        }

        let result = parts.join(". ");
        if result.trim().is_empty() {
            format!("Reference {}", self.id)
        } else {
            result
        }
    }
}

/// Funding information.
///
/// Maps to JATS `<funding-group>/<award-group>`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FundingInfo {
    /// Funding source/agency. From `<funding-source>`. `None` when the element
    /// is absent in the XML.
    pub source: Option<String>,
    /// Grant/award ID. From `<award-id>`.
    pub award_id: Option<String>,
    /// Funding statement. From `<funding-statement>`.
    pub statement: Option<String>,
}

/// Supplementary material.
///
/// Maps to JATS `<supplementary-material>`. Only contains domain-level
/// data from the XML. Inferred values like file type (derived from URL extension)
/// and layout attributes like `position` are excluded.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SupplementaryMaterial {
    /// Material ID. From `<supplementary-material id="...">`.
    pub id: String,
    /// Content type. From `<supplementary-material content-type="...">`.
    pub content_type: Option<String>,
    /// Title or label. From `<label>` or `<caption>/<title>`.
    pub title: Option<String>,
    /// Description. From `<caption>/<p>`.
    pub description: Option<String>,
    /// Resource href. From `<supplementary-material xlink:href="...">`.
    pub href: Option<String>,
}

impl SupplementaryMaterial {
    /// Check if this material is a tar archive based on the href extension.
    pub fn is_tar_file(&self) -> bool {
        if let Some(url) = &self.href {
            url.ends_with(".tar")
                || url.ends_with(".tar.gz")
                || url.ends_with(".tar.bz2")
                || url.ends_with(".tgz")
        } else {
            false
        }
    }

    /// Get file extension from the href.
    pub fn get_file_extension(&self) -> Option<String> {
        if let Some(url) = &self.href
            && let Some(filename) = url.split('/').next_back()
            && let Some(dot_index) = filename.rfind('.')
        {
            return Some(filename[dot_index + 1..].to_lowercase());
        }
        None
    }

    /// Check if this is an archive file (zip, tar, etc.).
    pub fn is_archive(&self) -> bool {
        if let Some(ext) = self.get_file_extension() {
            matches!(
                ext.as_str(),
                "zip" | "tar" | "gz" | "bz2" | "tgz" | "rar" | "7z"
            )
        } else {
            false
        }
    }
}

// ============================================================================
// Text mining support types
// ============================================================================

/// Term definition.
///
/// Maps to JATS `<def-list>/<def-item>`. Used for abbreviation lists and
/// glossaries commonly found in biomedical articles.
///
/// Example XML:
/// ```xml
/// <def-item>
///   <term>HPV</term>
///   <def><p>Human papillomavirus</p></def>
/// </def-item>
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Definition {
    /// Term being defined. From `<term>`.
    pub term: String,
    /// Definition text. From `<def>/<p>`.
    pub definition: String,
}

// ============================================================================
// Aggregate accessors
// ============================================================================

impl PmcArticle {
    /// PMC ID of this article (aggregate identity).
    pub fn pmcid(&self) -> &PmcId {
        &self.front.article_meta.pmcid
    }

    /// PubMed ID, if present.
    pub fn pmid(&self) -> Option<&PubMedId> {
        self.front.article_meta.pmid.as_ref()
    }

    /// DOI, if present.
    pub fn doi(&self) -> Option<&str> {
        self.front.article_meta.doi.as_deref()
    }

    /// Article title, if present.
    pub fn title(&self) -> Option<&str> {
        self.front.article_meta.title_group.article_title.as_deref()
    }

    /// Article subtitle, if present.
    pub fn subtitle(&self) -> Option<&str> {
        self.front.article_meta.title_group.subtitle.as_deref()
    }

    /// Authors.
    pub fn authors(&self) -> &[Author] {
        &self.front.article_meta.authors
    }

    /// Journal metadata.
    pub fn journal(&self) -> &JournalMeta {
        &self.front.journal_meta
    }

    /// Publication dates.
    pub fn pub_dates(&self) -> &[PublicationDate] {
        &self.front.article_meta.pub_dates
    }

    /// Subject categories.
    pub fn categories(&self) -> &[String] {
        &self.front.article_meta.categories
    }

    /// Volume number, if present.
    pub fn volume(&self) -> Option<&str> {
        self.front.article_meta.volume.as_deref()
    }

    /// Issue number, if present.
    pub fn issue(&self) -> Option<&str> {
        self.front.article_meta.issue.as_deref()
    }

    /// Keywords.
    pub fn keywords(&self) -> &[String] {
        &self.front.article_meta.keywords
    }

    /// Funding information.
    pub fn funding(&self) -> &[FundingInfo] {
        &self.front.article_meta.funding
    }

    /// Publication history dates.
    pub fn history(&self) -> &[HistoryDate] {
        &self.front.article_meta.history
    }

    /// Text of the main abstract (first `<abstract>`), if present.
    pub fn abstract_text(&self) -> Option<&str> {
        self.front
            .article_meta
            .abstracts
            .first()
            .map(|a| a.text.as_str())
    }

    /// Copyright statement, if present.
    pub fn copyright(&self) -> Option<&str> {
        self.front
            .article_meta
            .permissions
            .as_ref()
            .and_then(|p| p.copyright_statement.as_deref())
    }

    /// License text, if present.
    pub fn license_text(&self) -> Option<&str> {
        self.front
            .article_meta
            .permissions
            .as_ref()
            .and_then(|p| p.license.as_ref())
            .and_then(|l| l.text.as_deref())
    }

    /// License URL, if present.
    pub fn license_url(&self) -> Option<&str> {
        self.front
            .article_meta
            .permissions
            .as_ref()
            .and_then(|p| p.license.as_ref())
            .and_then(|l| l.href.as_deref())
    }

    /// Body sections (empty slice when the article has no body).
    pub fn sections(&self) -> &[Section] {
        self.body.as_ref().map_or(&[], |b| b.sections.as_slice())
    }

    /// References (empty slice when the article has no back matter).
    pub fn references(&self) -> &[Reference] {
        self.back.as_ref().map_or(&[], |b| b.references.as_slice())
    }

    /// Acknowledgments, if present.
    pub fn acknowledgments(&self) -> Option<&str> {
        self.back
            .as_ref()
            .and_then(|b| b.acknowledgments.as_deref())
    }

    /// Conflict of interest statement, if present.
    pub fn conflict_of_interest(&self) -> Option<&str> {
        self.back
            .as_ref()
            .and_then(|b| b.conflict_of_interest.as_deref())
    }

    /// Get tar files from supplementary materials.
    pub fn get_tar_files(&self) -> Vec<&SupplementaryMaterial> {
        self.supplementary_materials
            .iter()
            .filter(|m| m.is_tar_file())
            .collect()
    }

    /// Get all archive files from supplementary materials.
    pub fn get_archive_files(&self) -> Vec<&SupplementaryMaterial> {
        self.supplementary_materials
            .iter()
            .filter(|m| m.is_archive())
            .collect()
    }

    /// Check if the article has supplementary materials.
    pub fn has_supplementary_materials(&self) -> bool {
        !self.supplementary_materials.is_empty()
    }

    /// Get supplementary materials by content type.
    pub fn get_supplementary_materials_by_type(
        &self,
        content_type: &str,
    ) -> Vec<&SupplementaryMaterial> {
        self.supplementary_materials
            .iter()
            .filter(|m| m.content_type.as_ref().is_some_and(|ct| ct == content_type))
            .collect()
    }
}

// ============================================================================
// Section classification
// ============================================================================

/// Semantic classification of a JATS `<sec sec-type="...">` value.
///
/// The DTD leaves `@sec-type` open-ended, but the NLM/PMC tagging guidelines use
/// a recognized vocabulary for the standard parts of a research article (IMRaD
/// and friends). This enum maps those well-known values to type-safe variants so
/// text-mining pipelines can match on section role without string juggling,
/// while preserving any unrecognized value via [`SectionKind::Other`].
///
/// Use [`Section::kind`] to obtain this from a section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionKind {
    /// `intro` / `introduction`.
    Introduction,
    /// `methods` / `materials` / `materials|methods` / `methods|materials`.
    Methods,
    /// `results`.
    Results,
    /// `results|discussion`.
    ResultsDiscussion,
    /// `discussion`.
    Discussion,
    /// `conclusions`.
    Conclusions,
    /// `abstract`.
    Abstract,
    /// `background`.
    Background,
    /// `cases` / `case` / `case-report`.
    CaseStudy,
    /// `supplementary-material`.
    SupplementaryMaterial,
    /// `data-availability`.
    DataAvailability,
    /// A `sec-type` that is present but not one of the recognized values.
    Other(String),
    /// No `sec-type` attribute was present on the section.
    Unspecified,
}

impl SectionKind {
    /// Classify a raw `sec-type` attribute value.
    ///
    /// Matching is case-insensitive and tolerant of the `|` separator that JATS
    /// uses for combined sections (e.g. `materials|methods`).
    pub fn from_sec_type(sec_type: Option<&str>) -> Self {
        let Some(raw) = sec_type else {
            return SectionKind::Unspecified;
        };
        let normalized = raw.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "intro" | "introduction" => SectionKind::Introduction,
            "methods" | "materials" | "methods|materials" | "materials|methods"
            | "subjects|methods" => SectionKind::Methods,
            "results" => SectionKind::Results,
            "results|discussion" | "discussion|results" => SectionKind::ResultsDiscussion,
            "discussion" => SectionKind::Discussion,
            "conclusions" | "conclusion" => SectionKind::Conclusions,
            "abstract" => SectionKind::Abstract,
            "background" => SectionKind::Background,
            "cases" | "case" | "case-report" | "case-study" => SectionKind::CaseStudy,
            "supplementary-material" => SectionKind::SupplementaryMaterial,
            "data-availability" | "availability" => SectionKind::DataAvailability,
            _ => SectionKind::Other(normalized),
        }
    }

    /// Whether this is one of the core IMRaD sections
    /// (introduction, methods, results, discussion, conclusions).
    pub fn is_imrad(&self) -> bool {
        matches!(
            self,
            SectionKind::Introduction
                | SectionKind::Methods
                | SectionKind::Results
                | SectionKind::ResultsDiscussion
                | SectionKind::Discussion
                | SectionKind::Conclusions
        )
    }
}

/// Depth-first, pre-order iterator over a section subtree.
///
/// Yields sections in document order: a section is yielded before its
/// subsections. Created by [`Section::iter_subtree`] and
/// [`PmcArticle::all_sections`].
pub struct SectionIter<'a> {
    stack: Vec<&'a Section>,
}

impl<'a> Iterator for SectionIter<'a> {
    type Item = &'a Section;

    fn next(&mut self) -> Option<&'a Section> {
        let section = self.stack.pop()?;
        // Push children in reverse so they are popped in document order.
        self.stack.extend(section.subsections.iter().rev());
        Some(section)
    }
}

impl Section {
    /// Semantic classification of this section's `sec-type`.
    pub fn kind(&self) -> SectionKind {
        SectionKind::from_sec_type(self.section_type.as_deref())
    }

    /// Iterate this section and all of its nested subsections, depth-first in
    /// document order (pre-order: parent before children).
    pub fn iter_subtree(&self) -> SectionIter<'_> {
        SectionIter { stack: vec![self] }
    }

    /// All figures in this section and its subsections (recursive).
    pub fn all_figures(&self) -> Vec<&Figure> {
        self.iter_subtree().flat_map(|s| s.figures.iter()).collect()
    }

    /// All tables in this section and its subsections (recursive).
    pub fn all_tables(&self) -> Vec<&Table> {
        self.iter_subtree().flat_map(|s| s.tables.iter()).collect()
    }

    /// All display formulas in this section and its subsections (recursive).
    pub fn all_formulas(&self) -> Vec<&Formula> {
        self.iter_subtree()
            .flat_map(|s| s.formulas.iter())
            .collect()
    }

    /// Whitespace-delimited word count of this section's own `content`
    /// (not including subsections). Useful for readability/length metrics.
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }

    /// Whether this section carries no text and no child structure.
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
            && self.subsections.is_empty()
            && self.figures.is_empty()
            && self.tables.is_empty()
            && self.formulas.is_empty()
    }
}

impl Abstract {
    /// Whether this abstract has labeled/structured sections.
    pub fn is_structured(&self) -> bool {
        !self.sections.is_empty()
    }

    /// Find a structured abstract section by label (case-insensitive match).
    ///
    /// Returns the first section whose label equals `label` ignoring case, e.g.
    /// `abstract.section_by_label("methods")`.
    pub fn section_by_label(&self, label: &str) -> Option<&AbstractSection> {
        self.sections.iter().find(|s| {
            s.label
                .as_ref()
                .is_some_and(|l| l.eq_ignore_ascii_case(label))
        })
    }
}

impl Table {
    /// Total number of rows (header rows plus body rows).
    pub fn row_count(&self) -> usize {
        self.head.len() + self.body.len()
    }

    /// Iterate all rows, header rows first then body rows.
    pub fn rows(&self) -> impl Iterator<Item = &TableRow> {
        self.head.iter().chain(self.body.iter())
    }

    /// Number of columns, accounting for `colspan`. Computed as the maximum
    /// spanned cell count across all rows (0 for an empty table).
    pub fn column_count(&self) -> usize {
        self.rows()
            .map(|row| {
                row.cells
                    .iter()
                    .map(|c| c.colspan.unwrap_or(1).max(1) as usize)
                    .sum()
            })
            .max()
            .unwrap_or(0)
    }

    /// Whether the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.head.is_empty() && self.body.is_empty()
    }
}

impl Reference {
    /// Whether this reference carries a PubMed ID.
    pub fn has_pmid(&self) -> bool {
        self.pmid.is_some()
    }

    /// Whether this reference carries a DOI.
    pub fn has_doi(&self) -> bool {
        self.doi.is_some()
    }
}

// ============================================================================
// Article-wide text-mining accessors
// ============================================================================

impl PmcArticle {
    /// Iterate every body section in the article, flattened depth-first in
    /// document order (parent before children).
    ///
    /// This walks the entire `<body>` section tree so callers don't have to
    /// recurse through [`Section::subsections`] manually.
    pub fn all_sections(&self) -> SectionIter<'_> {
        SectionIter {
            stack: self.sections().iter().rev().collect(),
        }
    }

    /// All body sections whose semantic [`SectionKind`] matches `kind`
    /// (recursive). For example, `article.sections_of_kind(&SectionKind::Methods)`.
    pub fn sections_of_kind(&self, kind: &SectionKind) -> Vec<&Section> {
        self.all_sections().filter(|s| &s.kind() == kind).collect()
    }

    /// All body sections whose raw `sec-type` equals `sec_type`
    /// (case-insensitive, recursive).
    pub fn sections_by_type(&self, sec_type: &str) -> Vec<&Section> {
        self.all_sections()
            .filter(|s| {
                s.section_type
                    .as_ref()
                    .is_some_and(|t| t.eq_ignore_ascii_case(sec_type))
            })
            .collect()
    }

    /// First body section whose title contains `keyword` (case-insensitive,
    /// recursive). Handy for locating sections like "Statistical analysis".
    pub fn find_section_by_title(&self, keyword: &str) -> Option<&Section> {
        let needle = keyword.to_ascii_lowercase();
        self.all_sections().find(|s| {
            s.title
                .as_ref()
                .is_some_and(|t| t.to_ascii_lowercase().contains(&needle))
        })
    }

    /// All figures across the whole article body (recursive over sections).
    pub fn all_figures(&self) -> Vec<&Figure> {
        self.all_sections().flat_map(|s| s.figures.iter()).collect()
    }

    /// All tables across the whole article body (recursive over sections).
    pub fn all_tables(&self) -> Vec<&Table> {
        self.all_sections().flat_map(|s| s.tables.iter()).collect()
    }

    /// All display formulas across the whole article body (recursive).
    pub fn all_formulas(&self) -> Vec<&Formula> {
        self.all_sections()
            .flat_map(|s| s.formulas.iter())
            .collect()
    }

    /// Number of figures in the article body.
    pub fn figure_count(&self) -> usize {
        self.all_sections().map(|s| s.figures.len()).sum()
    }

    /// Number of tables in the article body.
    pub fn table_count(&self) -> usize {
        self.all_sections().map(|s| s.tables.len()).sum()
    }

    /// Concatenated plain text of the whole article body, in reading order.
    ///
    /// Section titles and paragraph content are joined with blank lines. Useful
    /// as a single input string for tokenizers, embeddings, or full-text search.
    pub fn body_text(&self) -> String {
        let mut out = String::new();
        for section in self.all_sections() {
            if let Some(title) = &section.title
                && !title.trim().is_empty()
            {
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str(title);
            }
            if !section.content.trim().is_empty() {
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str(&section.content);
            }
        }
        out
    }

    /// All abstracts (a JATS article may have several, e.g. a main and a
    /// graphical abstract).
    pub fn abstracts(&self) -> &[Abstract] {
        &self.front.article_meta.abstracts
    }

    /// Structured sections of the main (first) abstract, if it is structured.
    pub fn abstract_sections(&self) -> &[AbstractSection] {
        self.front
            .article_meta
            .abstracts
            .first()
            .map_or(&[], |a| a.sections.as_slice())
    }

    /// References that carry a PubMed ID (useful for building citation graphs).
    pub fn references_with_pmid(&self) -> Vec<&Reference> {
        self.references().iter().filter(|r| r.has_pmid()).collect()
    }

    /// References that carry a DOI.
    pub fn references_with_doi(&self) -> Vec<&Reference> {
        self.references().iter().filter(|r| r.has_doi()).collect()
    }

    /// References whose `publication_type` equals `pub_type` (case-insensitive).
    pub fn references_by_type(&self, pub_type: &str) -> Vec<&Reference> {
        self.references()
            .iter()
            .filter(|r| {
                r.publication_type
                    .as_ref()
                    .is_some_and(|t| t.eq_ignore_ascii_case(pub_type))
            })
            .collect()
    }

    /// Appendices (empty slice when the article has no back matter).
    pub fn appendices(&self) -> &[Section] {
        self.back.as_ref().map_or(&[], |b| b.appendices.as_slice())
    }

    /// Glossary / abbreviation definitions (empty slice when absent).
    pub fn glossary(&self) -> &[Definition] {
        self.back.as_ref().map_or(&[], |b| b.glossary.as_slice())
    }

    /// Supplementary materials attached to the article.
    pub fn supplementary_materials(&self) -> &[SupplementaryMaterial] {
        &self.supplementary_materials
    }

    /// Data availability statement, if present.
    pub fn data_availability(&self) -> Option<&str> {
        self.data_availability.as_deref()
    }

    /// Keyword groups (structured, with `kwd-group-type` / language).
    pub fn keyword_groups(&self) -> &[KeywordGroup] {
        &self.front.article_meta.keyword_groups
    }

    /// Subject groups (structured, with `subj-group-type`).
    pub fn subject_groups(&self) -> &[SubjectGroup] {
        &self.front.article_meta.subject_groups
    }

    /// Related articles (corrections, retractions, companions, …).
    pub fn related_articles(&self) -> &[RelatedArticle] {
        &self.front.article_meta.related_articles
    }

    /// Articles corrected by this one (`related-article-type="corrected-article"`).
    pub fn corrections(&self) -> Vec<&RelatedArticle> {
        self.related_articles()
            .iter()
            .filter(|r| r.is_correction())
            .collect()
    }

    /// Articles retracted by this one (`related-article-type="retracted-article"`).
    pub fn retractions(&self) -> Vec<&RelatedArticle> {
        self.related_articles()
            .iter()
            .filter(|r| r.is_retraction())
            .collect()
    }

    /// Author notes (correspondence / contribution / present-address statements).
    pub fn author_notes(&self) -> &[String] {
        &self.front.article_meta.author_notes
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(section_type: Option<&str>, title: Option<&str>, content: &str) -> Section {
        Section {
            id: None,
            section_type: section_type.map(String::from),
            label: None,
            title: title.map(String::from),
            content: content.to_string(),
            subsections: Vec::new(),
            figures: Vec::new(),
            tables: Vec::new(),
            formulas: Vec::new(),
        }
    }

    fn fig(id: &str) -> Figure {
        Figure {
            id: id.to_string(),
            label: None,
            caption: None,
            alt_text: None,
            fig_type: None,
            graphic_href: None,
        }
    }

    #[test]
    fn test_section_kind_classification() {
        assert_eq!(
            SectionKind::from_sec_type(Some("intro")),
            SectionKind::Introduction
        );
        assert_eq!(
            SectionKind::from_sec_type(Some("Methods")),
            SectionKind::Methods
        );
        assert_eq!(
            SectionKind::from_sec_type(Some("materials|methods")),
            SectionKind::Methods
        );
        assert_eq!(
            SectionKind::from_sec_type(Some("results|discussion")),
            SectionKind::ResultsDiscussion
        );
        assert_eq!(SectionKind::from_sec_type(None), SectionKind::Unspecified);
        assert_eq!(
            SectionKind::from_sec_type(Some("custom-thing")),
            SectionKind::Other("custom-thing".to_string())
        );
        assert!(SectionKind::Methods.is_imrad());
        assert!(!SectionKind::SupplementaryMaterial.is_imrad());
    }

    #[test]
    fn test_section_subtree_iteration_is_preorder() {
        // root -> [a -> [a1], b]
        let mut root = sec(Some("intro"), Some("Root"), "r");
        let mut a = sec(None, Some("A"), "a");
        a.subsections.push(sec(None, Some("A1"), "a1"));
        let b = sec(None, Some("B"), "b");
        root.subsections.push(a);
        root.subsections.push(b);

        let titles: Vec<&str> = root
            .iter_subtree()
            .filter_map(|s| s.title.as_deref())
            .collect();
        assert_eq!(titles, vec!["Root", "A", "A1", "B"]);
    }

    #[test]
    fn test_section_recursive_figures() {
        let mut root = sec(None, None, "");
        root.figures.push(fig("f1"));
        let mut child = sec(None, None, "");
        child.figures.push(fig("f2"));
        root.subsections.push(child);

        let ids: Vec<&str> = root.all_figures().iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, vec!["f1", "f2"]);
        assert_eq!(root.word_count(), 0);
    }

    #[test]
    fn test_article_text_mining_accessors() {
        let mut intro = sec(Some("intro"), Some("Introduction"), "Hello world.");
        intro.figures.push(fig("f1"));
        let mut methods = sec(
            Some("methods"),
            Some("Statistical analysis"),
            "We used t-tests.",
        );
        let mut sub = sec(None, Some("Sub"), "Nested.");
        sub.tables.push(Table {
            id: "t1".to_string(),
            label: None,
            caption: None,
            head: vec![TableRow {
                cells: vec![TableCell {
                    content: "h1".into(),
                    is_header: true,
                    colspan: Some(2),
                    rowspan: None,
                }],
            }],
            body: vec![TableRow {
                cells: vec![
                    TableCell {
                        content: "a".into(),
                        is_header: false,
                        colspan: None,
                        rowspan: None,
                    },
                    TableCell {
                        content: "b".into(),
                        is_header: false,
                        colspan: None,
                        rowspan: None,
                    },
                ],
            }],
            footnotes: Vec::new(),
        });
        methods.subsections.push(sub);

        let article = PmcArticle {
            article_type: None,
            front: Front {
                journal_meta: JournalMeta {
                    title: None,
                    abbreviation: None,
                    issn_print: None,
                    issn_electronic: None,
                    publisher: None,
                },
                article_meta: ArticleMeta {
                    pmcid: PmcId::from_u32(123),
                    pmid: None,
                    doi: None,
                    categories: Vec::new(),
                    title_group: TitleGroup {
                        article_title: None,
                        subtitle: None,
                    },
                    authors: Vec::new(),
                    pub_dates: Vec::new(),
                    volume: None,
                    issue: None,
                    fpage: None,
                    lpage: None,
                    elocation_id: None,
                    history: Vec::new(),
                    permissions: None,
                    abstracts: vec![Abstract {
                        abstract_type: None,
                        text: "Full abstract.".to_string(),
                        sections: vec![
                            AbstractSection {
                                label: Some("Background".into()),
                                text: "bg".into(),
                            },
                            AbstractSection {
                                label: Some("Methods".into()),
                                text: "m".into(),
                            },
                        ],
                    }],
                    keywords: Vec::new(),
                    keyword_groups: Vec::new(),
                    subject_groups: Vec::new(),
                    related_articles: Vec::new(),
                    author_notes: Vec::new(),
                    funding: Vec::new(),
                },
            },
            body: Some(Body {
                sections: vec![intro, methods],
            }),
            back: Some(Back {
                acknowledgments: None,
                conflict_of_interest: None,
                references: vec![
                    Reference {
                        id: "r1".into(),
                        publication_type: Some("journal".into()),
                        title: None,
                        authors: Vec::new(),
                        source: None,
                        year: None,
                        volume: None,
                        issue: None,
                        pages: None,
                        elocation_id: None,
                        editors: Vec::new(),
                        publisher_name: None,
                        publisher_loc: None,
                        edition: None,
                        isbn: None,
                        conf_name: None,
                        pmid: Some("111".into()),
                        doi: None,
                    },
                    Reference {
                        id: "r2".into(),
                        publication_type: Some("book".into()),
                        title: None,
                        authors: Vec::new(),
                        source: None,
                        year: None,
                        volume: None,
                        issue: None,
                        pages: None,
                        elocation_id: None,
                        editors: Vec::new(),
                        publisher_name: None,
                        publisher_loc: None,
                        edition: None,
                        isbn: None,
                        conf_name: None,
                        pmid: None,
                        doi: Some("10.1/x".into()),
                    },
                ],
                appendices: Vec::new(),
                glossary: Vec::new(),
            }),
            supplementary_materials: Vec::new(),
            data_availability: None,
        };

        // Recursive section walk reaches nested subsection.
        assert_eq!(article.all_sections().count(), 3);
        // Filter by semantic kind and raw type.
        assert_eq!(article.sections_of_kind(&SectionKind::Methods).len(), 1);
        assert_eq!(article.sections_by_type("intro").len(), 1);
        // Title search is recursive + case-insensitive substring.
        assert!(article.find_section_by_title("statistical").is_some());
        // Recursive figure/table discovery.
        assert_eq!(article.figure_count(), 1);
        assert_eq!(article.table_count(), 1);
        // Body text joins titles and content.
        let body = article.body_text();
        assert!(body.contains("Hello world."));
        assert!(body.contains("We used t-tests."));
        assert!(body.contains("Nested."));
        // Abstract section lookup.
        let abs = &article.abstracts()[0];
        assert!(abs.is_structured());
        assert_eq!(abs.section_by_label("methods").unwrap().text, "m");
        assert_eq!(article.abstract_sections().len(), 2);
        // Reference filters.
        assert_eq!(article.references_with_pmid().len(), 1);
        assert_eq!(article.references_with_doi().len(), 1);
        assert_eq!(article.references_by_type("JOURNAL").len(), 1);
        // Table geometry helpers.
        let table = article.all_tables()[0];
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 2);
        assert!(!table.is_empty());
    }
}
