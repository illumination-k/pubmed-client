use super::reader_utils::{get_attr, make_reader, read_text_content, skip_element};
use super::xml_utils;
use crate::pmc::models::{ArticleSection, Figure, Table};
use quick_xml::events::Event;
use quick_xml::name::QName;

/// Extract all sections from PMC XML content
pub fn extract_sections_enhanced(content: &str) -> Vec<ArticleSection> {
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
                let mut figures_section =
                    ArticleSection::new("figures".to_string(), "Figures section".to_string());
                figures_section.figures = float_figures;
                sections.push(figures_section);
            }
        }
    }

    sections
}

/// Extract abstract section using Reader for text, Reader scan for figures/tables
fn extract_abstract_section(content: &str) -> Option<ArticleSection> {
    let abstract_start = content.find("<abstract")?;
    let abstract_end_offset = content[abstract_start..].find("</abstract>")?;
    let abstract_xml =
        &content[abstract_start..abstract_start + abstract_end_offset + "</abstract>".len()];

    // Extract text content using Reader
    let mut reader = make_reader(abstract_xml);
    let mut buf = Vec::new();
    let mut text_parts = Vec::new();
    let mut in_abstract = false;

    loop {
        let action = match reader.read_event_into(&mut buf) {
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
        buf.clear();

        match action {
            SectionAction::EnterAbstract => in_abstract = true,
            SectionAction::ReadParagraph => {
                if let Ok(text) = read_text_content(&mut reader, b"p", &mut buf) {
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        text_parts.push(trimmed);
                    }
                }
            }
            SectionAction::SkipTitle => {
                let _ = read_text_content(&mut reader, b"title", &mut buf);
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
        let mut section = ArticleSection::with_title(
            "abstract".to_string(),
            "Abstract".to_string(),
            clean_content,
        );
        section.figures = figures;
        section.tables = tables;
        Some(section)
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
    SkipTitle,
    SkipTag(Vec<u8>),
}

/// Extract body sections using Reader with depth-aware `<sec>` parsing
fn extract_body_sections(content: &str) -> Vec<ArticleSection> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    let mut sections = Vec::new();
    let mut has_sec_tags = false;
    let mut body_paragraphs = Vec::new();
    let mut body_figures = Vec::new();
    let mut body_tables = Vec::new();

    loop {
        let action = match reader.read_event_into(&mut buf) {
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
                _ => SectionAction::Continue,
            },
            Ok(Event::Eof) => SectionAction::Break,
            Err(_) => SectionAction::Break,
            _ => SectionAction::Continue,
        };
        buf.clear();

        match action {
            SectionAction::ReadSection(id) => {
                has_sec_tags = true;
                if let Some(section) = parse_section_from_body(&mut reader, id, &mut buf) {
                    sections.push(section);
                }
            }
            SectionAction::ReadBodyParagraph => {
                let (text, inline_figs, inline_tables) =
                    read_paragraph_with_inline(&mut reader, &mut buf);
                if !text.is_empty() {
                    body_paragraphs.push(text);
                }
                body_figures.extend(inline_figs);
                body_tables.extend(inline_tables);
            }
            SectionAction::ReadFigure(attrs) => {
                if let Some(fig) = parse_figure_inner(&mut reader, attrs, &mut buf) {
                    body_figures.push(fig);
                }
            }
            SectionAction::ReadTable(attrs) => {
                if let Some(table) = parse_table_inner(&mut reader, attrs, &mut buf) {
                    body_tables.push(table);
                }
            }
            SectionAction::Break => break,
            _ => {}
        }
    }

    // If no sections found, create a body section from paragraphs
    if sections.is_empty() && !body_paragraphs.is_empty() {
        let text = body_paragraphs.join("\n");
        let mut section = ArticleSection::new("body".to_string(), text);
        section.figures = body_figures;
        section.tables = body_tables;
        sections.push(section);
    }

    sections
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
    buf: &mut Vec<u8>,
) -> Option<ArticleSection> {
    let mut title: Option<String> = None;
    let mut content_parts: Vec<String> = Vec::new();
    let mut subsections = Vec::new();
    let mut figures = Vec::new();
    let mut tables = Vec::new();

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
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
                other => SectionAction::SkipTag(other.to_vec()),
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"sec" => SectionAction::Break,
            Ok(Event::Eof) => SectionAction::Break,
            Err(_) => SectionAction::Break,
            _ => SectionAction::Continue,
        };
        buf.clear();

        match action {
            SectionAction::SkipTitle => {
                if let Ok(t) = read_text_content(reader, b"title", buf) {
                    let t = t.trim().to_string();
                    if !t.is_empty() {
                        title = Some(t);
                    }
                }
            }
            SectionAction::ReadParagraph => {
                let (text, inline_figs, inline_tables) = read_paragraph_with_inline(reader, buf);
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    content_parts.push(trimmed);
                }
                figures.extend(inline_figs);
                tables.extend(inline_tables);
            }
            SectionAction::ReadSection(sub_id) => {
                // Recursive: properly handles nested sections
                if let Some(sub) = parse_section_from_body(reader, sub_id, buf) {
                    subsections.push(sub);
                }
            }
            SectionAction::ReadFigure(attrs) => {
                if let Some(fig) = parse_figure_inner(reader, attrs, buf) {
                    figures.push(fig);
                }
            }
            SectionAction::ReadTable(attrs) => {
                if let Some(table) = parse_table_inner(reader, attrs, buf) {
                    tables.push(table);
                }
            }
            SectionAction::SkipTag(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            SectionAction::Break => break,
            _ => {}
        }
    }

    let section_content = content_parts.join("\n");

    if !section_content.trim().is_empty()
        || !subsections.is_empty()
        || !figures.is_empty()
        || !tables.is_empty()
    {
        let mut section = match title {
            Some(t) => ArticleSection::with_title(
                "section".to_string(),
                t,
                section_content.trim().to_string(),
            ),
            None => ArticleSection::new("section".to_string(), section_content.trim().to_string()),
        };

        section.id = id;
        section.figures = figures;
        section.tables = tables;
        section.subsections = subsections;

        Some(section)
    } else {
        None
    }
}

/// Read a `<p>` element, collecting text while extracting inline figures and tables.
///
/// Uses Cow<str> from unescape() to avoid allocations when text has no XML entities.
/// Detects `<fig>` and `<table-wrap>` inside `<p>` and parses them as structured data.
fn read_paragraph_with_inline(
    reader: &mut quick_xml::Reader<&[u8]>,
    buf: &mut Vec<u8>,
) -> (String, Vec<Figure>, Vec<Table>) {
    let mut text = String::new();
    let mut figures = Vec::new();
    let mut tables = Vec::new();
    let mut depth: u32 = 1; // We're inside <p>
    // Deferred figure/table parsing to avoid borrow conflicts
    let mut deferred_figs: Vec<FigAttrs> = Vec::new();
    let mut deferred_tables: Vec<TableAttrs> = Vec::new();

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"p" => depth += 1,
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
                        buf.clear();
                        break;
                    }
                }
            }
            Ok(Event::Eof) => {
                buf.clear();
                break;
            }
            Err(_) => {
                buf.clear();
                break;
            }
            _ => {}
        }
        buf.clear();

        // Process deferred figures/tables (buf is cleared, safe to use)
        for attrs in deferred_figs.drain(..) {
            if let Some(fig) = parse_figure_inner(reader, attrs, buf) {
                figures.push(fig);
            }
        }
        for attrs in deferred_tables.drain(..) {
            if let Some(table) = parse_table_inner(reader, attrs, buf) {
                tables.push(table);
            }
        }
    }

    (text.trim().to_string(), figures, tables)
}

// --- Figure and Table extraction using Reader scan ---

/// Extract all `<fig>` elements from content using Reader.
/// Scans the entire content string regardless of nesting depth.
fn extract_figures_from_content(content: &str) -> Vec<Figure> {
    let mut figures = Vec::new();
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let attrs = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"fig" => Some(FigAttrs {
                id: get_attr(e, b"id"),
                fig_type: get_attr(e, b"fig-type"),
            }),
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        buf.clear();

        if let Some(attrs) = attrs
            && let Some(fig) = parse_figure_inner(&mut reader, attrs, &mut buf)
        {
            figures.push(fig);
        }
    }

    figures
}

/// Extract all `<table-wrap>` elements from content using Reader.
fn extract_tables_from_content(content: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let attrs = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"table-wrap" => Some(TableAttrs {
                id: get_attr(e, b"id"),
            }),
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        buf.clear();

        if let Some(attrs) = attrs
            && let Some(table) = parse_table_inner(&mut reader, attrs, &mut buf)
        {
            tables.push(table);
        }
    }

    tables
}

struct FigAttrs {
    id: Option<String>,
    fig_type: Option<String>,
}

struct TableAttrs {
    id: Option<String>,
}

/// Parse figure content after `Event::Start` for `<fig>` has been consumed.
fn parse_figure_inner(
    reader: &mut quick_xml::Reader<&[u8]>,
    attrs: FigAttrs,
    buf: &mut Vec<u8>,
) -> Option<Figure> {
    let mut label: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut alt_text: Option<String> = None;
    let mut file_name: Option<String> = None;

    loop {
        let action = match reader.read_event_into(buf) {
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
        buf.clear();

        match action {
            FigAction::ReadLabel => {
                label = read_text_content(reader, b"label", buf).ok();
            }
            FigAction::ReadCaption => {
                caption = Some(
                    read_text_content(reader, b"caption", buf)
                        .unwrap_or_else(|_| "No caption available".to_string()),
                );
            }
            FigAction::ReadAltText => {
                alt_text = read_text_content(reader, b"alt-text", buf).ok();
            }
            FigAction::ReadGraphic(href) => {
                file_name = href;
                let _ = skip_element(reader, QName(b"graphic"), buf);
            }
            FigAction::Skip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            FigAction::Done => break,
            FigAction::Continue => {}
        }
    }

    let fig_id = attrs.id.unwrap_or_else(|| "fig_unknown".to_string());
    let fig_caption = caption.unwrap_or_else(|| "No caption available".to_string());

    let mut figure = Figure::new(fig_id, fig_caption);
    figure.label = label;
    figure.alt_text = alt_text;
    figure.fig_type = attrs.fig_type;
    figure.file_name = file_name;

    Some(figure)
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
fn parse_table_inner(
    reader: &mut quick_xml::Reader<&[u8]>,
    attrs: TableAttrs,
    buf: &mut Vec<u8>,
) -> Option<Table> {
    let mut label: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut footnotes = Vec::new();

    loop {
        let action = match reader.read_event_into(buf) {
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
        buf.clear();

        match action {
            TableAction::ReadLabel => {
                label = read_text_content(reader, b"label", buf).ok();
            }
            TableAction::ReadCaption => {
                caption = Some(
                    read_text_content(reader, b"caption", buf)
                        .unwrap_or_else(|_| "No caption available".to_string()),
                );
            }
            TableAction::ReadFootnote => {
                if let Ok(text) = read_text_content(reader, b"table-wrap-foot", buf) {
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        footnotes.push(trimmed);
                    }
                }
            }
            TableAction::Skip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TableAction::Done => break,
            TableAction::Continue => {}
        }
    }

    let table_id = attrs.id.unwrap_or_else(|| "table_unknown".to_string());
    let table_caption = caption.unwrap_or_else(|| "No caption available".to_string());

    let mut table = Table::new(table_id, table_caption);
    table.label = label;
    table.footnotes = footnotes;

    Some(table)
}

enum TableAction {
    Continue,
    Done,
    ReadLabel,
    ReadCaption,
    ReadFootnote,
    Skip(Vec<u8>),
}

/// Extract section title from section content
pub fn extract_section_title(content: &str) -> Option<String> {
    xml_utils::extract_text_between(content, "<title>", "</title>")
}

/// Extract section ID from section content
pub fn extract_section_id(content: &str) -> Option<String> {
    xml_utils::extract_attribute_value(content, "id")
}

/// Extract all paragraph content from a section
pub fn extract_paragraph_content(content: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let is_p = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"p" => true,
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => false,
        };
        buf.clear();

        if is_p && let Ok(text) = read_text_content(&mut reader, b"p", &mut buf) {
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                paragraphs.push(trimmed);
            }
        }
    }

    paragraphs
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
        assert_eq!(section.section_type, "abstract");
        assert_eq!(section.title, Some("Abstract".to_string()));
        assert!(section.content.contains("This is an abstract paragraph."));
    }

    #[test]
    fn test_extract_section_title() {
        let content = r#"<sec id="sec1"><title>Introduction</title><p>Content</p></sec>"#;
        let title = extract_section_title(content);
        assert_eq!(title, Some("Introduction".to_string()));
    }

    #[test]
    fn test_extract_section_id() {
        let content = r#"<sec id="sec1"><title>Introduction</title><p>Content</p></sec>"#;
        let id = extract_section_id(content);
        assert_eq!(id, Some("sec1".to_string()));
    }

    #[test]
    fn test_extract_paragraph_content() {
        let content = r#"
        <p>First paragraph.</p>
        <p>Second paragraph with <em>emphasis</em>.</p>
        "#;

        let paragraphs = extract_paragraph_content(content);
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0], "First paragraph.");
        assert_eq!(paragraphs[1], "Second paragraph with emphasis.");
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
        assert_eq!(figures[0].caption, "This is a test figure.");
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
        assert_eq!(tables[0].caption, "This is a test table.");
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
        assert_eq!(sections[0].section_type, "body");
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
}
