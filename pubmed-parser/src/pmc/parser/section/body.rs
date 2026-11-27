//! JATS `<body>` and `<sec>` parsing.

use crate::pmc::domain::Section;
use crate::pmc::parser::reader_utils::{get_attr, make_reader};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::mem;

use super::figure::FigAttrs;
use super::paragraph::is_block_level;
use super::table::TableAttrs;
use super::{SectionAction, SectionParts};

/// Extract body sections using Reader with depth-aware `<sec>` parsing.
///
/// `<sec>` children become sections. Everything else directly under `<body>`
/// — lead paragraphs before the first `<sec>` (editorials, commentaries),
/// paragraphs between sections, or a body with no `<sec>` at all — is
/// collected into `loose` and flushed, in document order, as an untitled
/// synthetic "body" section whenever a `<sec>` starts or the body ends.
pub(super) fn extract_body_sections(content: &str) -> Vec<Section> {
    let mut reader = make_reader(content);
    let mut sections = Vec::new();
    let mut loose = SectionParts::default();

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => classify_body_child(e),
            Ok(Event::Eof) => SectionAction::Break,
            Err(_) => SectionAction::Break,
            _ => SectionAction::Continue,
        };

        if let SectionAction::ReadSection(id) = action {
            flush_loose(&mut loose, &mut sections);
            if let Some(section) = parse_section_from_body(&mut reader, id) {
                sections.push(section);
            }
            continue;
        }

        if loose.apply(action, &mut reader) {
            break;
        }
    }

    flush_loose(&mut loose, &mut sections);
    sections
}

/// Move the loose `<body>` children collected so far into `sections`.
///
/// Text becomes a synthetic "body" section. Figures and tables without any
/// surrounding text are floating displays placed between sections; they are
/// attached to the preceding section, or kept pending until one exists.
fn flush_loose(loose: &mut SectionParts, sections: &mut Vec<Section>) {
    if loose.content_parts.is_empty() {
        if let Some(previous) = sections.last_mut() {
            let parts = mem::take(loose);
            previous.figures.extend(parts.figures);
            previous.tables.extend(parts.tables);
            previous
                .cited_reference_ids
                .extend(parts.cited_reference_ids);
        }
        return;
    }

    if let Some(section) = mem::take(loose).into_body_section() {
        sections.push(section);
    }
}

/// Classify a start element encountered directly under `<body>`.
fn classify_body_child(e: &BytesStart) -> SectionAction {
    match e.name().as_ref() {
        b"sec" => SectionAction::ReadSection(get_attr(e, b"id")),
        b"p" => SectionAction::ReadParagraph,
        b"fig" => SectionAction::ReadFigure(FigAttrs::from_start(e)),
        b"table-wrap" => SectionAction::ReadTable(TableAttrs::from_start(e)),
        // Block-level elements per JATS %para-level; — extract their text
        other if is_block_level(other) => SectionAction::ReadTextElement(other.to_vec()),
        _ => SectionAction::Continue,
    }
}

/// Classify a start element encountered inside a `<sec>` into the action needed
/// to consume it. Keeps the tag dispatch out of the main parse loop.
fn classify_section_child(e: &BytesStart) -> SectionAction {
    match e.name().as_ref() {
        b"title" => SectionAction::ReadTitle,
        b"p" => SectionAction::ReadParagraph,
        b"sec" => SectionAction::ReadSection(get_attr(e, b"id")),
        b"fig" => SectionAction::ReadFigure(FigAttrs::from_start(e)),
        b"table-wrap" => SectionAction::ReadTable(TableAttrs::from_start(e)),
        // Block-level elements per JATS %para-level; — extract text instead of skipping
        other if is_block_level(other) => SectionAction::ReadTextElement(other.to_vec()),
        other => SectionAction::SkipTag(other.to_vec()),
    }
}

/// Parse a single `<sec>` element using Reader for structure.
/// The reader has just consumed `Event::Start` for `<sec>`.
///
/// Uses a single Reader pass for text, figures, tables, and subsections.
/// Figures and tables are detected both as direct children of `<sec>` and
/// inline within `<p>` tags via `read_paragraph_with_inline`.
pub(super) fn parse_section_from_body(
    reader: &mut Reader<&[u8]>,
    id: Option<String>,
) -> Option<Section> {
    parse_sectioned_element(reader, id, b"sec")
}

/// Parse any element with the `<sec>` content model (`<sec>`, `<app>`, ...),
/// ending at the close tag `end_tag`. The reader has just consumed its start tag.
pub(super) fn parse_sectioned_element(
    reader: &mut Reader<&[u8]>,
    id: Option<String>,
    end_tag: &[u8],
) -> Option<Section> {
    let mut parts = SectionParts::default();

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => classify_section_child(e),
            Ok(Event::End(ref e)) if e.name().as_ref() == end_tag => SectionAction::Break,
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

#[cfg(test)]
mod tests {
    use super::super::extract_sections_enhanced;

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
    fn test_lead_paragraphs_before_first_section_are_kept() {
        let content = r#"
        <body>
            <p>Lead paragraph of an editorial.</p>
            <fig id="f1"><label>Figure 1</label><caption><p>Lead figure.</p></caption></fig>
            <sec id="s1"><title>Background</title><p>Section text.</p></sec>
            <table-wrap id="t1"><label>Table 1</label><caption><p>Floating table.</p></caption></table-wrap>
            <sec id="s2"><title>Outlook</title><p>More text.</p></sec>
            <p>Closing remark.</p>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        let titles: Vec<Option<&str>> = sections.iter().map(|s| s.title.as_deref()).collect();
        assert_eq!(titles, [None, Some("Background"), Some("Outlook"), None]);

        assert_eq!(sections[0].section_type.as_deref(), Some("body"));
        assert_eq!(sections[0].content, "Lead paragraph of an editorial.");
        assert_eq!(sections[0].figures.len(), 1);
        // A display element with no surrounding text joins the preceding section.
        assert_eq!(sections[1].tables.len(), 1);
        assert_eq!(sections[3].content, "Closing remark.");
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
