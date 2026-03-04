//! PMC domain models based on JATS Archiving 1.4 DTD
//!
//! These types represent the domain of PMC full-text articles with clean
//! separation from parsing and extraction concerns. Each type maps to
//! specific JATS XML elements as documented in the field comments.
//!
//! Design principles:
//! - DTD-faithful: reflects JATS Archiving 1.4 element structure
//! - No extraction concerns: fields like `file_path` or inferred `file_type` are excluded
//! - Type-safe IDs: uses `PmcId` / `PubMedId` instead of raw strings
//! - Reuses shared types: `Author` and `Affiliation` from `common::models`
//! - Text mining ready: structured abstracts, table content, formulas, definitions

use crate::common::{Author, PmcId, PubMedId};
use crate::error::ParseError;
use crate::pmc::parser::models;
use serde::{Deserialize, Serialize};

// ============================================================================
// Top-level article
// ============================================================================

/// PMC full-text article.
///
/// Maps to JATS `<article>`. Organized following the DTD's front/body/back structure,
/// with identifiers and metadata from `<article-meta>`, content from `<body>`,
/// and references/acknowledgments from `<back>`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PmcArticle {
    // --- Identifiers (<article-meta>/<article-id>) ---
    /// PMC ID (e.g., PMC7906746). From `<article-id pub-id-type="pmc">`.
    pub pmcid: PmcId,
    /// PubMed ID. From `<article-id pub-id-type="pmid">`.
    pub pmid: Option<PubMedId>,
    /// DOI. From `<article-id pub-id-type="doi">`.
    pub doi: Option<String>,

    // --- Article metadata ---
    /// Article type. From `<article article-type="...">` attribute.
    pub article_type: Option<String>,
    /// Subject categories. From `<article-categories>/<subj-group>/<subject>`.
    pub categories: Vec<String>,

    // --- Title (<title-group>) ---
    /// Article title. From `<article-title>`.
    pub title: String,
    /// Article subtitle. From `<subtitle>`.
    pub subtitle: Option<String>,

    // --- Contributors (<contrib-group>) ---
    /// Authors. From `<contrib-group>/<contrib contrib-type="author">`.
    pub authors: Vec<Author>,

    // --- Journal metadata (<journal-meta>) ---
    pub journal: JournalMeta,

    // --- Publication info (<article-meta>) ---
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

    // --- Abstract (<abstract>) ---
    /// Plain abstract text (flattened). From `<abstract>` without `<sec>` children,
    /// or concatenation of all section texts.
    pub abstract_text: Option<String>,
    /// Structured abstract sections. From `<abstract>/<sec>`.
    /// Present when the abstract has labeled sections (e.g., Background, Methods, Results).
    pub abstract_sections: Vec<AbstractSection>,

    // --- Keywords ---
    /// Keywords. From `<kwd-group>/<kwd>`.
    pub keywords: Vec<String>,

    // --- Content (<body>) ---
    /// Article sections. From `<body>/<sec>`.
    pub sections: Vec<Section>,

    // --- References (<back>/<ref-list>) ---
    /// Reference list. From `<ref-list>/<ref>`.
    pub references: Vec<Reference>,

    // --- Funding (<funding-group>) ---
    /// Funding information. From `<funding-group>/<award-group>`.
    pub funding: Vec<FundingInfo>,

    // --- Back matter (<back>) ---
    /// Acknowledgments. From `<back>/<ack>`.
    pub acknowledgments: Option<String>,
    /// Conflict of interest statement. From `<fn fn-type="COI-statement">`
    /// or `<sec sec-type="COI-statement">`.
    pub conflict_of_interest: Option<String>,
    /// Data availability statement. From `<sec sec-type="data-availability">`
    /// or `<notes notes-type="data-availability">`.
    pub data_availability: Option<String>,
    /// Supplementary materials. From `<supplementary-material>`.
    pub supplementary_materials: Vec<SupplementaryMaterial>,
    /// Appendices. From `<back>/<app-group>/<app>`.
    pub appendices: Vec<Section>,
    /// Glossary definitions. From `<back>/<glossary>/<def-list>`.
    pub glossary: Vec<Definition>,

    // --- Legal (<permissions>) ---
    /// Copyright statement. From `<copyright-statement>`.
    pub copyright: Option<String>,
    /// License text. From `<license>` body content.
    pub license: Option<String>,
    /// License URL. From `<license xlink:href="...">`.
    pub license_url: Option<String>,

    // --- History (<history>) ---
    /// Publication history dates. From `<history>/<date>`.
    pub history_dates: Vec<HistoryDate>,
}

// ============================================================================
// Front matter types
// ============================================================================

/// Journal metadata.
///
/// Maps to JATS `<journal-meta>`. Note that `volume` and `issue` are intentionally
/// excluded here — in the DTD they belong to `<article-meta>`, not `<journal-meta>`,
/// and are placed on [`PmcArticle`] accordingly.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalMeta {
    /// Journal title. From `<journal-title>`.
    pub title: String,
    /// Abbreviated journal title. From `<abbrev-journal-title>`.
    pub abbreviation: Option<String>,
    /// Print ISSN. From `<issn pub-type="ppub">`.
    pub issn_print: Option<String>,
    /// Electronic ISSN. From `<issn pub-type="epub">`.
    pub issn_electronic: Option<String>,
    /// Publisher name. From `<publisher>/<publisher-name>`.
    pub publisher: Option<String>,
}

/// Structured publication date.
///
/// Maps to JATS `<pub-date>`. A single article can have multiple publication dates
/// distinguished by `pub_type` (e.g., electronic vs. print vs. collection).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicationDate {
    /// Publication type. From `<pub-date pub-type="...">` or `<pub-date date-type="...">`.
    /// Common values: `"epub"`, `"ppub"`, `"collection"`.
    pub pub_type: Option<String>,
    /// Year. From `<year>`.
    pub year: Option<u16>,
    /// Month (1-12). From `<month>`.
    pub month: Option<u8>,
    /// Day (1-31). From `<day>`.
    pub day: Option<u8>,
}

/// Structured abstract section.
///
/// Maps to `<abstract>/<sec>`. Many biomedical journals use structured abstracts
/// with labeled sections (Background, Methods, Results, Conclusions).
/// This structure preserves the section boundaries for text mining pipelines
/// that need to process abstract sections independently.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AbstractSection {
    /// Section label (e.g., "Background", "Methods", "Results", "Conclusions").
    /// From `<title>` inside `<abstract>/<sec>`.
    pub label: Option<String>,
    /// Section text content. From `<p>` inside `<abstract>/<sec>`.
    pub text: String,
}

// ============================================================================
// Body content types
// ============================================================================

/// Article section.
///
/// Maps to JATS `<sec>`. Sections form a recursive tree via `subsections`.
/// Figures, tables, and formulas that appear inline within the section are
/// collected in their respective fields.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Figure {
    /// Figure ID. From `<fig id="...">`.
    pub id: String,
    /// Figure label (e.g., "Figure 1"). From `<label>`.
    pub label: Option<String>,
    /// Figure caption. From `<caption>/<p>`.
    pub caption: String,
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Table {
    /// Table ID. From `<table-wrap id="...">`.
    pub id: String,
    /// Table label (e.g., "Table 1"). From `<label>`.
    pub label: Option<String>,
    /// Table caption. From `<caption>/<p>`.
    pub caption: String,
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TableRow {
    /// Cells in this row. From `<th>` or `<td>` elements.
    pub cells: Vec<TableCell>,
}

/// A single table cell.
///
/// Maps to XHTML `<th>` or `<td>` inside JATS `<table>`.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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
// Back matter types
// ============================================================================

/// Reference citation.
///
/// Maps to JATS `<ref>` containing `<element-citation>` or `<mixed-citation>`.
#[derive(Debug, Serialize, Deserialize, Clone)]
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

/// Funding information.
///
/// Maps to JATS `<funding-group>/<award-group>`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FundingInfo {
    /// Funding source/agency. From `<funding-source>`.
    pub source: String,
    /// Grant/award ID. From `<award-id>`.
    pub award_id: Option<String>,
    /// Funding statement. From `<funding-statement>`.
    pub statement: Option<String>,
}

/// Publication history date.
///
/// Maps to JATS `<history>/<date>`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryDate {
    /// Date type. From `<date date-type="...">`.
    /// Common values: `"received"`, `"accepted"`, `"rev-recd"`.
    pub date_type: String,
    /// Year. From `<year>`.
    pub year: Option<u16>,
    /// Month (1-12). From `<month>`.
    pub month: Option<u8>,
    /// Day (1-31). From `<day>`.
    pub day: Option<u8>,
}

/// Supplementary material.
///
/// Maps to JATS `<supplementary-material>`. Only contains domain-level
/// data from the XML. Inferred values like file type (derived from URL extension)
/// and layout attributes like `position` are excluded.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Definition {
    /// Term being defined. From `<term>`.
    pub term: String,
    /// Definition text. From `<def>/<p>`.
    pub definition: String,
}

// ============================================================================
// Conversions from legacy models (models::*) → domain types
// ============================================================================

impl TryFrom<models::PmcFullText> for PmcArticle {
    type Error = ParseError;

    fn try_from(old: models::PmcFullText) -> Result<Self, Self::Error> {
        let pmcid = PmcId::parse(&old.pmcid)?;
        let pmid = old.pmid.as_deref().map(PubMedId::parse).transpose()?;

        // Parse pub_date string ("2023-12-25", "2023-12", "2023", "Unknown Date")
        // into a single PublicationDate. Lossy — original XML may have had multiple
        // <pub-date> elements with pub-type attributes, but the old parser flattens them.
        let pub_dates = parse_pub_date_string(&old.pub_date);

        // volume/issue move from JournalInfo to article level
        let volume = old.journal.volume.clone();
        let issue = old.journal.issue.clone();

        Ok(PmcArticle {
            pmcid,
            pmid,
            doi: old.doi,
            article_type: old.article_type,
            categories: old.categories,
            title: old.title,
            subtitle: None, // Not extracted by current parser
            authors: old.authors,
            journal: JournalMeta::from(old.journal),
            pub_dates,
            volume,
            issue,
            fpage: old.fpage,
            lpage: old.lpage,
            elocation_id: old.elocation_id,
            abstract_text: old.abstract_text,
            abstract_sections: Vec::new(), // Not extracted by current parser
            keywords: old.keywords,
            sections: old.sections.into_iter().map(Section::from).collect(),
            references: old.references.into_iter().map(Reference::from).collect(),
            funding: old.funding.into_iter().map(FundingInfo::from).collect(),
            acknowledgments: old.acknowledgments,
            conflict_of_interest: old.conflict_of_interest,
            data_availability: old.data_availability,
            supplementary_materials: old
                .supplementary_materials
                .into_iter()
                .map(SupplementaryMaterial::from)
                .collect(),
            appendices: Vec::new(), // Not extracted by current parser
            glossary: Vec::new(),   // Not extracted by current parser
            copyright: old.copyright,
            license: old.license,
            license_url: old.license_url,
            history_dates: old
                .history_dates
                .into_iter()
                .map(HistoryDate::from)
                .collect(),
        })
    }
}

impl From<models::JournalInfo> for JournalMeta {
    fn from(old: models::JournalInfo) -> Self {
        Self {
            title: old.title,
            abbreviation: old.abbreviation,
            issn_print: old.issn_print,
            issn_electronic: old.issn_electronic,
            publisher: old.publisher,
            // volume/issue intentionally omitted — moved to PmcArticle
        }
    }
}

impl From<models::ArticleSection> for Section {
    fn from(old: models::ArticleSection) -> Self {
        let section_type = if old.section_type.is_empty() {
            None
        } else {
            Some(old.section_type)
        };

        Self {
            id: old.id,
            section_type,
            label: None, // Not extracted by current parser
            title: old.title,
            content: old.content,
            subsections: old.subsections.into_iter().map(Section::from).collect(),
            figures: old.figures.into_iter().map(Figure::from).collect(),
            tables: old.tables.into_iter().map(Table::from).collect(),
            formulas: Vec::new(), // Not extracted by current parser
        }
    }
}

impl From<models::Figure> for Figure {
    fn from(old: models::Figure) -> Self {
        Self {
            id: old.id,
            label: old.label,
            caption: old.caption,
            alt_text: old.alt_text,
            fig_type: old.fig_type,
            graphic_href: old.file_name, // file_name was actually <graphic xlink:href>
                                         // file_path dropped — extraction concern
        }
    }
}

impl From<models::Table> for Table {
    fn from(old: models::Table) -> Self {
        Self {
            id: old.id,
            label: old.label,
            caption: old.caption,
            head: Vec::new(), // Not extracted by current parser
            body: Vec::new(), // Not extracted by current parser
            footnotes: old.footnotes,
        }
    }
}

impl From<models::Reference> for Reference {
    fn from(old: models::Reference) -> Self {
        Self {
            id: old.id,
            publication_type: old.ref_type, // renamed
            title: old.title,
            authors: old.authors,
            source: old.journal, // renamed: journal → source
            year: old.year,
            volume: old.volume,
            issue: old.issue,
            pages: old.pages,
            pmid: old.pmid,
            doi: old.doi,
        }
    }
}

impl From<models::FundingInfo> for FundingInfo {
    fn from(old: models::FundingInfo) -> Self {
        Self {
            source: old.source,
            award_id: old.award_id,
            statement: old.statement,
        }
    }
}

impl From<models::HistoryDate> for HistoryDate {
    fn from(old: models::HistoryDate) -> Self {
        Self {
            date_type: old.date_type,
            year: old.year,
            month: old.month,
            day: old.day,
        }
    }
}

impl From<models::SupplementaryMaterial> for SupplementaryMaterial {
    fn from(old: models::SupplementaryMaterial) -> Self {
        Self {
            id: old.id,
            content_type: old.content_type,
            title: old.title,
            description: old.description,
            href: old.file_url, // renamed: file_url → href
                                // file_type dropped — inferred value
                                // position dropped — layout attribute
        }
    }
}

/// Parse the legacy pub_date string into a `Vec<PublicationDate>`.
///
/// The old parser formats dates as "YYYY-MM-DD", "YYYY-MM", "YYYY",
/// or "Unknown Date". This conversion is lossy — the original XML may
/// have had multiple `<pub-date>` elements with `pub-type` attributes.
fn parse_pub_date_string(s: &str) -> Vec<PublicationDate> {
    if s == "Unknown Date" || s.is_empty() {
        return Vec::new();
    }

    let parts: Vec<&str> = s.split('-').collect();
    let year = parts.first().and_then(|y| y.parse::<u16>().ok());
    let month = parts.get(1).and_then(|m| m.parse::<u8>().ok());
    let day = parts.get(2).and_then(|d| d.parse::<u8>().ok());

    vec![PublicationDate {
        pub_type: None, // Lost in the old parser's string formatting
        year,
        month,
        day,
    }]
}
