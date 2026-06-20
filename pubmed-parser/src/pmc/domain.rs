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

    /// Keywords. From `<kwd-group>/<kwd>`.
    pub keywords: Vec<String>,

    /// Funding information. From `<funding-group>/<award-group>`.
    pub funding: Vec<FundingInfo>,
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
