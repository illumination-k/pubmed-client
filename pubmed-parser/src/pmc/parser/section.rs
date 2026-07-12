use crate::pmc::domain::{Figure, Section, Table};

use super::reader_utils::{get_attr, make_reader, read_text_content, skip_element, trim_in_place};
use quick_xml::events::Event;
use quick_xml::name::QName;
use tracing::warn;

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

/// Extract abstract section using Reader for text, Reader scan for figures/tables
fn extract_abstract_section(content: &str) -> Option<Section> {
    let abstract_start = content.find("<abstract")?;
    let abstract_end_offset = content[abstract_start..].find("</abstract>")?;
    let abstract_xml =
        &content[abstract_start..abstract_start + abstract_end_offset + "</abstract>".len()];

    // Extract text content using Reader
    let mut reader = make_reader(abstract_xml);
    let mut text_parts = Vec::new();
    let mut in_abstract = false;

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"abstract" => SectionAction::EnterAbstract,
                b"p" if in_abstract => SectionAction::ReadParagraph,
                b"title" if in_abstract => SectionAction::SkipTitle,
                _ => SectionAction::Continue,
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"abstract" => SectionAction::Break,
            Ok(Event::Eof) => SectionAction::Break,
            Err(_) => SectionAction::Break,
            _ => SectionAction::Continue,
        };

        match action {
            SectionAction::EnterAbstract => in_abstract = true,
            SectionAction::ReadParagraph => {
                if let Ok(text) = read_text_content(&mut reader, b"p") {
                    // `read_text_content` already returns trimmed text.
                    if !text.is_empty() {
                        text_parts.push(text);
                    }
                }
            }
            SectionAction::SkipTitle => {
                let _ = read_text_content(&mut reader, b"title");
            }
            SectionAction::Break => break,
            _ => {}
        }
    }

    // Extract figures and tables from the raw abstract content (handles inline figs)
    let figures = extract_figures_from_content(abstract_xml);
    let tables = extract_tables_from_content(abstract_xml);

    let clean_content = text_parts.join("\n");
    if !clean_content.is_empty() {
        Some(Section {
            id: None,
            section_type: Some("abstract".to_string()),
            label: None,
            title: Some("Abstract".to_string()),
            content: clean_content,
            subsections: Vec::new(),
            figures,
            tables,
            formulas: Vec::new(),
            // Abstract paragraph text is collected via `read_text_content`,
            // which does not track `<xref>` targets; abstracts rarely carry
            // bibliographic citations, so this is left empty by design.
            cited_reference_ids: Vec::new(),
        })
    } else {
        None
    }
}

/// Simple action enum to work around borrow checker (extract data from event before clearing buf)
enum SectionAction {
    Continue,
    Break,
    EnterAbstract,
    ReadParagraph,
    ReadSection(Option<String>),
    ReadBodyParagraph,
    ReadFigure(FigAttrs),
    ReadTable(TableAttrs),
    /// Extract text content from a block-level element (list, def-list, formula, etc.)
    ReadTextElement(Vec<u8>),
    SkipTitle,
    SkipTag(Vec<u8>),
}

/// Whether `tag` is a JATS block-level (`%para-level;`) element whose text we
/// extract inline rather than skipping. Shared by the body and `<sec>` parsers.
fn is_block_level(tag: &[u8]) -> bool {
    matches!(
        tag,
        b"list"
            | b"def-list"
            | b"disp-formula"
            | b"disp-formula-group"
            | b"disp-quote"
            | b"boxed-text"
            | b"code"
            | b"preformat"
            | b"media"
            | b"supplementary-material"
            | b"speech"
            | b"statement"
            | b"verse-group"
            | b"array"
            | b"graphic"
            | b"fn-group"
    )
}

/// Extract body sections using Reader with depth-aware `<sec>` parsing
fn extract_body_sections(content: &str) -> Vec<Section> {
    let mut reader = make_reader(content);
    let mut sections = Vec::new();
    let mut has_sec_tags = false;
    let mut body_paragraphs = Vec::new();
    let mut body_figures = Vec::new();
    let mut body_tables = Vec::new();
    let mut body_cited_refs = Vec::new();

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"sec" => SectionAction::ReadSection(get_attr(e, b"id")),
                b"p" if !has_sec_tags => SectionAction::ReadBodyParagraph,
                b"fig" if !has_sec_tags => SectionAction::ReadFigure(FigAttrs {
                    id: get_attr(e, b"id"),
                    fig_type: get_attr(e, b"fig-type"),
                }),
                b"table-wrap" if !has_sec_tags => SectionAction::ReadTable(TableAttrs {
                    id: get_attr(e, b"id"),
                }),
                // Block-level elements per JATS %para-level; — extract text in no-sec bodies
                other if !has_sec_tags && is_block_level(other) => {
                    SectionAction::ReadTextElement(other.to_vec())
                }
                _ => SectionAction::Continue,
            },
            Ok(Event::Eof) => SectionAction::Break,
            Err(_) => SectionAction::Break,
            _ => SectionAction::Continue,
        };

        match action {
            SectionAction::ReadSection(id) => {
                has_sec_tags = true;
                if let Some(section) = parse_section_from_body(&mut reader, id) {
                    sections.push(section);
                }
            }
            SectionAction::ReadBodyParagraph => {
                let (text, inline_figs, inline_tables, cited) =
                    read_paragraph_with_inline(&mut reader);
                if !text.is_empty() {
                    body_paragraphs.push(text);
                }
                body_figures.extend(inline_figs);
                body_tables.extend(inline_tables);
                body_cited_refs.extend(cited);
            }
            SectionAction::ReadFigure(attrs) => {
                if let Some(fig) = parse_figure_inner(&mut reader, attrs) {
                    body_figures.push(fig);
                }
            }
            SectionAction::ReadTable(attrs) => {
                if let Some(table) = parse_table_inner(&mut reader, attrs) {
                    body_tables.push(table);
                }
            }
            SectionAction::ReadTextElement(tag) => {
                if let Ok(text) = read_text_content(&mut reader, &tag) {
                    // `read_text_content` already returns trimmed text.
                    if !text.is_empty() {
                        body_paragraphs.push(text);
                    }
                }
            }
            SectionAction::Break => break,
            _ => {}
        }
    }

    // If no sections found, create a body section from paragraphs
    if sections.is_empty() && !body_paragraphs.is_empty() {
        let text = body_paragraphs.join("\n");
        sections.push(Section {
            id: None,
            section_type: Some("body".to_string()),
            label: None,
            title: None,
            content: text,
            subsections: Vec::new(),
            figures: body_figures,
            tables: body_tables,
            formulas: Vec::new(),
            cited_reference_ids: body_cited_refs,
        });
    }

    sections
}

/// Classify a start element encountered inside a `<sec>` into the action needed
/// to consume it. Keeps the tag dispatch out of the main parse loop.
fn classify_section_child(e: &quick_xml::events::BytesStart) -> SectionAction {
    match e.name().as_ref() {
        b"title" => SectionAction::SkipTitle,
        b"p" => SectionAction::ReadParagraph,
        b"sec" => SectionAction::ReadSection(get_attr(e, b"id")),
        b"fig" => SectionAction::ReadFigure(FigAttrs {
            id: get_attr(e, b"id"),
            fig_type: get_attr(e, b"fig-type"),
        }),
        b"table-wrap" => SectionAction::ReadTable(TableAttrs {
            id: get_attr(e, b"id"),
        }),
        // Block-level elements per JATS %para-level; — extract text instead of skipping
        other if is_block_level(other) => SectionAction::ReadTextElement(other.to_vec()),
        other => SectionAction::SkipTag(other.to_vec()),
    }
}

/// Accumulators filled while parsing the children of a single `<sec>`.
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
    fn apply(&mut self, action: SectionAction, reader: &mut quick_xml::Reader<&[u8]>) -> bool {
        match action {
            SectionAction::SkipTitle => {
                // `read_text_content` already returns trimmed text.
                if let Ok(t) = read_text_content(reader, b"title")
                    && !t.is_empty()
                {
                    self.title = Some(t);
                }
            }
            SectionAction::ReadParagraph => {
                let (text, inline_figs, inline_tables, cited) = read_paragraph_with_inline(reader);
                if !text.is_empty() {
                    self.content_parts.push(text);
                }
                self.figures.extend(inline_figs);
                self.tables.extend(inline_tables);
                self.cited_reference_ids.extend(cited);
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
            // Remaining variants never arise from `classify_section_child`.
            SectionAction::Continue
            | SectionAction::EnterAbstract
            | SectionAction::ReadBodyParagraph => {}
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
}

/// Parse a single `<sec>` element using Reader for structure.
/// The reader has just consumed `Event::Start` for `<sec>`.
///
/// Uses a single Reader pass for text, figures, tables, and subsections.
/// Figures and tables are detected both as direct children of `<sec>` and
/// inline within `<p>` tags via `read_paragraph_with_inline`.
fn parse_section_from_body(
    reader: &mut quick_xml::Reader<&[u8]>,
    id: Option<String>,
) -> Option<Section> {
    let mut parts = SectionParts::default();

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => classify_section_child(e),
            Ok(Event::End(ref e)) if e.name().as_ref() == b"sec" => SectionAction::Break,
            Ok(Event::Eof) => SectionAction::Break,
            Err(_) => SectionAction::Break,
            _ => SectionAction::Continue,
        };

        if parts.apply(action, reader) {
            break;
        }
    }

    parts.into_section(id)
}

/// Append the bibliographic reference targets of an `<xref>` start event to
/// `out`, in document order.
///
/// Only `<xref ref-type="bibr">` contributes — other cross-reference kinds
/// (`fig`, `table`, `disp-formula`, …) are ignored here. The `rid` attribute is
/// JATS `IDREFS`, so a grouped citation such as `rid="B1 B2 B3"` yields each id
/// separately. The `<xref>`'s visible text (the citation marker) is left in the
/// surrounding paragraph content untouched.
fn collect_bibr_rids(e: &quick_xml::events::BytesStart, out: &mut Vec<String>) {
    if get_attr(e, b"ref-type").as_deref() != Some("bibr") {
        return;
    }
    if let Some(rid) = get_attr(e, b"rid") {
        out.extend(rid.split_whitespace().map(str::to_string));
    }
}

/// Read a `<p>` element, collecting text while extracting inline figures and tables.
///
/// Uses Cow<str> from unescape() to avoid allocations when text has no XML entities.
/// Detects `<fig>` and `<table-wrap>` inside `<p>` and parses them as structured data.
/// Also records the `rid` targets of `<xref ref-type="bibr">` citations so callers
/// can link the paragraph to the references it cites.
fn read_paragraph_with_inline(
    reader: &mut quick_xml::Reader<&[u8]>,
) -> (String, Vec<Figure>, Vec<Table>, Vec<String>) {
    let mut text = String::new();
    let mut figures = Vec::new();
    let mut tables = Vec::new();
    let mut cited_ref_ids = Vec::new();
    let mut depth: u32 = 1; // We're inside <p>
    // Deferred figure/table parsing to avoid borrow conflicts
    let mut deferred_figs: Vec<FigAttrs> = Vec::new();
    let mut deferred_tables: Vec<TableAttrs> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"p" => depth += 1,
                b"xref" => collect_bibr_rids(e, &mut cited_ref_ids),
                b"fig" => {
                    deferred_figs.push(FigAttrs {
                        id: get_attr(e, b"id"),
                        fig_type: get_attr(e, b"fig-type"),
                    });
                }
                b"table-wrap" => {
                    deferred_tables.push(TableAttrs {
                        id: get_attr(e, b"id"),
                    });
                }
                _ => {} // Skip child tags, keep reading for text
            },
            Ok(Event::Text(ref e)) => {
                // Use Cow: borrows when no entities, only allocates when unescaping
                if let Ok(unescaped) = e.unescape() {
                    text.push_str(&unescaped);
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"p" {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }

        // Process deferred figures/tables
        for attrs in deferred_figs.drain(..) {
            if let Some(fig) = parse_figure_inner(reader, attrs) {
                figures.push(fig);
            }
        }
        for attrs in deferred_tables.drain(..) {
            if let Some(table) = parse_table_inner(reader, attrs) {
                tables.push(table);
            }
        }
    }

    (trim_in_place(text), figures, tables, cited_ref_ids)
}

// --- Figure and Table extraction using Reader scan ---

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
    extract_attrs: impl Fn(&quick_xml::events::BytesStart) -> A,
    parse_inner: impl Fn(&mut quick_xml::Reader<&[u8]>, A) -> Option<T>,
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

/// Extract all `<fig>` elements from content using Reader.
/// Scans the entire content string regardless of nesting depth.
fn extract_figures_from_content(content: &str) -> Vec<Figure> {
    scan_elements(
        content,
        b"fig",
        |e| FigAttrs {
            id: get_attr(e, b"id"),
            fig_type: get_attr(e, b"fig-type"),
        },
        parse_figure_inner,
    )
}

/// Extract all `<table-wrap>` elements from content using Reader.
fn extract_tables_from_content(content: &str) -> Vec<Table> {
    scan_elements(
        content,
        b"table-wrap",
        |e| TableAttrs {
            id: get_attr(e, b"id"),
        },
        parse_table_inner,
    )
}

struct FigAttrs {
    id: Option<String>,
    fig_type: Option<String>,
}

struct TableAttrs {
    id: Option<String>,
}

/// Parse figure content after `Event::Start` for `<fig>` has been consumed.
fn parse_figure_inner(reader: &mut quick_xml::Reader<&[u8]>, attrs: FigAttrs) -> Option<Figure> {
    let mut label: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut alt_text: Option<String> = None;
    let mut file_name: Option<String> = None;

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"label" => FigAction::ReadLabel,
                b"caption" => FigAction::ReadCaption,
                b"alt-text" => FigAction::ReadAltText,
                b"graphic" => {
                    let href = get_attr(e, b"xlink:href").or_else(|| get_attr(e, b"href"));
                    FigAction::ReadGraphic(href)
                }
                other => FigAction::Skip(other.to_vec()),
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"fig" => FigAction::Done,
            Ok(Event::Eof) => FigAction::Done,
            Err(_) => FigAction::Done,
            _ => FigAction::Continue,
        };

        match action {
            FigAction::ReadLabel => {
                label = read_text_content(reader, b"label").ok();
            }
            FigAction::ReadCaption => {
                caption = match read_text_content(reader, b"caption") {
                    Ok(text) => Some(text),
                    Err(e) => {
                        warn!(
                            figure_id = ?attrs.id,
                            error = %e,
                            "failed to parse figure caption"
                        );
                        None
                    }
                };
            }
            FigAction::ReadAltText => {
                alt_text = read_text_content(reader, b"alt-text").ok();
            }
            FigAction::ReadGraphic(href) => {
                file_name = href;
                let _ = skip_element(reader, QName(b"graphic"));
            }
            FigAction::Skip(name) => {
                let _ = skip_element(reader, QName(&name));
            }
            FigAction::Done => break,
            FigAction::Continue => {}
        }
    }

    let id = match attrs.id {
        Some(id) => id,
        None => {
            warn!("figure element missing id attribute");
            format!("fig_unknown_{}", line!())
        }
    };
    Some(Figure {
        id,
        label,
        caption,
        alt_text,
        fig_type: attrs.fig_type,
        graphic_href: file_name,
    })
}

enum FigAction {
    Continue,
    Done,
    ReadLabel,
    ReadCaption,
    ReadAltText,
    ReadGraphic(Option<String>),
    Skip(Vec<u8>),
}

/// Parse table-wrap content after `Event::Start` for `<table-wrap>` has been consumed.
fn parse_table_inner(reader: &mut quick_xml::Reader<&[u8]>, attrs: TableAttrs) -> Option<Table> {
    let mut label: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut footnotes = Vec::new();

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"label" => TableAction::ReadLabel,
                b"caption" => TableAction::ReadCaption,
                b"table-wrap-foot" => TableAction::ReadFootnote,
                other => TableAction::Skip(other.to_vec()),
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"table-wrap" => TableAction::Done,
            Ok(Event::Eof) => TableAction::Done,
            Err(_) => TableAction::Done,
            _ => TableAction::Continue,
        };

        match action {
            TableAction::ReadLabel => {
                label = read_text_content(reader, b"label").ok();
            }
            TableAction::ReadCaption => {
                caption = match read_text_content(reader, b"caption") {
                    Ok(text) => Some(text),
                    Err(e) => {
                        warn!(
                            table_id = ?attrs.id,
                            error = %e,
                            "failed to parse table caption"
                        );
                        None
                    }
                };
            }
            TableAction::ReadFootnote => {
                if let Ok(text) = read_text_content(reader, b"table-wrap-foot") {
                    // Already trimmed by `read_text_content`.
                    if !text.is_empty() {
                        footnotes.push(text);
                    }
                }
            }
            TableAction::Skip(name) => {
                let _ = skip_element(reader, QName(&name));
            }
            TableAction::Done => break,
            TableAction::Continue => {}
        }
    }

    let id = match attrs.id {
        Some(id) => id,
        None => {
            warn!("table-wrap element missing id attribute");
            format!("table_unknown_{}", line!())
        }
    };
    Some(Table {
        id,
        label,
        caption,
        head: Vec::new(),
        body: Vec::new(),
        footnotes,
    })
}

enum TableAction {
    Continue,
    Done,
    ReadLabel,
    ReadCaption,
    ReadFootnote,
    Skip(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_abstract_section() {
        let content = r#"
        <abstract>
            <p>This is an abstract paragraph.</p>
        </abstract>
        "#;

        let section = extract_abstract_section(content);
        assert!(section.is_some());

        let section = section.unwrap();
        assert_eq!(section.section_type, Some("abstract".to_string()));
        assert_eq!(section.title, Some("Abstract".to_string()));
        assert!(section.content.contains("This is an abstract paragraph."));
    }

    #[test]
    fn test_extract_figures_from_section() {
        let content = r#"
        <fig id="fig1" fig-type="diagram">
            <label>Figure 1</label>
            <caption>This is a test figure.</caption>
            <alt-text>Alternative text</alt-text>
        </fig>
        "#;

        let figures = extract_figures_from_content(content);
        assert_eq!(figures.len(), 1);
        assert_eq!(figures[0].id, "fig1");
        assert_eq!(figures[0].label, Some("Figure 1".to_string()));
        assert_eq!(
            figures[0].caption.as_deref(),
            Some("This is a test figure.")
        );
        assert_eq!(figures[0].alt_text, Some("Alternative text".to_string()));
        assert_eq!(figures[0].fig_type, Some("diagram".to_string()));
    }

    #[test]
    fn test_extract_tables_from_section() {
        let content = r#"
        <root>
        <table-wrap id="table1">
            <label>Table 1</label>
            <caption>This is a test table.</caption>
            <table>
                <tr><th>Header</th></tr>
                <tr><td>Data</td></tr>
            </table>
        </table-wrap>
        </root>
        "#;

        let tables = extract_tables_from_content(content);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].id, "table1");
        assert_eq!(tables[0].label, Some("Table 1".to_string()));
        assert_eq!(tables[0].caption.as_deref(), Some("This is a test table."));
    }

    #[test]
    fn test_nested_sections_depth() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Methods</title>
            <sec id="sec1.1">
                <title>Study Design</title>
                <p>Inner content.</p>
            </sec>
            <p>Outer content after subsection.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);

        let methods = &sections[0];
        assert_eq!(methods.title, Some("Methods".to_string()));
        assert!(methods.content.contains("Outer content"));
        assert_eq!(methods.subsections.len(), 1);
        assert_eq!(
            methods.subsections[0].title,
            Some("Study Design".to_string())
        );
        assert!(methods.subsections[0].content.contains("Inner content"));
    }

    #[test]
    fn test_body_without_sections() {
        let content = r#"
        <body>
            <p>Just a paragraph.</p>
            <p>Another paragraph.</p>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section_type, Some("body".to_string()));
        assert!(sections[0].content.contains("Just a paragraph."));
        assert!(sections[0].content.contains("Another paragraph."));
    }

    #[test]
    fn test_inline_figure_in_paragraph() {
        let content = r#"
        <body>
            <p>Some text <fig id="fig1"><label>Figure 1</label><caption>Test caption</caption><graphic xlink:href="fig1.jpg"/></fig> more text.</p>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        // Figures should be found even when inline in <p>
        assert!(
            !sections[0].figures.is_empty(),
            "Expected figures to be extracted from inline position"
        );
        assert_eq!(sections[0].figures[0].id, "fig1");
    }

    // --- Tests for JATS %para-level; elements that were previously skipped ---

    #[test]
    fn test_list_text_extraction_in_section() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Methods</title>
            <p>Before list.</p>
            <list list-type="bullet">
                <list-item><p>First item</p></list-item>
                <list-item><p>Second item</p></list-item>
            </list>
            <p>After list.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("Before list."));
        assert!(section.content.contains("First item"));
        assert!(section.content.contains("Second item"));
        assert!(section.content.contains("After list."));
    }

    #[test]
    fn test_def_list_text_extraction_in_section() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Abbreviations</title>
            <def-list>
                <def-item>
                    <term>DNA</term>
                    <def><p>Deoxyribonucleic acid</p></def>
                </def-item>
                <def-item>
                    <term>RNA</term>
                    <def><p>Ribonucleic acid</p></def>
                </def-item>
            </def-list>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("DNA"));
        assert!(section.content.contains("Deoxyribonucleic acid"));
        assert!(section.content.contains("RNA"));
        assert!(section.content.contains("Ribonucleic acid"));
    }

    #[test]
    fn test_disp_formula_text_extraction() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Model</title>
            <p>The equation is:</p>
            <disp-formula id="eq1">
                <label>(1)</label>
                <tex-math>E = mc^2</tex-math>
            </disp-formula>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("The equation is:"));
        assert!(
            section.content.contains("E = mc^2"),
            "Formula text should be extracted, got: {}",
            section.content
        );
    }

    #[test]
    fn test_boxed_text_extraction() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Results</title>
            <boxed-text>
                <title>Key Finding</title>
                <p>Important result goes here.</p>
            </boxed-text>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(
            section.content.contains("Important result goes here."),
            "Boxed text content should be extracted, got: {}",
            section.content
        );
    }

    #[test]
    fn test_code_extraction() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Implementation</title>
            <code language="python">print("hello world")</code>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(
            section.content.contains("print(\"hello world\")"),
            "Code content should be extracted, got: {}",
            section.content
        );
    }

    #[test]
    fn test_disp_quote_extraction() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Discussion</title>
            <disp-quote>
                <p>To be or not to be, that is the question.</p>
            </disp-quote>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("To be or not to be"));
    }

    #[test]
    fn test_preformat_extraction() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Data</title>
            <preformat>
Column1  Column2  Column3
value1   value2   value3
            </preformat>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("Column1"));
        assert!(section.content.contains("value1"));
    }

    #[test]
    fn test_mixed_elements_in_section() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Mixed Content</title>
            <p>Paragraph text.</p>
            <list list-type="order">
                <list-item><p>Ordered item one</p></list-item>
                <list-item><p>Ordered item two</p></list-item>
            </list>
            <fig id="fig1">
                <label>Figure 1</label>
                <caption>A test figure</caption>
            </fig>
            <disp-formula id="eq1">
                <label>(2)</label>
                <tex-math>a^2 + b^2 = c^2</tex-math>
            </disp-formula>
            <p>Final paragraph.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("Paragraph text."));
        assert!(section.content.contains("Ordered item one"));
        assert!(section.content.contains("a^2 + b^2 = c^2"));
        assert!(section.content.contains("Final paragraph."));
        assert_eq!(section.figures.len(), 1);
        assert_eq!(section.figures[0].id, "fig1");
    }

    #[test]
    fn test_body_without_sec_with_list() {
        let content = r#"
        <body>
            <p>Introduction paragraph.</p>
            <list list-type="bullet">
                <list-item><p>Bullet point one</p></list-item>
                <list-item><p>Bullet point two</p></list-item>
            </list>
            <p>Conclusion paragraph.</p>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section_type, Some("body".to_string()));
        assert!(sections[0].content.contains("Introduction paragraph."));
        assert!(sections[0].content.contains("Bullet point one"));
        assert!(sections[0].content.contains("Bullet point two"));
        assert!(sections[0].content.contains("Conclusion paragraph."));
    }

    #[test]
    fn test_media_in_section() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Supplementary</title>
            <media mimetype="video" xlink:href="movie1.mp4">
                <caption><p>Supplementary Movie 1</p></caption>
            </media>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(
            section.content.contains("Supplementary Movie 1"),
            "Media caption should be extracted, got: {}",
            section.content
        );
    }

    #[test]
    fn test_fn_group_in_section() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Notes</title>
            <p>Main text.</p>
            <fn-group>
                <fn id="fn1"><p>Author contribution note.</p></fn>
                <fn id="fn2"><p>Funding disclosure.</p></fn>
            </fn-group>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("Main text."));
        assert!(
            section.content.contains("Author contribution note."),
            "fn-group content should be extracted, got: {}",
            section.content
        );
    }

    #[test]
    fn test_supplementary_material_extraction() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Supporting Information</title>
            <supplementary-material id="sm1">
                <caption><p>Supplementary dataset S1.</p></caption>
            </supplementary-material>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert!(
            sections[0].content.contains("Supplementary dataset S1."),
            "supplementary-material content should be extracted, got: {}",
            sections[0].content
        );
    }

    #[test]
    fn test_statement_and_verse_group_extraction() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Misc</title>
            <statement id="st1"><p>Theorem statement text.</p></statement>
            <verse-group><verse-line>A line of verse.</verse-line></verse-group>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(
            section.content.contains("Theorem statement text."),
            "statement content should be extracted, got: {}",
            section.content
        );
        assert!(
            section.content.contains("A line of verse."),
            "verse-group content should be extracted, got: {}",
            section.content
        );
    }

    // --- Tests for in-text citation linkage (<xref ref-type="bibr">) ---

    #[test]
    fn test_cited_reference_ids_captured_per_section() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Introduction</title>
            <p>Prior work established this <xref ref-type="bibr" rid="B1">1</xref>,
               and later studies confirmed it <xref ref-type="bibr" rid="B2">2</xref>.</p>
            <p>A follow-up <xref ref-type="bibr" rid="B3">3</xref> extended the results.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(
            section.cited_reference_ids,
            vec!["B1".to_string(), "B2".to_string(), "B3".to_string()]
        );
        // The visible citation markers stay in the content unchanged.
        assert!(section.content.contains('1'));
        assert!(section.content.contains("Prior work established this"));
    }

    #[test]
    fn test_grouped_citation_rids_split() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Methods</title>
            <p>Several groups reported this <xref ref-type="bibr" rid="B1 B2 B3">1-3</xref>.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert_eq!(
            sections[0].cited_reference_ids,
            vec!["B1".to_string(), "B2".to_string(), "B3".to_string()]
        );
    }

    #[test]
    fn test_non_bibr_xref_not_captured_as_citation() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Results</title>
            <p>As shown in <xref ref-type="fig" rid="fig1">Figure 1</xref>, the effect
               is significant <xref ref-type="bibr" rid="B5">5</xref>.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        // Only the bibr xref contributes; the figure xref is ignored.
        assert_eq!(sections[0].cited_reference_ids, vec!["B5".to_string()]);
    }

    #[test]
    fn test_cited_reference_ids_empty_without_citations() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Discussion</title>
            <p>No citations in this paragraph.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert!(sections[0].cited_reference_ids.is_empty());
    }

    #[test]
    fn test_cited_reference_ids_in_body_without_sections() {
        let content = r#"
        <body>
            <p>Early results <xref ref-type="bibr" rid="B1">1</xref> were promising.</p>
            <p>Later work <xref ref-type="bibr" rid="B2">2</xref> disagreed.</p>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section_type, Some("body".to_string()));
        assert_eq!(
            sections[0].cited_reference_ids,
            vec!["B1".to_string(), "B2".to_string()]
        );
    }

    #[test]
    fn test_cited_reference_ids_scoped_to_own_section() {
        // Each <sec> keeps only the citations from its own paragraphs; the
        // recursive accessor on the domain model aggregates subsections.
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Outer</title>
            <p>Outer cite <xref ref-type="bibr" rid="B1">1</xref>.</p>
            <sec id="sec1.1">
                <title>Inner</title>
                <p>Inner cite <xref ref-type="bibr" rid="B2">2</xref>.</p>
            </sec>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let outer = &sections[0];
        assert_eq!(outer.cited_reference_ids, vec!["B1".to_string()]);
        assert_eq!(outer.subsections.len(), 1);
        assert_eq!(
            outer.subsections[0].cited_reference_ids,
            vec!["B2".to_string()]
        );
        // Recursive accessor collects both, in document order.
        let all: Vec<&String> = outer.all_cited_reference_ids();
        assert_eq!(all, vec![&"B1".to_string(), &"B2".to_string()]);
    }

    #[test]
    fn test_unrecognized_tag_is_skipped_without_dropping_siblings() {
        // A tag that is neither structural nor block-level must be skipped
        // cleanly, leaving surrounding paragraphs intact.
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Intro</title>
            <p>Before unknown.</p>
            <unknown-tag><p>ignored content</p></unknown-tag>
            <p>After unknown.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert!(section.content.contains("Before unknown."));
        assert!(section.content.contains("After unknown."));
        assert!(
            !section.content.contains("ignored content"),
            "content of skipped tag should not be extracted, got: {}",
            section.content
        );
    }
}
