//! Section-tree parsing for PMC/JATS full text.
//!
//! Split by JATS element group rather than by helper/driver layer:
//!
//! | module | elements |
//! | --- | --- |
//! | [`abstracts`] | `<abstract>` |
//! | [`body`] | `<body>`, `<sec>` |
//! | [`paragraph`] | `<p>` and its inline children (`<xref>`, `<fig>`, …) |
//! | [`figure`] | `<fig>` |
//! | [`table`] | `<table-wrap>` |
//!
//! This file keeps the entry point plus the two pieces the element modules
//! share: [`SectionAction`], the vocabulary the tag dispatchers emit, and
//! [`SectionParts`], the accumulator every section is built from.

mod abstracts;
mod body;
mod figure;
mod paragraph;
mod table;

use crate::pmc::domain::{Figure, Section, Table};
use crate::pmc::parser::reader_utils::{make_reader, read_text_content, skip_element};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;

use abstracts::extract_abstract_section;
use body::{extract_body_sections, parse_section_from_body};
use figure::{FigAttrs, extract_figures_from_content, parse_figure_inner};
use paragraph::read_paragraph_with_inline;
use table::{TableAttrs, parse_table_inner};

/// Extract all sections from PMC XML content
pub(crate) fn extract_sections_enhanced(content: &str) -> Vec<Section> {
    let mut sections = Vec::new();

    // Extract abstract first
    if let Some(abstract_section) = extract_abstract_section(content) {
        sections.push(abstract_section);
    }

    // Extract body sections with Reader-based parsing
    if let Some(body_start) = content.find("<body>")
        && let Some(body_end) = content[body_start..].find("</body>")
    {
        let body_content = &content[body_start + 6..body_start + body_end];
        sections.extend(extract_body_sections(body_content));
    }

    // Extract figures from floats-group and add to first section
    if let Some(floats_start) = content.find("<floats-group>")
        && let Some(floats_end) = content[floats_start..].find("</floats-group>")
    {
        let floats_content =
            &content[floats_start..floats_start + floats_end + "</floats-group>".len()];
        let float_figures = extract_figures_from_content(floats_content);
        if !float_figures.is_empty() {
            if let Some(first_section) = sections.first_mut() {
                first_section.figures.extend(float_figures);
            } else {
                sections.push(Section {
                    id: None,
                    section_type: Some("figures".to_string()),
                    label: None,
                    title: Some("Figures".to_string()),
                    content: String::new(),
                    subsections: Vec::new(),
                    figures: float_figures,
                    tables: Vec::new(),
                    formulas: Vec::new(),
                    cited_reference_ids: Vec::new(),
                });
            }
        }
    }

    sections
}

/// What to do with an element encountered inside a `<body>` or a `<sec>`.
///
/// Produced by the tag dispatchers in [`body`] and consumed by
/// [`SectionParts::apply`]. Attributes are carried in the variants because the
/// reader's borrow of the start event ends before the element body is parsed.
enum SectionAction {
    Continue,
    Break,
    ReadTitle,
    ReadParagraph,
    ReadSection(Option<String>),
    ReadFigure(FigAttrs),
    ReadTable(TableAttrs),
    /// Extract text content from a block-level element (list, def-list, formula, etc.)
    ReadTextElement(Vec<u8>),
    SkipTag(Vec<u8>),
}

/// Accumulators filled while parsing the children of a single `<sec>`.
///
/// Also used for the children of a `<body>` that never opens a `<sec>`, which
/// are collapsed into one synthetic section by [`SectionParts::into_body_section`].
#[derive(Default)]
struct SectionParts {
    title: Option<String>,
    content_parts: Vec<String>,
    subsections: Vec<Section>,
    figures: Vec<Figure>,
    tables: Vec<Table>,
    cited_reference_ids: Vec<String>,
}

impl SectionParts {
    /// Apply one classified action to the accumulators.
    /// Returns `true` when the enclosing `<sec>` is finished (Break/EOF).
    ///
    /// `ReadSection` recurses back into [`parse_section_from_body`], which
    /// builds a fresh `SectionParts` and calls this method again. The resulting
    /// `apply` ↔ `parse_section_from_body` cycle is the intended shape: it is
    /// how arbitrarily nested `<sec>` trees are consumed, not an accidental
    /// tangle. Recursion depth is bounded by the nesting depth of the document.
    fn apply(&mut self, action: SectionAction, reader: &mut Reader<&[u8]>) -> bool {
        match action {
            SectionAction::ReadTitle => {
                // `read_text_content` already returns trimmed text.
                if let Ok(t) = read_text_content(reader, b"title")
                    && !t.is_empty()
                {
                    self.title = Some(t);
                }
            }
            SectionAction::ReadParagraph => {
                let para = read_paragraph_with_inline(reader);
                if !para.text.is_empty() {
                    self.content_parts.push(para.text);
                }
                self.figures.extend(para.figures);
                self.tables.extend(para.tables);
                self.cited_reference_ids.extend(para.cited_reference_ids);
            }
            SectionAction::ReadSection(sub_id) => {
                // Recursive: properly handles nested sections
                if let Some(sub) = parse_section_from_body(reader, sub_id) {
                    self.subsections.push(sub);
                }
            }
            SectionAction::ReadFigure(attrs) => {
                if let Some(fig) = parse_figure_inner(reader, attrs) {
                    self.figures.push(fig);
                }
            }
            SectionAction::ReadTable(attrs) => {
                if let Some(table) = parse_table_inner(reader, attrs) {
                    self.tables.push(table);
                }
            }
            SectionAction::ReadTextElement(tag) => {
                if let Ok(text) = read_text_content(reader, &tag)
                    && !text.is_empty()
                {
                    self.content_parts.push(text);
                }
            }
            SectionAction::SkipTag(name) => {
                let _ = skip_element(reader, QName(&name));
            }
            SectionAction::Break => return true,
            SectionAction::Continue => {}
        }
        false
    }

    /// Build a `Section`, or `None` when it carries no content at all.
    fn into_section(self, id: Option<String>) -> Option<Section> {
        // Each part is already trimmed and non-empty, so the joined content has
        // no leading/trailing whitespace — no extra trim/allocation needed.
        let section_content = self.content_parts.join("\n");

        if section_content.is_empty()
            && self.subsections.is_empty()
            && self.figures.is_empty()
            && self.tables.is_empty()
        {
            return None;
        }

        Some(Section {
            id,
            section_type: Some("section".to_string()),
            label: None,
            title: self.title,
            content: section_content,
            subsections: self.subsections,
            figures: self.figures,
            tables: self.tables,
            formulas: Vec::new(),
            cited_reference_ids: self.cited_reference_ids,
        })
    }

    /// Build the synthetic `"body"` section for a `<body>` with no `<sec>`.
    ///
    /// Unlike [`SectionParts::into_section`] this requires text: a body holding
    /// only floating figures or tables is left to `<floats-group>` handling in
    /// [`extract_sections_enhanced`] rather than becoming a contentless section.
    fn into_body_section(self) -> Option<Section> {
        if self.content_parts.is_empty() {
            return None;
        }

        Some(Section {
            id: None,
            section_type: Some("body".to_string()),
            label: None,
            title: None,
            content: self.content_parts.join("\n"),
            subsections: Vec::new(),
            figures: self.figures,
            tables: self.tables,
            formulas: Vec::new(),
            cited_reference_ids: self.cited_reference_ids,
        })
    }
}

/// Scan `content` for every top-level `<tag>` element and parse each one.
///
/// Walks the whole content string regardless of nesting depth. `extract_attrs`
/// pulls the attributes off the start event (before the buffer is cleared) and
/// `parse_inner` consumes the element body. Shared by the figure and table
/// scanners, which differ only in the tag name, the attribute type, and the
/// inner parser.
fn scan_elements<A, T>(
    content: &str,
    tag: &[u8],
    extract_attrs: impl Fn(&BytesStart) -> A,
    parse_inner: impl Fn(&mut Reader<&[u8]>, A) -> Option<T>,
) -> Vec<T> {
    let mut results = Vec::new();
    let mut reader = make_reader(content);

    loop {
        let attrs = match reader.read_event() {
            Ok(Event::Start(ref e)) if e.name().as_ref() == tag => Some(extract_attrs(e)),
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };

        if let Some(attrs) = attrs
            && let Some(item) = parse_inner(&mut reader, attrs)
        {
            results.push(item);
        }
    }

    results
}
