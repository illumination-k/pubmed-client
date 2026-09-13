//! Structured output types shared by the MCP tools.
//!
//! Every tool answers with [`rmcp::handler::server::wrapper::Json`], so rmcp
//! advertises the wrapped type's JSON schema as the tool's `outputSchema` and
//! puts the serialized value in the response's `structuredContent` (the same
//! JSON is echoed as a text content block for clients that only read
//! `content`).
//!
//! These shapes are defined here rather than reused from `pubmed-parser`
//! because the wire format is a public contract of the server: the domain
//! models are free to gain fields, and a tool must be able to omit what the
//! caller opted out of.

use pubmed_client::{Figure, Reference, Section, Table};
use rmcp::schemars;
use serde::Serialize;

/// A JATS body section, with its subsections nested as in the source XML.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct SectionOut {
    /// Section id, from `<sec id="...">`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Section type, from `<sec sec-type="...">` (e.g. "intro", "methods").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_type: Option<String>,
    /// Section number/label (e.g. "2.1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Section title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Concatenated paragraph text of this section (excluding subsections).
    pub content: String,
    /// Nested subsections, in document order.
    pub subsections: Vec<SectionOut>,
}

impl From<&Section> for SectionOut {
    fn from(section: &Section) -> Self {
        Self {
            id: section.id.clone(),
            section_type: section.section_type.clone(),
            label: section.label.clone(),
            title: section.title.clone(),
            content: section.content.clone(),
            subsections: sections_out(&section.subsections),
        }
    }
}

/// Convert a slice of parsed sections into their output shape.
pub fn sections_out(sections: &[Section]) -> Vec<SectionOut> {
    sections.iter().map(SectionOut::from).collect()
}

/// Figure metadata. The figure itself is referenced by `graphic_href`, never
/// inlined.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct FigureOut {
    /// Figure id, from `<fig id="...">`.
    pub id: String,
    /// Figure label (e.g. "Figure 1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Figure caption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Alt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_text: Option<String>,
    /// Figure type (e.g. "figure", "scheme", "chart").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fig_type: Option<String>,
    /// Graphic file reference, from `<graphic xlink:href="...">`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphic_href: Option<String>,
}

impl From<&Figure> for FigureOut {
    fn from(figure: &Figure) -> Self {
        Self {
            id: figure.id.clone(),
            label: figure.label.clone(),
            caption: figure.caption.clone(),
            alt_text: figure.alt_text.clone(),
            fig_type: figure.fig_type.clone(),
            graphic_href: figure.graphic_href.clone(),
        }
    }
}

/// Table metadata. Row data is deliberately left out: a full table body is
/// large, and these tools exist to describe an article's structure.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct TableOut {
    /// Table id, from `<table-wrap id="...">`.
    pub id: String,
    /// Table label (e.g. "Table 1").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Table caption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Number of header rows.
    pub head_row_count: usize,
    /// Number of body rows.
    pub body_row_count: usize,
}

impl From<&Table> for TableOut {
    fn from(table: &Table) -> Self {
        Self {
            id: table.id.clone(),
            label: table.label.clone(),
            caption: table.caption.clone(),
            head_row_count: table.head.len(),
            body_row_count: table.body.len(),
        }
    }
}

/// A bibliographic reference from an article's back matter.
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct ReferenceOut {
    /// Reference id, from `<ref id="...">`; body sections cite it by this id.
    pub id: String,
    /// Human-readable citation, assembled from the structured fields below.
    pub citation: String,
    /// Cited work's title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Author names, in document order.
    pub authors: Vec<String>,
    /// Journal name or book title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Publication year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    /// Journal volume.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    /// Journal issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
    /// Page range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    /// PubMed ID of the cited work, when the publisher tagged one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmid: Option<String>,
    /// DOI of the cited work, when the publisher tagged one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
}

impl From<&Reference> for ReferenceOut {
    fn from(reference: &Reference) -> Self {
        Self {
            id: reference.id.clone(),
            citation: reference.format_citation(),
            title: reference.title.clone(),
            authors: reference
                .authors
                .iter()
                .map(|author| author.full_name.clone())
                .collect(),
            source: reference.source.clone(),
            year: reference.year.clone(),
            volume: reference.volume.clone(),
            issue: reference.issue.clone(),
            pages: reference.pages.clone(),
            pmid: reference.pmid.clone(),
            doi: reference.doi.clone(),
        }
    }
}
